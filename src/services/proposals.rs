use diesel::RunQueryDsl;
use scraper::{
    proposals::{Proposal, ProposalScraper},
    AustriaScraper, ProposalPayload, Scraper,
};

use crate::{
    models::{DbProposal, ProposalDelegate},
    today, Shutdown,
};

use super::scrape_untill_success;

async fn scrape_new_proposals(prev_proposals: &[Proposal]) -> crate::Result<Vec<Proposal>> {
    let scraper = AustriaScraper::default();

    let today = chrono::Local::now().date_naive();
    let date_start = today - chrono::Duration::days(5);
    // let date_start = today;
    let date_end = today + chrono::Duration::days(1);

    let proposal_payload = ProposalPayload::new().with_date_range(date_start, date_end);

    let proposals = scraper.post_proposals_payload(proposal_payload).await?;
    let proposal_paths = scraper.proposal_paths(proposals)?;

    let mut new_proposals = Vec::new();

    for proposal_path in &proposal_paths {
        // skipping proposals that can't be processed right now
        let Ok(proposal) = scraper.get_proposal(&proposal_path).await else {
            continue;
        };
        if prev_proposals.contains(&proposal) {
            continue;
        }

        let documents = proposal.documents.clone();
        tokio::task::spawn_blocking(|| {
            for document in documents {
                let url = format!("{}{}", AustriaScraper::BASE_URL, document.link);
                let response = reqwest::blocking::get(&url).unwrap();
                let contents = response.text().unwrap();
                
                let path = std::path::PathBuf::from(format!(".{}", document.link));
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                std::fs::write(path, contents).unwrap();
            }
        }).await?;
        
        new_proposals.push(proposal);

    }
    Ok(new_proposals)
}

fn update_proposals_in_db(new_proposals: &[Proposal]) {
    use crate::schema::proposals;

    println!("props count: {}", new_proposals.len());
    for proposal in new_proposals {

        println!("id: {}, title: {}", proposal.id, proposal.title);

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
            println!("receiver_id: {}", receiver_id);
            insert_prop_del(&proposal.id, *receiver_id as i32, true);
        }

        for issuer_id in &proposal.issuer_ids {
            insert_prop_del(&proposal.id, *issuer_id as i32, false);
        }
    }
    println!("fin");
}

pub fn insert_prop_del(proposal_id: &str, delegate_id: i32, is_receiver: bool) {
    use crate::schema::proposal_delegates;

    let db_proposal_delegate = ProposalDelegate {
        proposal_id: proposal_id.to_string(),
        delegate_id,
        is_receiver
    };

    diesel::insert_into(proposal_delegates::table)
        .values(&db_proposal_delegate)
        .on_conflict((
            proposal_delegates::proposal_id,
            proposal_delegates::delegate_id,
        ))
        .do_nothing()
        .execute(&mut crate::establish_connection())
        .unwrap_or_default();
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
