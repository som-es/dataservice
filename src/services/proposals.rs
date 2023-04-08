use diesel::RunQueryDsl;
use scraper::{
    proposals::{Proposal, ProposalScraper},
    AustriaScraper, ProposalPayload,
};

use crate::{today, Shutdown, models::{DbProposal, ProposalDelegate}};

use super::scrape_untill_success;

async fn scrape_new_proposals(prev_proposals: &[Proposal]) -> crate::Result<Vec<Proposal>> {
    let scraper = AustriaScraper::default();

    let date_start = chrono::Local::now().date_naive();
    let date_start = date_start - chrono::Duration::days(2);
    let date_end = date_start + chrono::Duration::days(1);

    let proposal_payload = ProposalPayload::new().with_date_range(date_start, date_end);

    let proposals = scraper.post_proposals_payload(proposal_payload).await?;
    let proposal_paths = scraper.proposal_paths(proposals)?;

    let mut new_proposals = Vec::new();

    for proposal_path in &proposal_paths {
        let proposal = scraper.get_proposal(&proposal_path).await?;
        if prev_proposals.contains(&proposal) {
            continue;
        }

        new_proposals.push(proposal);
    }
    Ok(new_proposals)
}

fn update_proposals_in_db(new_proposals: &[Proposal]) {
    use crate::schema::proposals;

    for proposal in new_proposals {

        let db_proposal = DbProposal {
            id: proposal.id.clone(),
            title: proposal.title.clone(),
            description: proposal.description.clone(),
            ityp: proposal.ityp.clone(),
            gp: proposal.gp.clone(), 
        };

        diesel::insert_into(proposals::table)
            .values(&db_proposal)
            .on_conflict(proposals::id)
            .do_nothing()
            .execute(&mut crate::establish_connection())
            .expect("msg");

        for receiver_id in &proposal.receiver_ids {
            insert_prop_del(&proposal.id, *receiver_id as i32)
        }

        for issuer_id in &proposal.receiver_ids {
            insert_prop_del(&proposal.id, *issuer_id as i32)
        }
        
    }
}

pub fn insert_prop_del(proposal_id: &str, delegate_id: i32) {
    use crate::schema::proposal_delegates;

    let db_proposal_delegate = ProposalDelegate {
        proposal_id: proposal_id.to_string(),
        delegate_id,
    };

    diesel::insert_into(proposal_delegates::table)
        .values(&db_proposal_delegate)
        .on_conflict((proposal_delegates::proposal_id, proposal_delegates::delegate_id))
        .do_nothing()
        .execute(&mut crate::establish_connection())
        .expect("msg");
}

#[derive(Debug, Default)]
pub struct PropInit {}

pub async fn proposals_once2() {}

async fn proposals_once(prev_proposals: &[Proposal]) -> Vec<Proposal> {
    println!("try scraping data"); // convert to log

    let new_proposals = scrape_untill_success::<Vec<Proposal>, _>(|| async {
        scrape_new_proposals(&prev_proposals).await
    })
    .await;

    update_proposals_in_db(&new_proposals);

    new_proposals
}

pub async fn run() {
    let mut prev_proposals = Vec::new();

    let start_date = today();

    loop {
        prev_proposals.extend(proposals_once(&prev_proposals).await);

        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

        if today() > start_date {
            prev_proposals.clear();
        }
    }
}


pub async fn run_shut(mut shutdown: Shutdown) {
    let mut prev_proposals = Vec::new();

    let start_date = today();

    while !shutdown.is_shutdown() {
        tokio::select! {
            _ = shutdown.recv() => {
                println!("shutting down");
                // return ok?
                break;
            }
            _ = async {
                prev_proposals.extend(proposals_once(&prev_proposals).await);
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        
                if today() > start_date {
                    prev_proposals.clear();
                }
            } => {}
        }
    }

}
