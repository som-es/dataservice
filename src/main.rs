
use chrono::NaiveDate;
use scraper::{AustriaScraper, ProposalPayload, proposals::{ProposalScraper, Proposal}};

async fn scrape_new_proposals(prev_propsals: &[Proposal]) -> Result<Vec<Proposal>, Box<dyn std::error::Error>> {
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
    // add logs etc clean up

    // reset daily
    let mut prev_proposals = vec![];
    loop {

        let mut new_proposals = scrape_new_proposals(&prev_proposals).await;
        while let Err(e) = new_proposals {
            new_proposals = scrape_new_proposals(&prev_proposals).await;
            println!("Error: {e}");
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
        let new_proposals = new_proposals.expect("Above loop always runs till Ok");

        println!("Found {} new proposals", new_proposals.len());

        for proposal in &new_proposals {
            println!("Proposal: {}", proposal.title);
        }

        prev_proposals.extend(new_proposals);

        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    }    
}
