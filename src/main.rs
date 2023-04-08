mod services;
mod shutdown;
mod db;

pub use db::*;

use services::{proposals::PropInit};
pub use shutdown::*;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

use std::{future::Future, collections::HashSet};

use chrono::NaiveDate;
use scraper::{
    proposals::{Proposal, ProposalScraper},
    AustriaScraper, ProposalPayload,
};
use tokio::sync::{broadcast, mpsc};

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
    // tokio::join!(services::proposals::run(),);

    tokio::spawn(async move {
        run(tokio::signal::ctrl_c()).await;
    }).await.unwrap();
}

pub async fn run_services(state: &State) {
    let hs = HashSet::new();
    let res = services::delegates::delegates_once(&hs).await;
    println!("res: {res:?}");
    tokio::join!(
        // services::proposals::run(),
        services::proposals::run_shut(Shutdown::new(state.notify_shutdown.subscribe()))
    );
}

pub struct State {
    notify_shutdown: broadcast::Sender<()>,
    shutdown_complete_rx: mpsc::Receiver<()>,
    shutdown_complete_tx: mpsc::Sender<()>,
}

pub async fn run(shutdown: impl Future) {
    let (notify_shutdown, _) = broadcast::channel(1);
    let (shutdown_complete_tx, shutdown_complete_rx) = mpsc::channel(1);

    let state = State {
        notify_shutdown,
        shutdown_complete_rx,
        shutdown_complete_tx,
    };

    tokio::select! {
        _ = shutdown => {
            println!("Shutdown signal received");
        }
        _ = run_services(&state) => {
            println!("Services finished");
        }
    }

    let State {
        notify_shutdown,
        mut shutdown_complete_rx,
        shutdown_complete_tx,
    } = state;

    drop(notify_shutdown);
    drop(shutdown_complete_tx);

    let _ = shutdown_complete_rx.recv().await;
}
