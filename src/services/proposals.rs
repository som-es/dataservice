use std::{sync::Arc, thread::JoinHandle};

use dotenvy::dotenv;
use log::error;
use scraper::{
    proposals::{ProposalScraper, ProposalsPayload},
    AustriaScraper, Proposal,
};
use tokio::{sync::RwLock, time::Instant};

use crate::{connect_pg, Shutdown};

use super::scrape_untill_success;

async fn scrape_new_proposals() -> crate::Result<Vec<String>> {
    let scraper = AustriaScraper::default();

    let today = chrono::Local::now().date_naive();
    let date_start = today - chrono::Duration::days(365 * 5);
    // let date_start = today;
    let date_end = today;

    // let proposal_payload = ProposalsPayload::new().with_date_range(date_start, date_end);
    let proposal_payload = ProposalsPayload::default();

    let proposals = scraper.post_proposals_payload(proposal_payload).await?;
    let proposal_paths = scraper.proposal_paths(proposals)?;

    Ok(proposal_paths)
}

async fn scrape_new_proposals_last_5_days() -> crate::Result<Vec<String>> {
    let scraper = AustriaScraper::default();

    let today = chrono::Local::now().date_naive();
    let date_start = today - chrono::Duration::days(5);
    let date_end = today;

    let proposal_payload = ProposalsPayload::new().with_date_range(date_start, date_end);

    let proposals = scraper.post_proposals_payload(proposal_payload).await?;
    let proposal_paths = scraper.proposal_paths(proposals)?;

    Ok(proposal_paths)
}

use log::info;
use sqlx::PgPool;

