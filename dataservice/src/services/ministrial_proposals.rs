use std::{
    collections::HashMap,
    sync::{Arc, LazyLock},
};

use chrono::Utc;
use combx::{FromScrapedType, GovProposal, create_gov_officials_map};
use dotenvy_macro::dotenv;
use somes_scraper::{
    AustriaScraper, GovOfficial,
    ministrial_proposals::{GegenstandInfo, MinistrialProposal, MinistrialProposalScraper},
    proposals::ProposalsPayload,
};
use tokio::{sync::mpsc::Sender, time::Instant};

use crate::{
    AppState, CompareHash, GPS, Shutdown,
    ai_task::AiTask,
    api_limiter::ApiLimiter,
    enforce_ai_run,
    services::scrape_untill_success,
    should_update::StoredUpdatedAt,
    today,
    topics::{TableType, TopicType, insert_topics_generic_batch},
};

mod notify;
pub use notify::*;

mod ai_tasks;
pub use ai_tasks::*;

mod inserts;
pub use inserts::*;

mod db;

pub async fn scrape_new_ministrial_props(
    scraper: &AustriaScraper,
    parliament_api_limiter: ApiLimiter,
) -> crate::Result<Vec<MinistrialProposal>> {
    let mut proposal_payload = ProposalsPayload::default();
    proposal_payload.scrape_info = scraper.scrape_info.clone();
    let ministrial_proposals = parliament_api_limiter
        .run(scraper.post_ministrial_proposals_payload(proposal_payload))
        .await?;
    let mut ministrial_props_paths = scraper.ministrial_prop_paths(ministrial_proposals)?;
    sort_ministerial_prop_paths_desc(&mut ministrial_props_paths);

    let mut new_ministrial_props = Vec::with_capacity(ministrial_props_paths.len());

    for ministrial_prop_path in ministrial_props_paths {
        let Ok(ministrial_prop) = parliament_api_limiter
            .run(scraper.get_ministrial_proposal(ministrial_prop_path.clone()))
            .await
        else {
            log::debug!("Cannot process ministrial proposal: '{ministrial_prop_path:?}'");
            continue;
        };

        new_ministrial_props.push(ministrial_prop);
    }

    Ok(new_ministrial_props)
}

pub async fn ministrial_proposal_once(
    only_latest: bool,
    pg_pool: &AppState,
    ai_task_tx: Sender<AiTask>,
) -> Vec<MinistrialProposal> {
    log::info!("Try scraping ministiral proposals");
    let mut scraper = AustriaScraper::default();

    let gov_officials = Arc::new(create_gov_officials_map().await);

    for gp in GPS {
        log::info!("scrape ministrial proposals for GP: {gp}");
        scraper.scrape_info.gp = vec![gp.into()];
        if only_latest {
            // this seems to not work! - Minsterial Props and NRSITZ do not have date filtering???
            // scraper.scrape_info.date_start = today_and_time()
            //     .naive_local()
            //     .checked_sub_months(Months::new(3))
        }

        let new_ministrial_props = scrape_untill_success::<Vec<_>, _>(|| async {
            scrape_new_ministrial_props(&scraper, pg_pool.parliament_api_limiter.clone()).await
        })
        .await;

        log::info!(
            "scraped {} new ministrial props",
            new_ministrial_props.len()
        );

        if let Err(e) = update_ministrial_proposals_in_db2(
            pg_pool,
            new_ministrial_props,
            gov_officials.clone(),
            Some(ai_task_tx.clone()),
            only_latest,
        )
        .await
        {
            log::error!(
                "could not insert all ministerial proposals for gp: {gp}, because of: {e:?}"
            );
        }
        if only_latest {
            break;
        }
        scraper.scrape_info.date_start = None;
    }

    // new_legis_inits
    vec![]
}

const MINISTRIAL_PROP_STILL_SEND_DAYS: LazyLock<i64> = LazyLock::new(|| {
    dotenv!("LEGIS_INIT_STILL_SEND_DAYS")
        .parse::<i64>()
        .unwrap_or(10)
});

