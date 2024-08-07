use dataservice::{connect_pg, delegates::delegate_count, services, Shutdown};
use sqlx::PgPool;
use std::future::Future;
use tokio::sync::{broadcast, mpsc};

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
    {
        let pool = connect_pg().await;
        if delegate_count(&pool).await == Some(0) {
            log::info!("Delegates are 0, start scraping ...");
            let hs = std::collections::HashSet::new();
            let _res = services::delegates::delegates_once(&hs).await;
        }
    }

    tokio::join!(
        // services::proposals::run(),
        tokio::spawn(services::legislative_initiatives::run_shut_every_gp(
            Shutdown::new(state.notify_shutdown.subscribe())
        )),
        tokio::spawn(services::legislative_initiatives::run_shut_latest_gp(
            Shutdown::new(state.notify_shutdown.subscribe())
        )),
        tokio::spawn(services::ministrial_proposals::run_shut_every_gp(
            Shutdown::new(state.notify_shutdown.subscribe())
        )),
        tokio::spawn(services::ministrial_proposals::run_shut_latest_gp(
            Shutdown::new(state.notify_shutdown.subscribe())
        )),
        tokio::spawn(services::proposals::run_shut(Shutdown::new(
            state.notify_shutdown.subscribe()
        ))),
        tokio::spawn(services::proposals::run_shut_last_5_days(Shutdown::new(
            state.notify_shutdown.subscribe()
        ))),
        tokio::spawn(services::plenar::run_shut(Shutdown::new(
            state.notify_shutdown.subscribe()
        ))),
        tokio::spawn(services::delegates::run_shut(Shutdown::new(
            state.notify_shutdown.subscribe()
        ))),
        tokio::spawn(services::parties::run_shut(Shutdown::new(
            state.notify_shutdown.subscribe()
        ))),
    )
    .0
    .unwrap();
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
