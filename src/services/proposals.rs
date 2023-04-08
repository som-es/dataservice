use scraper::{
    proposals::{Proposal, ProposalScraper},
    AustriaScraper, ProposalPayload,
};

use crate::{today, Shutdown};

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
                for prop in &prev_proposals {
                    println!("title: {}, id: {}, issuer: {:?}, recv: {:?},", prop.title, prop.id, prop.issuer_ids, prop.receiver_ids);
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        
                if today() > start_date {
                    prev_proposals.clear();
                }
            } => {}
        }
    }

}