use sqlx::{postgres::PgPool, query};

impl StoredUpdatedAt for MinistrialProposal {
    async fn stored_updated_at(&self, pg: &PgPool) -> Option<chrono::DateTime<Utc>> {
        query!(
            "select raw_data_updated_at from ministrial_proposals where gp = $1 and ityp = $2 and inr = $3",
            self.gp,
            self.ityp,
            self.inr as i32
        )
        .fetch_optional(pg)
        .await
        .ok()
        .flatten()
        .and_then(|x| x.raw_data_updated_at)
    }
}

impl CompareHash for GovProposal {
    async fn stored_hash(&self, pg: &PgPool) -> Option<u64> {
        sqlx::query_scalar!(
            "select hash from ministrial_proposals where gp = $1 and ityp = $2 and inr = $3",
            self.ministrial_proposal.gp,
            self.ministrial_proposal.ityp,
            self.ministrial_proposal.inr as i32
        )
        .fetch_one(pg)
        .await
        .ok()
        .flatten()
        .map(|hash| hash as u64)
    }
}

pub async fn update_or_insert_ministerial_proposal(
    app_state: &AppState,
    ai_task_tx: Option<Sender<AiTask>>,
    gov_officials: Arc<HashMap<u64, Vec<GovOfficial>>>,
    ministrial_prop: MinistrialProposal,
    new_ministrial_prop_tx: Option<Sender<(i32, GovProposal)>>,
) -> sqlx::Result<()> {
    let pool = &app_state.pg_pool;
    let mut gov_proposal =
        GovProposal::from_scraped_type((), (ministrial_prop, gov_officials.clone())).await;

    let (should_update, hash) = crate::should_update_by_hash(&gov_proposal, pool).await;
    if !enforce_ai_run() && !should_update {
        log::info!(
            "No need to update ministrial proposal: {}/ME/{}",
            gov_proposal.ministrial_proposal.gp,
            gov_proposal.ministrial_proposal.inr
        );
        return Ok(());
    }

    log::info!(
        "Processing ministrial proposal: {}/ME/{}",
        gov_proposal.ministrial_proposal.gp,
        gov_proposal.ministrial_proposal.inr
    );
    // log::info!("inserting single ministrial prop");

    let _start = Instant::now();
    // Insert or update the ministrial proposal
    let ministrial_prop_id = sqlx::query!(
                r#"
                INSERT INTO ministrial_proposals (
                    title, inr, emphasis, description, ityp, gp, raw_data_created_at, raw_data_updated_at,
                    due_to, ressort, ressort_shortform, legis_init_gp, legis_init_inr, legis_init_ityp, has_vote_result,
                    updated_at, hash
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
                ON CONFLICT (inr, ityp, gp)
                DO UPDATE SET
                    title = EXCLUDED.title,
                    emphasis = EXCLUDED.emphasis,
                    description = EXCLUDED.description,
                    raw_data_created_at = EXCLUDED.raw_data_created_at,
                    raw_data_updated_at = EXCLUDED.raw_data_updated_at,
                    due_to = EXCLUDED.due_to,
                    ressort = EXCLUDED.ressort,
                    ressort_shortform = EXCLUDED.ressort_shortform,
                    legis_init_gp = EXCLUDED.legis_init_gp,
                    legis_init_inr = EXCLUDED.legis_init_inr,
                    legis_init_ityp = EXCLUDED.legis_init_ityp,
                    has_vote_result = EXCLUDED.has_vote_result,
                    updated_at = EXCLUDED.updated_at,
                    hash = EXCLUDED.hash
                RETURNING id
                "#,
                gov_proposal.ministrial_proposal.title,
                gov_proposal.ministrial_proposal.inr,
                gov_proposal.ministrial_proposal.emphasis,
                gov_proposal.ministrial_proposal.description,
                gov_proposal.ministrial_proposal.ityp,
                gov_proposal.ministrial_proposal.gp,
                gov_proposal.ministrial_proposal.raw_data_created_at,
                gov_proposal.ministrial_proposal.raw_data_updated_at,
                gov_proposal.ministrial_proposal.due_to,
                gov_proposal.ministrial_proposal.ressort,
                gov_proposal.ministrial_proposal.ressort_shortform,
                gov_proposal.ministrial_proposal.legis_init_gp,
                gov_proposal.ministrial_proposal.legis_init_inr,
                gov_proposal.ministrial_proposal.legis_init_ityp,
                gov_proposal.ministrial_proposal.has_vote_result,
                crate::today_and_time(),
                hash as i64
            )
            .fetch_one(pool)
            .await?
            .id;

    gov_proposal.id = Some(ministrial_prop_id);

    #[cfg(feature = "ai")]
    if let Some(ai_task_tx) = &ai_task_tx {
        use crate::{LATEST_GP, ai_task::Priority};

        let (priority, inner_gov_prop_mail_tx) =
            if gov_proposal.ministrial_proposal.raw_data_created_at
                >= today() - chrono::Duration::days(90)
            {
                (Priority::Highest, new_ministrial_prop_tx.clone())
            } else if gov_proposal.ministrial_proposal.gp == LATEST_GP {
                (Priority::High, None)
            } else {
                (Priority::Lowest, None)
            };

        let inner_gov_proposal = gov_proposal.clone();
        let pool = pool.clone();
        let redis_client = app_state.redis_client.clone();
        let documents = gov_proposal.documents.clone();
        let summarization_task = AiTask {
            input: None,
            priority,
            pool: pool.clone(),
            score: priority.weight(),
            task_id: crate::hash!(app_state.parliament),
            db_entry_id: ministrial_prop_id,
            rerunnable: true,
            created_at: Instant::now(),
            task: Box::new(move |task_args| {
                let inner_gov_proposal = inner_gov_proposal.clone();
                let pool = pool.clone();
                let redis_client = redis_client.clone();
                let inner_gov_prop_mail_tx = inner_gov_prop_mail_tx.clone();
                let documents = documents.clone();
                Box::pin(async move {
                    let ai_summary =
                        summarize_ministerial_proposal_documents(task_args, &documents).await?;

                    let is_ai_summary_none = ai_summary.is_none();
                    // not returning on None to still send gov prop
                    let mut inner_gov_proposal = inner_gov_proposal;
                    inner_gov_proposal.ai_summary = ai_summary;

                    if let Some(gov_prop_mail_tx) = inner_gov_prop_mail_tx {
                        check_and_send_ministrial_email(
                            &pool,
                            &inner_gov_proposal,
                            ministrial_prop_id,
                            &gov_prop_mail_tx,
                        )
                        .await;
                    }

                    if is_ai_summary_none {
                        return Ok(());
                    }

                    let Some(mut redis_client) = redis_client else {
                        return Ok(());
                    };

                    crate::notify_update_streamed(
                        combx::Index::GovProposals,
                        &inner_gov_proposal,
                        &mut redis_client,
                    )
                    .await?;
                    Ok(())
                })
            }),
        };

        log::info!(
            "Sending AI task for ministerial prop summarization: {ministrial_prop_id} / {}/ME/{} with priority {priority:?}",
            gov_proposal.ministrial_proposal.gp,
            gov_proposal.ministrial_proposal.inr
        );

        if let Err(e) = ai_task_tx.send(summarization_task).await {
            log::error!("Could not send AI task for ministerial prop summarization: {e:?}");
        }
    }

    if enforce_ai_run() && !should_update {
        log::info!("No need to update further!");
        return Ok(());
    }

    log::info!(
        "Continue updating ministrial proposal related data for proposal {}/ME/{}",
        gov_proposal.ministrial_proposal.gp,
        gov_proposal.ministrial_proposal.inr
    );

    let mut tx = pool.begin().await?;

    if let Err(e) = sqlx::query!(
        "delete from ministrial_issuer where ministrial_proposal_id = $1",
        ministrial_prop_id
    )
    .execute(&mut *tx)
    .await
    {
        log::error!(
            "Cannot delete all ministrial proposal issuers from {ministrial_prop_id} because of: {e:}"
        );
    }

    for issuer_id in &gov_proposal.ministerial_issuers {
        if let Err(e) = sqlx::query!(
            r#"
                    INSERT INTO ministrial_issuer (delegate_id, ministrial_proposal_id)
                    VALUES ($1, $2)
                    ON CONFLICT (ministrial_proposal_id, delegate_id)
                    DO NOTHING
                    "#,
            *issuer_id,
            ministrial_prop_id
        )
        .execute(&mut *tx)
        .await
        {
            log::error!(
                "Cannot insert ministrial issuer for {ministrial_prop_id} because of: {e:}"
            );
        }
    }

    if let Err(e) = tx.commit().await {
        log::error!(
            "Cannot commit transaction for ministrial issuer replacement: {ministrial_prop_id}, {e:?}"
        );
    }

    #[cfg(not(feature = "ai"))]
    if let Some(new_ministrial_prop_tx) = &new_ministrial_prop_tx {
        check_and_send_ministrial_email(
            pool,
            &gov_proposal,
            ministrial_prop_id,
            new_ministrial_prop_tx,
        )
        .await;
    }

    let mut tx = pool.begin().await?;

    if let Err(e) = insert_topics_generic_batch(
        &mut tx,
        &gov_proposal.topics,
        ministrial_prop_id,
        TopicType::Standard,
        TableType::MinisterialProposal,
    )
    .await
    {
        log::error!("Error inserting topics for legis init {ministrial_prop_id}: {e:?}");
    }

    if let Err(e) = insert_topics_generic_batch(
        &mut tx,
        &gov_proposal.eurovoc_topics,
        ministrial_prop_id,
        TopicType::Eurovoc,
        TableType::MinisterialProposal,
    )
    .await
    {
        log::error!("Error inserting eurovoc topics for legis init {ministrial_prop_id}: {e:?}");
    }
    if let Err(e) = insert_topics_generic_batch(
        &mut tx,
        &gov_proposal.other_keyword_topics,
        ministrial_prop_id,
        TopicType::OtherKeyword,
        TableType::MinisterialProposal,
    )
    .await
    {
        log::error!(
            "Error inserting other keyword topics for legis init {ministrial_prop_id}: {e:?}"
        );
    }

    let mut tx = pool.begin().await?;
    // Insert or update documents
    if let Err(e) =
        insert_ministerial_documents_batch(&mut tx, &gov_proposal.documents, ministrial_prop_id)
            .await
    {
        log::error!("Error inserting documents for ministerial prop {ministrial_prop_id}: {e:?}");
    }

    tx.commit().await?;

    if should_update
        && let Some(client) = &app_state.redis_client
        && let Err(e) = crate::notify_update_streamed(
            combx::Index::GovProposals,
            &gov_proposal,
            &mut client.clone(),
        )
        .await
    {
        log::error!("Could not notify for gov prop update!: {e:?}");
    }
    Ok(())
}

pub async fn update_ministrial_proposals_in_db2(
    app_state: &AppState,
    ministrial_props: Vec<MinistrialProposal>,
    gov_officials: Arc<HashMap<u64, Vec<GovOfficial>>>,
    ai_task_tx: Option<Sender<AiTask>>,
    only_latest: bool,
) -> Result<(), sqlx::Error> {
    log::info!(
        "Updating {} ministrial proposal in database...",
        ministrial_props.len()
    );

    let pool = &app_state.pg_pool;

    let (new_ministrial_prop_tx, new_ministrial_prop_rx) = if only_latest {
        let (tx, rx) = tokio::sync::mpsc::channel::<(i32, GovProposal)>(1024);
        (Some(tx), Some(rx))
    } else {
        (None, None)
    };

    for ministrial_prop in ministrial_props.into_iter() {
        let gp = ministrial_prop.gp.clone();
        let inr = ministrial_prop.inr;
        if let Err(e) = update_or_insert_ministerial_proposal(
            app_state,
            ai_task_tx.clone(),
            gov_officials.clone(),
            ministrial_prop,
            new_ministrial_prop_tx.clone(),
        )
        .await
        {
            log::error!("could not update or insert ministerial prop {gp}/{inr} because of {e:}",)
        }
        // let should_update = crate::should_update_by_time(&ministrial_prop, pool).await;
    }

    drop(new_ministrial_prop_tx);

    let pool = pool.clone();
    tokio::task::spawn(async move {
        if let Some(mut new_ministrial_prop_rx) = new_ministrial_prop_rx {
            let mut all_gov_proposals = vec![];
            while let Some(gov_proposal) = new_ministrial_prop_rx.recv().await {
                all_gov_proposals.push(gov_proposal);
            }

            let simple_gov_officials = get_active_gov_officals(&pool).await;
            let simple_gov_officials = simple_gov_officials
                .unwrap_or_default()
                .into_iter()
                .map(|gov| (gov.id, gov))
                .collect::<HashMap<_, _>>();

            let notify_user_gov_props =
                find_notifiers_ministrial_props(&pool, &all_gov_proposals).await;

            notify_users(&notify_user_gov_props, &simple_gov_officials);
        }
    });

    Ok(())
}

async fn check_and_send_ministrial_email(
    pool: &sqlx::Pool<sqlx::Postgres>,
    gov_proposal: &GovProposal,
    ministrial_prop_id: i32,
    new_gov_prop_tx: &Sender<(i32, GovProposal)>,
) {
    if gov_proposal.eurovoc_topics.is_empty()
        && gov_proposal
            .ai_summary
            .as_ref()
            .map(|ai_summary| ai_summary.full_summary.topics.is_empty())
            .unwrap_or(true)
    {
        return;
    }
    if let Ok(info) = sqlx::query!(
        "select * from ministrial_email_info where already_sent_id = $1",
        ministrial_prop_id
    )
    .fetch_optional(pool)
    .await
    {
        let diff = today().to_utc() - gov_proposal.ministrial_proposal.raw_data_created_at;

        if diff.num_days() <= *MINISTRIAL_PROP_STILL_SEND_DAYS && info.is_none() {
            log::info!("Sending fitting ministrial prop: {ministrial_prop_id}");
            if let Err(e) = new_gov_prop_tx
                .send((ministrial_prop_id, gov_proposal.clone()))
                .await
            {
                log::error!("could not send ministrial prop to mail notify service: {e:?}")
            }

            if let Err(e) = sqlx::query!(
                "insert into ministrial_email_info (already_sent_id) values ($1)",
                ministrial_prop_id
            )
            .execute(pool)
            .await
            {
                log::error!(
                    "could not insert into ministrial email info for id {ministrial_prop_id}: {e:?}"
                );
            }
        }
    }
}

pub async fn run_shut_latest_gp(
    mut shutdown: Shutdown,
    app_state: AppState,
    ai_task_tx: Sender<AiTask>,
) {
    let legis_init_sleep = std::env::var("MINISTRIAL_PROPOSAL_LATEST_SLEEP")
        .unwrap_or_else(|_| "2000".to_string())
        .parse()
        .unwrap_or_else(|e| {
            log::warn!(
                "Could not parse MINISTRIAL_PROPOSAL_LATEST_SLEEP env var: {e}. Using default value of 2000 seconds"
            );
            2000
        });
    #[cfg(feature = "ministerial_latest")]
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
                    log::info!("Starting ministrial proposal (latest) scraper");
                    ministrial_proposal_once(true, &app_state, ai_task_tx).await;
                }).await {
                    log::error!("Error in ministrial proposal (latest) scraper: {:?}", e);
                }
                // legis_init_once().await;
                log::info!("ministrial proposal (latest) scraper service sleeps for {legis_init_sleep} seconds...");
                tokio::time::sleep(tokio::time::Duration::from_secs(legis_init_sleep)).await;
            } => {}
        }
    }
}

