use combx::GovProposal;
use dataservice::{
    ai_task::AiTask, connect_pg, delegates::delegate_count, services, Shutdown, DATABASE_URL,
};
use std::future::Future;
use tokio::sync::{broadcast, mpsc};

#[tokio::main(flavor = "multi_thread", worker_threads = 1024)]
async fn main() {
    simple_logger::init_with_level(log::Level::Error).unwrap();
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

pub fn decree_pages() -> usize {
    std::env::var("DECREE_PAGES")
        .unwrap_or_else(|_| "150".to_string())
        .parse()
        .unwrap_or_else(|e| {
            log::warn!(
                "Could not parse DECREE_PAGES env var: {e}. Using default value of 150 pages"
            );
            150
        })
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

    let (new_ministrial_proposal_tx, new_ministrial_proposal_rx) =
        tokio::sync::mpsc::channel::<Option<(i32, GovProposal)>>(128);

    let (ai_task_tx, ai_task_rx) = tokio::sync::mpsc::channel::<AiTask>(16536 * 22);

    let latest_legis_init_sleep = std::env::var("LEGISLATIVE_INITIATIVE_SLEEP_LATEST")
        .unwrap_or_else(|_| "300".to_string())
        .parse()
        .unwrap_or_else(|e| {
            log::warn!(
                "Could not parse LEGISLATIVE_INITIATIVE_SLEEP_LATEST env var: {e}. Using default value of 300 seconds"
            );
            300
        });

    tokio::join!(
        // services::proposals::run(),
        tokio::spawn(services::legislative_initiatives::run_shut_every_gp(
            Shutdown::new(state.notify_shutdown.subscribe()),
            pool.clone(),
            ai_task_tx.clone(),
        )),
        /*tokio::spawn(services::legislative_initiatives::run_shut_latest_gp(
            Shutdown::new(state.notify_shutdown.subscribe()),
            pool.clone(),
            new_vote_result_tx.clone(),
            ai_task_tx.clone(),
            latest_legis_init_sleep + 150,
            true,
            false,
        )),*/
        tokio::spawn(services::legislative_initiatives::run_shut_latest_gp(
            Shutdown::new(state.notify_shutdown.subscribe()),
            pool.clone(),
            new_vote_result_tx.clone(),
            ai_task_tx.clone(),
            latest_legis_init_sleep - 40,
            true,
            true,
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
            decree_pages() as u32
        )),
        tokio::spawn(services::mailer::notify_on_new_vote_results_service(
            Shutdown::new(state.notify_shutdown.subscribe()),
            new_vote_result_rx,
            pool.clone()
        )),
        tokio::spawn(services::mailer::notify_on_new_ministrial_props_service(
            Shutdown::new(state.notify_shutdown.subscribe()),
            new_ministrial_proposal_rx,
            pool.clone()
        )),
        tokio::spawn(services::delete_was_updated::delete_was_updated_entries(
            Shutdown::new(state.notify_shutdown.subscribe()),
            pool.clone()
        )),
        tokio::spawn(services::dates::run_shut_dates(
            Shutdown::new(state.notify_shutdown.subscribe()),
            pool.clone()
        )),
        tokio::spawn(services::dates::run_shut_dates_latest(
            Shutdown::new(state.notify_shutdown.subscribe()),
            pool.clone()
        )),
        tokio::spawn(services::ai_task_service::ai_tasks_service(
            Shutdown::new(state.notify_shutdown.subscribe()),
            pool.clone(),
            ai_task_rx,
        )),
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
