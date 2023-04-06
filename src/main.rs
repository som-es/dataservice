mod services;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

use chrono::NaiveDate;
use scraper::{
    proposals::{Proposal, ProposalScraper},
    AustriaScraper, ProposalPayload,
};

#[inline]
pub fn today() -> NaiveDate {
    chrono::Local::now().date_naive()
}

async fn scrape_new_proposals(prev_propsals: &[Proposal]) -> Result<Vec<Proposal>> {
    let scraper = AustriaScraper::default();

    let date_start = chrono::Local::now().date_naive();

    let date_end = date_start + chrono::Duration::days(1);

    let proposal_payload = ProposalPayload::new().with_date_range(date_start, date_end);
    let json = proposal_payload.as_json();
    let json = scraper::serde_json::to_string_pretty(&json).unwrap();
    //println!("{json}");

    let proposals = scraper.post_proposals_payload(proposal_payload).await?;

    let proposal_paths = scraper.proposal_paths(proposals).unwrap();

    let mut new_proposals = Vec::new();

    for proposal_path in &proposal_paths {
        let proposal = scraper.get_proposal(&proposal_path).await?;
        if prev_propsals.contains(&proposal) {
            continue;
        }

        new_proposals.push(proposal);
    }

    Ok(new_proposals)
}

#[tokio::main]
async fn main() {
    services::proposals::run();

    loop {
        
    }
}