pub async fn run_shut_every_gp(
    mut shutdown: Shutdown,
    app_state: AppState,
    only_latest: bool,
    ai_task_tx: Sender<AiTask>,
) {
    if only_latest {
        return;
    }
    let legis_init_sleep = std::env::var("MINISTRIAL_PROPOSAL_SLEEP")
        .unwrap_or_else(|_| "10000".to_string())
        .parse()
        .unwrap_or_else(|e| {
            log::warn!(
                "Could not parse MINISTRIAL_PROPOSAL_SLEEP env var: {e}. Using default value of 10000 seconds"
            );
            10000
        });

    #[cfg(feature = "ministerial")]
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
                    log::info!("Starting ministrial proposal scraper");

                    ministrial_proposal_once(false, &app_state, ai_task_tx).await;
                }).await {
                    log::error!("Error in ministrial proposal scraper: {:?}", e);
                }
                // legis_init_once().await;
                log::info!("ministrial proposal scraper service sleeps for {legis_init_sleep} seconds...");
                tokio::time::sleep(tokio::time::Duration::from_secs(legis_init_sleep)).await;
            } => {}
        }
    }
}

fn convert_inr_from_path_to_num(path: &str) -> Option<usize> {
    let start = path.rfind('/')?;
    path[start + 1..].parse().ok()
}

