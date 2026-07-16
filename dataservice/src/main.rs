use dataservice::{
    ApiLimiter, AppState, DATABASE_URL, REDIS_URL, Shutdown, ai_task::AiTask, connect_pg,
    delegates::delegate_count, services, setup_app_data,
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

    /// Runs only voteables for vote results
    #[arg(long, default_value_t = false)]
    only_voteable: bool,
    #[arg(long, default_value_t = log::Level::Error)]
    log_level: log::Level,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 32)]
async fn main() {
    let _ = dotenvy::dotenv().ok();
    let args = Args::parse();

    simple_logger::init_with_level(args.log_level).unwrap();

    if let Err(e) = setup_app_data().await {
        log::error!("Cannot create app data directory: {e:?}");
    }

    #[cfg(not(feature = "tesseract"))]
    if let Err(e) = combx::download_ocr_models(&dataservice::model_dir()).await {
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

#[cfg(not(feature = "eu"))]
pub async fn run_eu_services(notify_shutdown: tokio::sync::broadcast::Sender<()>, _config: Config) {
}

#[cfg(feature = "eu")]
pub async fn run_eu_services(notify_shutdown: tokio::sync::broadcast::Sender<()>, config: Config) {
    use dataservice::eu_database_url;

    let (llm_ai_task_tx, llm_ai_task_rx, cpu_ai_task_rx, pool, app_state) =
        setup(&eu_database_url(), 1).await;
    {
        if delegate_count(&pool).await == Some(0) {
            use dataservice::parties::PartyService;

            log::info!("Delegates are 0, start scraping ...");
            let hs = std::collections::HashSet::new();
            let _res = services::delegates::delegates_once::<
                somes_scraper::eu_parliament::EuParliamentScraper,
            >(&hs, app_state.clone())
            .await;

            <somes_scraper::eu_parliament::EuParliamentScraper as PartyService>::run(
                &pool,
                app_state.parliament_api_limiter.clone(),
            )
            .await;
        }
    }

    tokio::join! {
        tokio::spawn(
            services::delegates::run_shut::<somes_scraper::eu_parliament::EuParliamentScraper>(
                Shutdown::new(notify_shutdown.subscribe()),
                app_state.clone()
            )
        ),
        tokio::spawn(
            services::parties::run_shut::<somes_scraper::eu_parliament::EuParliamentScraper>(
                Shutdown::new(notify_shutdown.subscribe()),
                pool.clone(),
                app_state.parliament_api_limiter.clone()
            )
        ),
    }
    .0
    .unwrap()
}

#[cfg(not(feature = "austria"))]
pub async fn run_at_services(notify_shutdown: tokio::sync::broadcast::Sender<()>, _config: Config) {
}

#[cfg(feature = "austria")]
pub async fn run_at_services(notify_shutdown: tokio::sync::broadcast::Sender<()>, config: Config) {
    use dataservice::{api_max_concurrent_tasks_from_env, at_database_url};

    let (llm_ai_task_tx, llm_ai_task_rx, cpu_ai_task_rx, pool, app_state) =
        setup(&at_database_url(), api_max_concurrent_tasks_from_env()).await;

    {
        if delegate_count(&pool).await == Some(0) {
            log::info!("Delegates are 0, start scraping ...");
            let hs = std::collections::HashSet::new();
            let _res = services::delegates::delegates_once::<somes_scraper::AustriaScraper>(
                &hs,
                app_state.clone(),
            )
            .await;
            services::parties::parties_once(&pool, app_state.parliament_api_limiter.clone()).await;

            services::plenar::plenar_info_once(
                &pool,
                app_state.parliament_api_limiter.clone(),
                Some(llm_ai_task_tx.clone()),
                false,
            )
            .await;
            services::legislative_initiatives::legis_init_once(
                false,
                true,
                &app_state,
                llm_ai_task_tx.clone(),
            )
            .await;

            let app_state1 = app_state.clone();

            let inner_ai_task_tx = llm_ai_task_tx.clone();
            let ministerial_proposals = async move {
                services::ministrial_proposals::ministrial_proposal_once(
                    false,
                    &app_state1,
                    inner_ai_task_tx.clone(),
                )
                .await
            };
            let app_state1 = app_state.clone();
            let inner_ai_task_tx = llm_ai_task_tx.clone();
            let decrees = async move {
                services::ministrial_decrees::save_ministrial_decrees(
                    decree_pages() as u32,
                    app_state1,
                    inner_ai_task_tx,
                    true,
                )
                .await;
            };
            let pool1 = pool.clone();
            let parliament_api_limiter = app_state.parliament_api_limiter.clone();
            let dates = async move {
                services::dates::save_dates(&pool1, parliament_api_limiter, false).await
            };
            tokio::join!(
                tokio::spawn(ministerial_proposals),
                tokio::spawn(decrees),
                tokio::spawn(dates)
            )
            .0
            .unwrap();
        }
        if config.force_fill {
            let app_state1 = app_state.clone();
            let inner_ai_task_tx = llm_ai_task_tx.clone();
            let legis_inits = async move {
                services::legislative_initiatives::legis_init_once(
                    true,
                    false,
                    &app_state1,
                    inner_ai_task_tx.clone(),
                )
                .await;
            };

            let pool1 = pool.clone();
            let parliament_api_limiter = app_state.parliament_api_limiter.clone();
            let inner_ai_task_tx = llm_ai_task_tx.clone();
            let plenars = async move {
                services::plenar::plenar_info_once(
                    &pool1,
                    parliament_api_limiter,
                    Some(inner_ai_task_tx),
                    true,
                )
                .await;
            };

            let inner_ai_task_tx = llm_ai_task_tx.clone();
            let app_state1 = app_state.clone();
            let ministerial_proposals = async move {
                services::ministrial_proposals::ministrial_proposal_once(
                    false,
                    &app_state1,
                    inner_ai_task_tx.clone(),
                )
                .await;
            };
            let inner_ai_task_tx = llm_ai_task_tx.clone();
            let app_state1 = app_state.clone();
            let decrees = async move {
                services::ministrial_decrees::save_ministrial_decrees(
                    decree_pages() as u32,
                    app_state1,
                    inner_ai_task_tx,
                    false,
                )
                .await;
            };
            let pool1 = pool.clone();
            let parliament_api_limiter = app_state.parliament_api_limiter.clone();
            let dates = async move {
                let _ = services::dates::save_dates(&pool1, parliament_api_limiter, false).await;
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
            Shutdown::new(notify_shutdown.subscribe()),
            app_state.clone(),
            llm_ai_task_tx.clone(),
            config.only_latest,
        )),
        tokio::spawn(services::legislative_initiatives::run_shut_latest_gp(
            Shutdown::new(notify_shutdown.subscribe()),
            app_state.clone(),
            llm_ai_task_tx.clone(),
            latest_legis_init_sleep + 5000,
            ONLY_LATEST,
            false, // also fetches everything that isn't a law
        )),
        tokio::spawn(services::legislative_initiatives::run_shut_latest_gp(
            Shutdown::new(notify_shutdown.subscribe()),
            app_state.clone(),
            llm_ai_task_tx.clone(),
            latest_legis_init_sleep + 60,
            ONLY_LATEST,
            FETCH_ONLY_LAWS,
        )),
        tokio::spawn(services::ministrial_proposals::run_shut_every_gp(
            Shutdown::new(notify_shutdown.subscribe()),
            app_state.clone(),
            config.only_latest,
            llm_ai_task_tx.clone(),
        )),
        tokio::spawn(services::ministrial_proposals::run_shut_latest_gp(
            Shutdown::new(notify_shutdown.subscribe()),
            app_state.clone(),
            llm_ai_task_tx.clone(),
        )),
        tokio::spawn(services::plenar::run_shut(
            Shutdown::new(notify_shutdown.subscribe()),
            pool.clone(),
            app_state.parliament_api_limiter.clone(),
            llm_ai_task_tx.clone(),
            config.only_latest
        )),
        tokio::spawn(services::plenar::run_shut_latest(
            Shutdown::new(notify_shutdown.subscribe()),
            pool.clone(),
            app_state.parliament_api_limiter.clone(),
            llm_ai_task_tx.clone()
        )),
        tokio::spawn(
            services::delegates::run_shut::<somes_scraper::AustriaScraper>(
                Shutdown::new(notify_shutdown.subscribe()),
                app_state.clone()
            )
        ),
        tokio::spawn(
            services::parties::run_shut::<somes_scraper::AustriaScraper>(
                Shutdown::new(notify_shutdown.subscribe()),
                pool.clone(),
                app_state.parliament_api_limiter.clone()
            )
        ),
        // latest
        tokio::spawn(services::ministrial_decrees::run_shut_pages(
            Shutdown::new(notify_shutdown.subscribe()),
            app_state.clone(),
            decree_pages() as u32,
            llm_ai_task_tx.clone(),
            true
        )),
        // latest
        tokio::spawn(services::ministrial_decrees::run_shut_pages(
            Shutdown::new(notify_shutdown.subscribe()),
            app_state.clone(),
            decree_pages() as u32,
            llm_ai_task_tx.clone(),
            false
        )),
        tokio::spawn(services::dates::run_shut_dates(
            Shutdown::new(notify_shutdown.subscribe()),
            pool.clone(),
            app_state.parliament_api_limiter.clone()
        )),
        tokio::spawn(services::dates::run_shut_dates_latest(
            Shutdown::new(notify_shutdown.subscribe()),
            pool.clone(),
            app_state.parliament_api_limiter.clone()
        )),
        tokio::spawn(services::ai_task_service::ai_tasks_service(
            Shutdown::new(notify_shutdown.subscribe()),
            pool.clone(),
            llm_ai_task_rx,
            1,
        )),
        tokio::spawn(services::ai_task_service::ai_tasks_service(
            Shutdown::new(notify_shutdown.subscribe()),
            pool.clone(),
            cpu_ai_task_rx,
            16,
        )),
        tokio::spawn(services::political_analysis_service(
            Shutdown::new(notify_shutdown.subscribe()),
            pool.clone(),
            llm_ai_task_tx.clone(),
        )),
        tokio::spawn(services::parliamentary_qa_service(
            Shutdown::new(notify_shutdown.subscribe()),
            app_state.clone(),
            llm_ai_task_tx.clone(),
            true,
        )),
        tokio::spawn(services::parliamentary_qa_service(
            Shutdown::new(notify_shutdown.subscribe()),
            app_state.clone(),
            llm_ai_task_tx.clone(),
            false,
        ))
    )
    .0
    .unwrap();
}

async fn setup(
    database_url: &str,
    max_api_fetch_tasks: usize,
) -> (
    mpsc::Sender<AiTask>,
    mpsc::Receiver<AiTask>,
    mpsc::Receiver<AiTask>,
    sqlx::Pool<sqlx::Postgres>,
    AppState,
) {
    let (llm_ai_task_tx, llm_ai_task_rx) = tokio::sync::mpsc::channel::<AiTask>(16536 * 22);
    let (cpu_ai_task_tx, cpu_ai_task_rx) = tokio::sync::mpsc::channel::<AiTask>(16536 * 22);

    let db_path = database_url.split("@").last().unwrap_or_default();
    log::info!("Connecting to database {db_path}",);
    let pool = connect_pg(database_url).await;
    let redis_client = redis::Client::open(REDIS_URL);

    match &redis_client {
        Ok(client) => {
            if client.get_connection().is_err() {
                log::error!("Could not establish redis database connection!");
            }
        }
        Err(e) => {
            log::error!("Could not create redis client: {e}");
        }
    }
    let redis_client = redis_client;
    let redis_client = match redis_client {
        Ok(redis_client) => redis_client.get_connection_manager().await,
        Err(e) => Err(e),
    }
    .ok();
    let app_state = AppState {
        pg_pool: pool.clone(),
        redis_client,
        parliament_api_limiter: ApiLimiter::new(max_api_fetch_tasks),
        cpu_task_tx: Some(cpu_ai_task_tx.clone()),
    };
    (
        llm_ai_task_tx,
        llm_ai_task_rx,
        cpu_ai_task_rx,
        pool,
        app_state,
    )
}

pub struct State {
    notify_shutdown: broadcast::Sender<()>,
    shutdown_complete_rx: mpsc::Receiver<()>,
    shutdown_complete_tx: mpsc::Sender<()>,
}

#[derive(Debug, Clone, Copy)]
pub struct Config {
    force_fill: bool,
    exit_after_force_fill: bool,
    only_latest: bool,
}

pub async fn run_services(notify_shutdown: tokio::sync::broadcast::Sender<()>, config: Config) {
    tokio::join! {
        tokio::spawn(run_at_services(notify_shutdown.clone(), config)),
        tokio::spawn(run_eu_services(notify_shutdown.clone(), config))
    }
    .0
    .unwrap();
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
        _ = run_services(state.notify_shutdown.clone(), config) => {
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
