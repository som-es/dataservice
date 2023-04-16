mod db;
mod services;
mod shutdown;

pub use db::*;

pub use shutdown::*;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

use std::{collections::HashSet, future::Future};

use chrono::NaiveDate;

use tokio::sync::{broadcast, mpsc};

#[inline]
pub fn today() -> NaiveDate {
    chrono::Local::now().date_naive()
}

#[tokio::main]
async fn main() {
    simple_logger::init_with_level(log::Level::Info).unwrap();
    // tokio::join!(services::proposals::run(),);

    tokio::spawn(async move {
        run(tokio::signal::ctrl_c()).await;
    })
    .await
    .unwrap();
}

pub async fn run_services(state: &State) {
    // let hs = HashSet::new();
    // let res = services::delegates::delegates_once(&hs).await;
    // println!("res: {res:?}");
    tokio::join!(
        // services::proposals::run(),
        services::legislative_initiatives::run_shut(Shutdown::new(state.notify_shutdown.subscribe())),
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
