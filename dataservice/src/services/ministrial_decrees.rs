use std::{
    collections::HashMap,
    sync::{Arc, LazyLock},
};

use combx::{
    Decree, FromScrapedType, Index, LegislativePeriod, PostgresEnhancer, with_data::gps::gps,
};
use dotenvy_macro::dotenv;
use somes_scraper::{AustriaScraper, Thing, decree::RisScraper};
use sqlx::PgPool;
use tokio::sync::mpsc::Sender;

use crate::{
    AppState, CompareHash, Shutdown, ai_task::AiTask, enforce_ai_run,
    ministrial_proposals::get_active_gov_officals, should_update_by_hash, today,
};

mod ai_tasks;
mod notify;
pub use notify::*;

const DECREE_STILL_SEND_DAYS: LazyLock<i64> = LazyLock::new(|| {
    dotenv!("LEGIS_INIT_STILL_SEND_DAYS")
        .parse::<i64>()
        .unwrap_or(10)
});

pub async fn save_ministrial_decrees(
    pages: u32,
    app_state: AppState,
    ai_task_tx: Sender<AiTask>,
    only_latest: bool,
) {
    let scraper = AustriaScraper::default();

    let mut all_gps = gps(&app_state.pg_pool).await.unwrap();
    all_gps.insert(
        0,
        LegislativePeriod {
            gp: "".into(),
            start_date: today().date_naive(),
        },
    );

    let all_gps: Arc<[LegislativePeriod]> = Arc::from(all_gps);

    let (new_decree_tx, new_decree_rx) = if only_latest {
        let (tx, rx) = tokio::sync::mpsc::channel::<(i32, Decree)>(100);
        (Some(tx), Some(rx))
    } else {
        (None, None)
    };

    for page in 1..=pages {
        log::info!("Currently on page {page}..");
        let Ok(decrees) = scraper.fetch_and_parse_ministrial_decrees(page).await else {
            log::error!("Cannot scrape ministrial decrees on page {page}");
            continue;
        };

        for decree in decrees {
            if let Err(e) = insert_ministrial_decree(
                decree,
                &app_state,
                all_gps.clone(),
                &ai_task_tx,
                new_decree_tx.clone(),
            )
            .await
            {
                log::error!("Cannot insert decree: {e:?}");
            }
        }

        if only_latest && page >= 2 {
            break;
        }
    }

    drop(new_decree_tx);

    // Send email notifications for new decrees
    if let Some(mut new_decree_rx) = new_decree_rx {
        let pool = app_state.pg_pool.clone();
        tokio::task::spawn(async move {
            let mut all_decrees = vec![];
            while let Some(decree) = new_decree_rx.recv().await {
                all_decrees.push(decree);
            }

            if all_decrees.is_empty() {
                return;
            }

            let simple_gov_officials = get_active_gov_officals(&pool).await;
            let simple_gov_officials = simple_gov_officials
                .unwrap_or_default()
                .into_iter()
                .map(|gov| (gov.id, gov))
                .collect::<HashMap<_, _>>();

            let notify_user_decrees = find_notifiers_decrees(&pool, &all_decrees).await;

            notify_users(&notify_user_decrees, &simple_gov_officials);

            let notify_push_decrees = find_push_notifiers_decrees(&pool, &all_decrees).await;

            send_push_for_decrees(&pool, &notify_push_decrees).await;
        });
    }
}

impl CompareHash for Decree {
    async fn stored_hash(&self, pg: &PgPool) -> Option<u64> {
        sqlx::query_scalar!(
            "select hash from ministrial_decrees where ris_id = $1",
            self.ris_id
        )
        .fetch_one(pg)
        .await
        .ok()
        .flatten()
        .map(|hash| hash as u64)
    }
}

