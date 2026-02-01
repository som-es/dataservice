use combx::GovProposal;
use dataservice::{
    ai_task::{self, AiTask},
    connect_pg,
    delegates::delegate_count,
    model_dir, services, setup_app_data, Shutdown,
};
use std::future::Future;
use tokio::sync::{broadcast, mpsc};

use clap::Parser;

/// Somes-Dataservice
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Force insert and update all build-time-configured data
    #[arg(short, long, default_value_t = false)]
    force_fill: bool,

    /// Exit after force fill was completed
    #[arg(long, default_value_t = false)]
    exit_after_force_fill: bool,

    /// Runs only latest services
    #[arg(long, default_value_t = false)]
    only_latest: bool,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 1024)]
async fn main() {
    let args = Args::parse();

    simple_logger::init_with_level(log::Level::Info).unwrap();

    if let Err(e) = setup_app_data().await {
        log::error!("Cannot create app data directory: {e:?}");
    }

    #[cfg(not(feature = "tesseract"))]
    if let Err(e) = combx::download_ocr_models(&model_dir()).await {
        log::error!("Cannot download ocr models: {e:?}");
    }

    tokio::spawn(async move {
        run(
            tokio::signal::ctrl_c(),
            Config {
                force_fill: args.force_fill,
                exit_after_force_fill: args.exit_after_force_fill,
                only_latest: args.only_latest,
            },
        )
        .await;
    })
    .await
    .unwrap();
}

pub const ONLY_LATEST: bool = true;
pub const FETCH_ONLY_LAWS: bool = true;

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

pub async fn run_services(state: &State, config: Config) {
    let (ai_task_tx, ai_task_rx) = tokio::sync::mpsc::channel::<AiTask>(16536 * 22);

    let pool = connect_pg().await;
    {
        if delegate_count(&pool).await == Some(0) {
            log::info!("Delegates are 0, start scraping ...");
            let hs = std::collections::HashSet::new();
            let _res = services::delegates::delegates_once(&hs, pool.clone()).await;
            services::parties::parties_once(&pool).await;

            services::plenar::plenar_info_once(&pool, false).await;
            services::legislative_initiatives::legis_init_once(
                false,
                true,
                &pool,
                None,
                ai_task_tx.clone(),
            )
            .await;

            let pool1 = pool.clone();

            let inner_ai_task_tx = ai_task_tx.clone();
            let ministerial_proposals = async move {
                services::ministrial_proposals::ministrial_proposal_once(
                    false,
                    &pool1,
                    None,
                    inner_ai_task_tx.clone(),
                )
                .await
            };
            let pool1 = pool.clone();
            let decrees = async move {
                services::ministrial_decrees::save_ministrial_decrees(
                    decree_pages() as u32,
                    &pool1,
                )
                .await;
            };
            let pool1 = pool.clone();
            let dates = async move { services::dates::save_dates(&pool1, false).await };
            tokio::join!(
                tokio::spawn(ministerial_proposals),
                tokio::spawn(decrees),
                tokio::spawn(dates)
            )
            .0
            .unwrap();
        }
        if config.force_fill {
            let pool1 = pool.clone();
            let inner_ai_task_tx = ai_task_tx.clone();
            let legis_inits = async move {
                services::legislative_initiatives::legis_init_once(
                    true,
                    false,
                    &pool1,
                    None,
                    inner_ai_task_tx.clone(),
                )
                .await;
            };

            let pool1 = pool.clone();
            let plenars = async move {
                services::plenar::plenar_info_once(&pool1, true).await;
            };

            let ai_task_tx = ai_task_tx.clone();
            let pool1 = pool.clone();
            let ministerial_proposals = async move {
                services::ministrial_proposals::ministrial_proposal_once(
                    false,
                    &pool1,
                    None,
                    ai_task_tx.clone(),
                )
                .await;
            };
            let pool1 = pool.clone();
            let decrees = async move {
                services::ministrial_decrees::save_ministrial_decrees(
                    decree_pages() as u32,
                    &pool1,
                )
                .await;
            };
            let pool1 = pool.clone();
            let dates = async move {
                let _ = services::dates::save_dates(&pool1, false).await;
            };

            let handles = [
                tokio::spawn(plenars),
                tokio::spawn(legis_inits),
                tokio::spawn(ministerial_proposals),
                tokio::spawn(decrees),
                tokio::spawn(dates),
            ];

            if !config.only_latest || config.exit_after_force_fill {
                for handle in handles {
                    if let Err(e) = handle.await {
                        log::error!("Cannot await handle: {e:?}")
                    }
                }
                return;
            }
        }
    }

    let (new_vote_result_tx, new_vote_result_rx) =
        tokio::sync::mpsc::channel::<Option<(i32, somes_scraper::LegislativeInitiative)>>(128);

    let (new_ministrial_proposal_tx, new_ministrial_proposal_rx) =
        tokio::sync::mpsc::channel::<Option<(i32, GovProposal)>>(128);

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
            config.only_latest,
        )),
        tokio::spawn(services::legislative_initiatives::run_shut_latest_gp(
            Shutdown::new(state.notify_shutdown.subscribe()),
            pool.clone(),
            new_vote_result_tx.clone(),
            ai_task_tx.clone(),
            latest_legis_init_sleep + 500,
            ONLY_LATEST,
            false, // also fetches everything that isn't a law
        )),
        tokio::spawn(services::legislative_initiatives::run_shut_latest_gp(
            Shutdown::new(state.notify_shutdown.subscribe()),
            pool.clone(),
            new_vote_result_tx.clone(),
            ai_task_tx.clone(),
            latest_legis_init_sleep - 40,
            ONLY_LATEST,
            FETCH_ONLY_LAWS,
        )),
        tokio::spawn(services::ministrial_proposals::run_shut_every_gp(
            Shutdown::new(state.notify_shutdown.subscribe()),
            pool.clone(),
            config.only_latest,
            ai_task_tx.clone(),
        )),
        tokio::spawn(services::ministrial_proposals::run_shut_latest_gp(
            Shutdown::new(state.notify_shutdown.subscribe()),
            pool.clone(),
            new_ministrial_proposal_tx,
            ai_task_tx.clone(),
        )),
        tokio::spawn(services::plenar::run_shut(
            Shutdown::new(state.notify_shutdown.subscribe()),
            pool.clone(),
            config.only_latest
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

pub struct Config {
    force_fill: bool,
    exit_after_force_fill: bool,
    only_latest: bool,
}

pub async fn run(shutdown: impl Future, config: Config) {
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
        _ = run_services(&state, config) => {
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
