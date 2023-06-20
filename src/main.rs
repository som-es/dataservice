use dataservice::{services, Shutdown};
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
    // only run when delegate db is empty!
    // let hs = std::collections::HashSet::new();
    // let _res = services::delegates::delegates_once(&hs).await;
    // println!("res: {_res:?}");
    tokio::join!(
        // services::proposals::run(),
        services::legislative_initiatives::run_shut(Shutdown::new(
            state.notify_shutdown.subscribe()
        )),
        services::proposals::run_shut(Shutdown::new(state.notify_shutdown.subscribe())),
        services::plenar::run_shut(Shutdown::new(state.notify_shutdown.subscribe())),
        services::delegates::run_shut(Shutdown::new(state.notify_shutdown.subscribe())),
        services::parties::run_shut(Shutdown::new(state.notify_shutdown.subscribe())),
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
