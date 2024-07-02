use diesel::RunQueryDsl;
use dotenvy::dotenv;
use log::error;
use scraper::{
    proposals::{Proposal, ProposalScraper, ProposalsPayload},
    AustriaScraper, Fetch,
};

use crate::{
    models::{DbProposal, DbProposalDocument, ProposalDelegate},
    schema::proposal_documents,
    today, Shutdown,
};

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

async fn update_proposals_in_db(new_proposals_paths: &[String]) {
    let mut conn = crate::establish_connection();
    use crate::schema::proposals;
    log::info!("Updating proposals in database...");
    let scraper = AustriaScraper::default();
    for proposal_path in new_proposals_paths {
        let Ok(proposal) = scraper.get_proposal(proposal_path).await else {
            error!("CANNOT PROCESS {proposal_path}");
            continue;
        };
        let db_proposal = DbProposal {
            id: proposal.id.clone(),
            title: proposal.title.clone(),
            description: proposal.description.clone(),
            ityp: proposal.ityp.clone(),
            gp: proposal.gp.clone(),
            created_at: proposal.created_at,
            updated_at: proposal.updated_at
        };

        if let Err(e) = diesel::insert_into(proposals::table)
            .values(&db_proposal)
            .on_conflict(proposals::id)
            .do_update()
            .set(&db_proposal)
            .execute(&mut conn)
        {
            log::error!("Error inserting proposal into database: '{e}'. The following proposal was not inserted: {proposal:?}");
            continue;
        }

        for document in &proposal.documents {
            let legis_doc = DbProposalDocument {
                proposal_id: proposal.id.clone(),
                title: document.title.clone(),
                document_type: document.ftype.to_string(),
                document_url: document.link.clone(),
            };

            if let Err(e) = diesel::insert_into(proposal_documents::table)
                .values(&legis_doc)
                .on_conflict((
                    proposal_documents::document_url,
                    proposal_documents::proposal_id,
                ))
                .do_update()
                .set(&legis_doc)
                .execute(&mut conn)
            {
                log::error!("(proposal) Error inserting document: '{e}'. The error occured for the following document: {document:?}")
            }
        }

        for receiver_id in &proposal.receiver_ids {
            if let Err(e) = insert_prop_del(&proposal.id, *receiver_id as i32, true) {
                log::error!("Error inserting receiver of proposal into database: '{e}'. The following proposal receiver was not inserted: {receiver_id:?}, for proposal: {proposal:?}");
                continue;
            }
        }

        for issuer_id in &proposal.issuer_ids {
            if let Err(e) = insert_prop_del(&proposal.id, *issuer_id as i32, false) {
                log::error!("Error inserting issuer of proposal into database: '{e}'. The following proposal issuer was not inserted: {issuer_id:?}, for proposal: {proposal:?}");
                continue;
            }
        }
    }
}

pub fn insert_prop_del(
    proposal_id: &str,
    delegate_id: i32,
    is_receiver: bool,
) -> diesel::QueryResult<usize> {
    use crate::schema::proposal_delegates;

    let db_proposal_delegate = ProposalDelegate {
        proposal_id: proposal_id.to_string(),
        delegate_id,
        is_receiver,
    };

    diesel::insert_into(proposal_delegates::table)
        .values(&db_proposal_delegate)
        .on_conflict((
            proposal_delegates::proposal_id,
            proposal_delegates::delegate_id,
        ))
        .do_nothing()
        .execute(&mut crate::establish_connection())
}

#[derive(Debug, Default)]
pub struct PropInit {}

pub async fn proposals_once2() {}

async fn proposals_once() {
    log::info!("Scraping new proposals...");

    let new_proposals =
        scrape_untill_success::<Vec<String>, _>(|| async { scrape_new_proposals().await }).await;
    update_proposals_in_db(&new_proposals).await;
}

async fn proposals_once_last_5_days() {
    log::info!("Scraping new last 5 days proposals...");

    let new_proposals =
        scrape_untill_success::<Vec<String>, _>(|| async { scrape_new_proposals_last_5_days().await }).await;
    update_proposals_in_db(&new_proposals).await;
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
                proposals_once_last_5_days().await;
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
                proposals_once().await;
                log::info!("Sleeping for {proposal_sleep} seconds.");
                tokio::time::sleep(tokio::time::Duration::from_secs(proposal_sleep)).await;
            } => {}
        }
    }
}