fn sort_ministerial_prop_paths_desc(prop_paths: &mut Vec<GegenstandInfo>) {
    prop_paths.sort_by(|a, b| {
        convert_inr_from_path_to_num(&b.path)
            .unwrap()
            .cmp(&convert_inr_from_path_to_num(&a.path).unwrap())
    });
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDateTime;
    use somes_scraper::{AustriaScraper, ministrial_proposals::GegenstandInfo};

    use crate::{ministrial_proposals::sort_ministerial_prop_paths_desc, today};

    #[test]
    fn test_sort_ministerial_paths() {
        let mut paths = vec![
            GegenstandInfo {
                path: "/gegenstand/XXVIII/ME/1".into(),
                ressort: "wenig".into(),
                ressort_shortform: "nix".into(),
                due_to: today().date_naive(),
            },
            GegenstandInfo {
                path: "/gegenstand/XXVIII/ME/15".into(),
                ressort: "wenig".into(),
                ressort_shortform: "nix".into(),
                due_to: today().date_naive(),
            },
            GegenstandInfo {
                path: "/gegenstand/XXVIII/ME/32".into(),
                ressort: "wenig".into(),
                ressort_shortform: "nix".into(),
                due_to: today().date_naive(),
            },
            GegenstandInfo {
                path: "/gegenstand/XXVIII/ME/101".into(),
                ressort: "wenig".into(),
                ressort_shortform: "nix".into(),
                due_to: today().date_naive(),
            },
            GegenstandInfo {
                path: "/gegenstand/XXVIII/ME/10".into(),
                ressort: "wenig".into(),
                ressort_shortform: "nix".into(),
                due_to: today().date_naive(),
            },
            GegenstandInfo {
                path: "/gegenstand/XXVIII/ME/40".into(),
                ressort: "wenig".into(),
                ressort_shortform: "nix".into(),
                due_to: today().date_naive(),
            },
        ];
        sort_ministerial_prop_paths_desc(&mut paths);
        dbg!(&paths);
    }

    #[test]
    fn test_date_time_parse() {
        let x = NaiveDateTime::parse_from_str("2014-5-17T12:34:56", "%Y-%m-%dT%H:%M:%S").unwrap();
        println!("x: {x:?}");
    }

    #[tokio::test]
    async fn test_legis_init_empty_gp_scrape() {
        let mut scraper: AustriaScraper = AustriaScraper::default();
        // println!("s: {:?}", scraper.scrape_info.gp);

        // let out = crate::legislative_initiatives::scrape_new_legis_inits(&scraper).await.unwrap();
        // println!("out: {out:?}");
    }
}