pub async fn insert_ministrial_decree(
    decree: Thing,
    app_state: &AppState,
    all_gps: Arc<[LegislativePeriod]>,
    ai_task_tx: &Sender<AiTask>,
    new_decree_tx: Option<Sender<(i32, Decree)>>,
) -> crate::Result<()> {
    let pg_pool = &app_state.pg_pool;
    let mut decree =
        combx::Decree::from_scraped_type(PostgresEnhancer(pg_pool.clone()), (decree, all_gps))
            .await;

    let (should_update, hash) = should_update_by_hash(&decree, pg_pool).await;
    if !enforce_ai_run() && !should_update {
        log::info!("No need to update decree: {}", decree.ris_id);
        return Ok(());
    }

    if let Some(gov_official_id) = decree.gov_official_id {
        let decree_id = sqlx::query!(
            "
            INSERT INTO ministrial_decrees (
                gov_official_id,
                ris_id,
                ministrial_issuer,
                title,
                short_title,
                publication_date,
                part,
                document_url,
                eli,
                gp,
                hash
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT (ris_id) DO UPDATE
            SET
                gov_official_id = EXCLUDED.gov_official_id,
                title = EXCLUDED.title,
                short_title = EXCLUDED.short_title,
                publication_date = EXCLUDED.publication_date,
                part = EXCLUDED.part,
                document_url = EXCLUDED.document_url,
                eli = EXCLUDED.eli,
                gp = EXCLUDED.gp,
                updated_at = NOW(),
                hash = EXCLUDED.hash
            RETURNING id;
            ",
            gov_official_id,
            decree.ris_id,
            decree.ministrial_issuer,
            decree.title,
            decree.short_title,
            decree.publication_date,
            decree.part,
            decree.document_url,
            decree.eli,
            decree.gp,
            hash as i64,
        )
        .fetch_one(pg_pool)
        .await?;

        decree.id = Some(decree_id.id);
        let decree_id = decree_id.id;

        // Check and send email notification for new decree
        #[cfg(not(feature = "ai"))]
        if let Some(new_decree_tx) = new_decree_tx {
            check_and_send_decree_email(pg_pool, &decree, decree_id, &new_decree_tx).await;
        }

        if should_update
            && let Some(client) = &app_state.redis_client
            && let Err(e) =
                crate::notify_update_streamed(Index::Decrees, &decree, &mut client.clone()).await
        {
            log::error!("Could not notify for decree update!: {e:?}");
        }

        #[cfg(feature = "ai")]
        {
            use tokio::time::Instant;

            use crate::{LATEST_GP, ai_task::Priority};

            let (priority, new_decree_tx) =
                if decree.publication_date >= (today() - chrono::Duration::days(90)).date_naive() {
                    (Priority::Highest, new_decree_tx.clone())
                } else if decree.gp.as_deref() == Some(LATEST_GP) {
                    (Priority::High, None)
                } else {
                    (Priority::Lowest, None)
                };

            let inner_decree = decree.clone();
            let pool = pg_pool.clone();
            let documents = decree.documents.clone();
            let summarization_task = AiTask {
                input: None,
                priority,
                score: priority.weight(),
                pool: pool.clone(),
                task_id: crate::hash!(app_state.parliament),
                db_entry_id: decree_id,
                rerunnable: true,
                created_at: Instant::now(),
                task: Box::new(move |task_args| {
                    let inner_decree = inner_decree.clone();
                    let pool = pool.clone();
                    let documents = documents.clone();
                    let new_decree_tx = new_decree_tx.clone();
                    Box::pin(async move {
                        use crate::ministrial_decrees::ai_tasks::summarize_decrees_documents;

                        let ai_summary = summarize_decrees_documents(task_args, &documents).await?;

                        // not returning on None to still send decree
                        let mut inner_decree = inner_decree;
                        inner_decree.ai_summary = ai_summary;
                        if let Some(decree_mail_tx) = new_decree_tx {
                            check_and_send_decree_email(
                                &pool,
                                &inner_decree,
                                decree_id,
                                &decree_mail_tx,
                            )
                            .await;
                        }

                        Ok(())
                        // TODO: update meilisearch index on finish
                    })
                }),
            };

            log::info!(
                "Sending AI task for ministerial decree summarization: {} ({:?}) with priority {priority:?}",
                decree_id,
                decree.gp,
            );

            if let Err(e) = ai_task_tx.send(summarization_task).await {
                log::error!("Could not send AI task for ministerial decree summarization: {e:?}");
            }
        }

        if enforce_ai_run() && !should_update {
            log::info!("No need to update further!");
            return Ok(());
        }

        for document in &decree.documents {
            sqlx::query!(
                "
                INSERT INTO ministrial_decrees_documents (
                    ministrial_decree_id,
                    title,
                    document_type,
                    document_url
                ) VALUES ($1, $2, $3, $4)
                ON CONFLICT (ministrial_decree_id, document_url) DO UPDATE
                SET
                    title = EXCLUDED.title,
                    document_type = EXCLUDED.document_type,
                    document_url = EXCLUDED.document_url;
                ",
                decree_id,
                document.title,
                document.document_type,
                document.document_url
            )
            .execute(pg_pool)
            .await?;
        }
    }

    Ok(())
}

async fn check_and_send_decree_email(
    pool: &sqlx::Pool<sqlx::Postgres>,
    decree: &Decree,
    decree_id: i32,
    new_decree_tx: &Sender<(i32, Decree)>,
) {
    if let Ok(info) = sqlx::query!(
        "select * from decree_email_info where already_sent_id = $1",
        decree_id
    )
    .fetch_optional(pool)
    .await
    {
        let diff = today().to_utc().date_naive() - decree.publication_date;

        if diff.num_days() <= *DECREE_STILL_SEND_DAYS && info.is_none() {
            log::info!("Sending fitting decree: {decree_id}");
            if let Err(e) = new_decree_tx.send((decree_id, decree.clone())).await {
                log::error!("could not send decree to mail notify service: {e:?}")
            }

            if let Err(e) = sqlx::query!(
                "insert into decree_email_info (already_sent_id) values ($1)",
                decree_id
            )
            .execute(pool)
            .await
            {
                log::error!(
                    "could not insert into decree_email_info for decree {decree_id}: {e:?}"
                );
            }
        }
    }
}

pub async fn run_shut_pages(
    mut shutdown: Shutdown,
    app_state: AppState,
    pages: u32,
    ai_task_tx: Sender<AiTask>,
    only_latest: bool,
) {
    let mut decree_sleep = std::env::var("DECREE_SLEEP")
        .unwrap_or_else(|_| "1000".to_string())
        .parse()
        .unwrap_or_else(|e| {
            log::warn!(
                "Could not parse DECREE SLEEP env var: {e}. Using default value of 1000 seconds"
            );
            1000
        });

    if !only_latest {
        decree_sleep *= 60;
    }

    let latest_info = if only_latest { "latest" } else { "" };

    #[cfg(feature = "decrees")]
    while !shutdown.is_shutdown() {
        let ai_task_tx = ai_task_tx.clone();
        tokio::select! {
            _ = shutdown.recv() => {
                println!("shutting down");
                // return ok?
                break;
            }
            _ = async {
                let app_state = app_state.clone();
                if let Err(e) = tokio::task::spawn(async move {
                    log::info!("Starting {latest_info} Decree Scraper");
                    save_ministrial_decrees(pages, app_state, ai_task_tx, only_latest).await;
                }).await {
                    log::error!("Error in {latest_info} Decree Scraper: {:?}", e);
                }
                log::info!("{latest_info} Decree Scraper service sleeps for {decree_sleep} seconds...");
                tokio::time::sleep(tokio::time::Duration::from_secs(decree_sleep)).await;
            } => {}
        }
    }
}