pub async fn update_proposals_in_db2(
    pool: &PgPool,
    new_proposals_paths: &[String],
) -> Result<(), sqlx::Error> {
    info!(
        "Updating proposals in database... amount: {}",
        new_proposals_paths.len()
    );

    let scraper = Arc::new(AustriaScraper::default());
    let mut handles: Vec<tokio::task::JoinHandle<()>> = vec![];

    const MAX_TASKS: usize = 5;

    let start = Instant::now();

    for proposal_path in new_proposals_paths {
        let pool = pool.clone();
        let proposal_path = proposal_path.clone();
        let scraper = scraper.clone();

        if handles.len() > MAX_TASKS {
            for handle in handles.drain(..MAX_TASKS) {
                handle.await.unwrap()
            }
        }

        handles.push(tokio::spawn(async move {
            let pool = pool;
            let pool = &pool;
            let proposal = match scraper.get_proposal(&proposal_path).await {
                Ok(prop) => prop,
                Err(e) => {
                    error!("CANNOT PROCESS {proposal_path}, e: {e:?}");
                    return;
                    // continue;
                }
            };

            // Insert or update proposal
            if let Err(e) = sqlx::query!(
                r#"
            INSERT INTO proposals (id, title, description, ityp, gp, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (id) DO UPDATE SET
                title = EXCLUDED.title,
                description = EXCLUDED.description,
                ityp = EXCLUDED.ityp,
                gp = EXCLUDED.gp,
                created_at = EXCLUDED.created_at,
                updated_at = EXCLUDED.updated_at
            "#,
                proposal.id,
                proposal.title,
                proposal.description,
                proposal.ityp,
                proposal.gp,
                proposal.created_at,
                proposal.updated_at
            )
            .execute(pool)
            .await
            {
                log::error!("cannot insert proposal {proposal:?} into database: {e:?}");
                return;
            }

            // Insert or update documents
            for document in &proposal.documents {
                if let Err(e) = sqlx::query!(
                    r#"
                INSERT INTO proposal_documents (proposal_id, title, document_type, document_url)
                VALUES ($1, $2, $3, $4)
                ON CONFLICT (document_url, proposal_id) DO UPDATE SET
                    title = EXCLUDED.title,
                    document_type = EXCLUDED.document_type
                "#,
                    proposal.id,
                    document.title,
                    document.ftype.to_string(),
                    document.link
                )
                .execute(pool)
                .await
                {
                    log::error!("cannot insert document {document:?} into database: {e:?}")
                }
            }

            // Insert receivers
            for receiver_id in &proposal.receiver_ids {
                if let Err(e) =
                    insert_prop_del2(pool, &proposal.id, *receiver_id as i32, true).await
                {
                    error!("Error inserting receiver of proposal into database: '{e}'. The following proposal receiver was not inserted: {receiver_id:?}, for proposal: {proposal:?}");
                }
            }

            // Insert issuers
            for issuer_id in &proposal.issuer_ids {
                if let Err(e) = insert_prop_del2(pool, &proposal.id, *issuer_id as i32, false).await
                {
                    error!("Error inserting issuer of proposal into database: '{e}'. The following proposal issuer was not inserted: {issuer_id:?}, for proposal: {proposal:?}");
                }
            }

            // Insert topics
            if let Err(e) = insert_topics2(pool, &proposal).await {
                error!("cannot insert topics into db: {e:?}")
            }
        }));
    }

    for handle in handles {
        handle.await.unwrap();
    }

    log::info!(
        "---------- !!!!! complete proposal insertion took: {:?}",
        start.elapsed()
    );

    Ok(())
}

async fn insert_topics2(pool: &PgPool, proposal: &Proposal) -> Result<(), sqlx::Error> {
    for topic in &proposal.topics {
        sqlx::query!(
            r#"
            INSERT INTO topics_proposals (topic, proposals_id)
            VALUES ($1, $2)
            ON CONFLICT (proposals_id, topic) DO UPDATE SET
                topic = EXCLUDED.topic
            "#,
            topic,
            proposal.id
        )
        .execute(pool)
        .await?;
    }

    for topic in &proposal.eurovoc_topics {
        sqlx::query!(
            r#"
            INSERT INTO eurovoc_topics_proposals (topic, proposals_id)
            VALUES ($1, $2)
            ON CONFLICT (proposals_id, topic) DO UPDATE SET
                topic = EXCLUDED.topic
            "#,
            topic,
            proposal.id
        )
        .execute(pool)
        .await?;
    }

    for topic in &proposal.other_keyword_topics {
        sqlx::query!(
            r#"
            INSERT INTO other_keyword_topics_proposals (topic, proposals_id)
            VALUES ($1, $2)
            ON CONFLICT (proposals_id, topic) DO UPDATE SET
                topic = EXCLUDED.topic
            "#,
            topic,
            proposal.id
        )
        .execute(pool)
        .await?;
    }

    Ok(())
}

pub async fn insert_prop_del2(
    pool: &PgPool,
    proposal_id: &str,
    delegate_id: i32,
    is_receiver: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO proposal_delegates (proposal_id, delegate_id, is_receiver)
        VALUES ($1, $2, $3)
        ON CONFLICT (proposal_id, delegate_id) DO NOTHING
        "#,
        proposal_id,
        delegate_id,
        is_receiver
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn proposals_once2() {}

async fn proposals_once(pg_pool: &PgPool) {
    log::info!("Scraping new proposals...");

    let new_proposals =
        scrape_untill_success::<Vec<String>, _>(|| async { scrape_new_proposals().await }).await;
    update_proposals_in_db2(pg_pool, &new_proposals)
        .await
        .unwrap();
}

async fn proposals_once_last_5_days(pg_pool: &PgPool) {
    log::info!("Scraping new last 5 days proposals...");

    let new_proposals = scrape_untill_success::<Vec<String>, _>(|| async {
        scrape_new_proposals_last_5_days().await
    })
    .await;
    update_proposals_in_db2(pg_pool, &new_proposals)
        .await
        .unwrap();
}

pub async fn run_shut_last_5_days(mut shutdown: Shutdown) {
    dotenv().ok();

    let proposal_sleep = std::env::var("PROPOSAL_SLEEP")
        .unwrap_or_else(|_| "1000".to_string())
        .parse()
        .unwrap_or_else(|e| {
            log::error!("Error parsing PROPOSAL_SLEEP: {e}. Using default value of 1000 seconds.");
            100
        });

    while !shutdown.is_shutdown() {
        tokio::select! {
            _ = shutdown.recv() => {
                println!("shutting down");
                // return ok?
                break;
            }
            _ = async {
                let pg_pool = connect_pg().await;
                proposals_once_last_5_days(&pg_pool).await;
                log::info!("Sleeping for {proposal_sleep} seconds (last 5 days).");
                tokio::time::sleep(tokio::time::Duration::from_secs(proposal_sleep)).await;
            } => {}
        }
    }
}

pub async fn run_shut(mut shutdown: Shutdown) {
    dotenv().ok();

    let proposal_sleep_days = std::env::var("FULL_PROPOSAL_SLEEP")
        .unwrap_or_else(|_| "7".to_string())
        .parse()
        .unwrap_or_else(|e| {
            log::error!("Error parsing FULL_PROPOSAL_SLEEP: {e}. Using default value of 7 days.");
            7
        });
    let proposal_sleep = proposal_sleep_days * 24 * 60 * 60;
    while !shutdown.is_shutdown() {
        tokio::select! {
            _ = shutdown.recv() => {
                println!("shutting down");
                // return ok?
                break;
            }
            _ = async {
                let pg_pool = connect_pg().await;
                proposals_once(&pg_pool).await;
                log::info!("Sleeping for {proposal_sleep} seconds.");
                tokio::time::sleep(tokio::time::Duration::from_secs(proposal_sleep)).await;
            } => {}
        }
    }
}
