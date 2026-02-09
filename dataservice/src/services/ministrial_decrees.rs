use std::{
    hash::{Hash, Hasher},
    sync::Arc,
};

use combx::{
    with_data::gps::{gps, LegislativePeriod},
    Decree, FromScrapedType, PostgresEnhancer,
};
use somes_scraper::{decree::RisScraper, AustriaScraper, Thing};
use sqlx::PgPool;
use tokio::sync::mpsc::Sender;

use crate::{ai_task::AiTask, should_update_by_hash, today, AppState, CompareHash, Shutdown};

mod ai_tasks;

pub async fn save_ministrial_decrees(pages: u32, app_state: AppState, ai_task_tx: Sender<AiTask>) {
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

    for page in 1..=pages {
        log::info!("Currently on page {page}..");
        let Ok(decrees) = scraper.fetch_and_parse_ministrial_decrees(page).await else {
            log::error!("Cannot scrape ministrial decrees on page {page}");
            continue;
        };

        for decree in decrees {
            if let Err(e) =
                insert_ministrial_decree(decree, &app_state, all_gps.clone(), &ai_task_tx).await
            {
                log::error!("Cannot insert decree: {e:?}");
            }
        }
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

    fn hash_of(&self) -> u64 {
        let mut hasher = std::hash::DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }
}

pub async fn insert_ministrial_decree(
    decree: Thing,
    app_state: &AppState,
    all_gps: Arc<[LegislativePeriod]>,
    ai_task_tx: &Sender<AiTask>,
) -> crate::Result<()> {
    let pg_pool = &app_state.pg_pool;
    let decree =
        combx::Decree::from_scraped_type(PostgresEnhancer(pg_pool.clone()), (decree, all_gps))
            .await;

    let (should_update, hash) = should_update_by_hash(&decree, pg_pool).await;

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

        if should_update {
            if let Some(client) = &app_state.redis_client {
                if let Err(e) = crate::notify_update_streamed("decrees", &decree, client).await {
                    log::error!("Could not notify for decree update!: {e:?}");
                }
            }
        }

        #[cfg(feature = "ai")]
        {
            use crate::{ai_task::Priority, LATEST_GP};

            let priority =
                if decree.publication_date >= (today() - chrono::Duration::days(90)).date_naive() {
                    Priority::Highest
                } else if decree.gp.as_deref() == Some(LATEST_GP) {
                    Priority::High
                } else {
                    Priority::Lowest
                };

            let inner_decree = decree.clone();
            let pool = pg_pool.clone();
            let documents = decree.documents.clone();
            let summarization_task = AiTask {
                input: None,
                priority,
                task_id: crate::hash!(),
                db_entry_id: decree_id.id,
                task: Box::new(move |task_args| {
                    Box::pin(async move {
                        use crate::ministrial_decrees::ai_tasks::summarize_decrees_documents;

                        let _ai_summary =
                            summarize_decrees_documents(task_args, &documents).await?;

                        Ok(())
                        // TODO: update meilisearch index on finish
                    })
                }),
            };

            log::info!(
                "Sending AI task for ministerial decree summarization: {} ({:?}) with priority {priority:?}",
                decree_id.id,
                decree.gp,
            );

            if let Err(e) = ai_task_tx.send(summarization_task).await {
                log::error!("Could not send AI task for ministerial decree summarization: {e:?}");
            }
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
                decree_id.id,
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

pub async fn run_shut_pages(
    mut shutdown: Shutdown,
    app_state: AppState,
    pages: u32,
    ai_task_tx: Sender<AiTask>,
) {
    let decree_sleep = std::env::var("DECREE_SLEEP")
        .unwrap_or_else(|_| "1000".to_string())
        .parse()
        .unwrap_or_else(|e| {
            log::warn!(
                "Could not parse DECREE SLEEP env var: {e}. Using default value of 1000 seconds"
            );
            1000
        });

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
                    log::info!("Starting Decree Scraper");
                    save_ministrial_decrees(pages, app_state, ai_task_tx).await;
                }).await {
                    log::error!("Error in Decree Scraper: {:?}", e);
                }
                log::info!("Decree Scraper service sleeps for {decree_sleep} seconds...");
                tokio::time::sleep(tokio::time::Duration::from_secs(decree_sleep)).await;
            } => {}
        }
    }
}
