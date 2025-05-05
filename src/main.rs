use dataservice::{
    connect_pg,
    delegates::{create_gov_officials_map, delegate_count},
    services, Shutdown, DATABASE_URL,
};
use std::future::Future;
use tokio::sync::{broadcast, mpsc};

#[tokio::main(flavor = "multi_thread", worker_threads = 1024)]
async fn main() {
    simple_logger::init_with_level(log::Level::Info).unwrap();
    // let _ = tokio::task::spawn_blocking(|| {
    //     let _x = &*GOV_OFFICIALS;
    //     // log::info!("gov of map: {x:?}");
    // })
    // .await;
    // tokio::join!(services::proposals::run(),);

    tokio::spawn(async move {
        run(tokio::signal::ctrl_c()).await;
    })
    .await
    .unwrap();
}

pub async fn run_services(state: &State) {
    // let pool = connect_pg().await;

    let pool = connect_pg().await;
    {
        if delegate_count(&pool).await == Some(0) {
            log::info!("Delegates are 0, start scraping ...");
            let hs = std::collections::HashSet::new();
            let _res = services::delegates::delegates_once(&hs, pool.clone()).await;
            services::parties::parties_once(&pool).await;
        }
    }

    let (new_vote_result_tx, new_vote_result_rx) =
        tokio::sync::mpsc::channel::<Option<(i32, somes_scraper::LegislativeInitiative)>>(128);

    let (new_ministrial_proposal_tx, new_ministrial_proposal_rx) = tokio::sync::mpsc::channel::<
        Option<(i32, somes_scraper::ministrial_proposals::MinistrialProposal)>,
    >(128);

    tokio::join!(
        // services::proposals::run(),
        tokio::spawn(services::legislative_initiatives::run_shut_every_gp(
            Shutdown::new(state.notify_shutdown.subscribe()),
            pool.clone()
        )),
        tokio::spawn(services::legislative_initiatives::run_shut_latest_gp(
            Shutdown::new(state.notify_shutdown.subscribe()),
            pool.clone(),
            new_vote_result_tx.clone()
        )),
        tokio::spawn(services::ministrial_proposals::run_shut_every_gp(
            Shutdown::new(state.notify_shutdown.subscribe()),
            pool.clone(),
        )),
        tokio::spawn(services::ministrial_proposals::run_shut_latest_gp(
            Shutdown::new(state.notify_shutdown.subscribe()),
            pool.clone(),
            new_ministrial_proposal_tx
        )),
        tokio::spawn(services::proposals::run_shut(
            Shutdown::new(state.notify_shutdown.subscribe()),
            pool.clone()
        )),
        tokio::spawn(services::proposals::run_shut_last_5_days(
            Shutdown::new(state.notify_shutdown.subscribe()),
            pool.clone()
        )),
        tokio::spawn(services::plenar::run_shut(
            Shutdown::new(state.notify_shutdown.subscribe()),
            pool.clone()
        )),
        tokio::spawn(services::plenar::run_shut_latest(
            Shutdown::new(state.notify_shutdown.subscribe()),
            pool.clone()
        )),
        tokio::spawn(services::delegates::run_shut(
            Shutdown::new(state.notify_shutdown.subscribe()),
            pool.clone()
        )),
        tokio::spawn(services::parties::run_shut(
            Shutdown::new(state.notify_shutdown.subscribe()),
            pool.clone()
        )),
        tokio::spawn(services::ministrial_decrees::run_shut_pages(
            Shutdown::new(state.notify_shutdown.subscribe()),
            pool.clone(),
            100
        )),
        tokio::spawn(
            services::new_vote_results_mailer::notify_on_new_vote_results_service(
                Shutdown::new(state.notify_shutdown.subscribe()),
                new_vote_result_rx,
                pool.clone()
            )
        ),
        tokio::spawn(
            services::new_ministrial_prop_mailer::notify_on_new_ministrial_props_service(
                Shutdown::new(state.notify_shutdown.subscribe()),
                new_ministrial_proposal_rx,
                pool.clone()
            )
        ),
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
