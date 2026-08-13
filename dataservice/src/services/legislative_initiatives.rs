use chrono::{Days, Utc};
use combx::{Parliament, VoteResult};
use tokio::sync::mpsc::Sender;
mod db;
pub mod notify;
pub use db::*;

mod inserts;
mod scraping;
use scraping::*;

mod ai_tasks;
pub use ai_tasks::*;

use somes_scraper::{
    AustriaScraper, LegislativeInitiative,
    eu_parliament::EuParliamentScraper,
    legis_init::{LEGIS_IS_PLENARY_VOTEABLE_ON_DOKTYPES, LegislativeInitiatives},
};
use sqlx::{PgPool, query, query_scalar};
use std::sync::{Arc, LazyLock};

use crate::{
    AppState, CompareHash, GPS, Gps, LATEST_GP, Shutdown, ai_task::AiTask,
    should_update::StoredUpdatedAt, today_and_time,
};

pub(self) const LEGIS_INIT_STILL_SEND_DAYS: LazyLock<i64> = LazyLock::new(|| {
    std::env::var("LEGIS_INIT_STILL_SEND_DAYS")
        .unwrap_or("10".to_string())
        .parse::<i64>()
        .unwrap_or(10)
});

pub trait VoteResultService:
    LegislativeInitiatives + Send + Sync + 'static + Gps + Default
{
    const RUNS_PARALLEL: bool;
}

impl VoteResultService for AustriaScraper {
    const RUNS_PARALLEL: bool = true;
}

impl VoteResultService for EuParliamentScraper {
    const RUNS_PARALLEL: bool = false;
}

pub async fn legis_init_once<S: VoteResultService>(
    only_latest: bool,
    only_voteable: bool,
    pg_pool: &AppState,
    ai_task_tx: Sender<AiTask>,
) -> Vec<LegislativeInitiative> {
    log::info!("Try scraping legislative initiatives");

    let mut handles = vec![];
    for gp in S::gps() {
        if only_latest && gp != S::LATEST_GP {
            continue;
        }
        let pg_pool = pg_pool.clone();
        let ai_task_tx = ai_task_tx.clone();

        let handle = tokio::spawn(async move {
            let mut scraper = S::default();
            log::info!("LEGIS INIT once GP: {gp}");
            scraper.scrape_info_mut().gp = vec![gp.clone()];
            if only_latest {
                let days = if only_voteable {
                    // this number is a magic number -> maybe reason it with average completion time + n or something
                    // could still be possible that very old (+1001 days old) newly completed legis inits are never sent as email!
                    333
                } else {
                    100
                };
                scraper.scrape_info_mut().date_start = today_and_time()
                    .naive_local()
                    .checked_sub_days(Days::new(days));
            } else {
                scraper.scrape_info_mut().date_start = None;
            }

            let scraper = Arc::new(scraper);

            let searched_doktyps = if only_voteable {
                Some(LEGIS_IS_PLENARY_VOTEABLE_ON_DOKTYPES.as_slice())
            } else {
                None
            };

            if let Err(e) = scrape_new_legis_inits(
                &pg_pool,
                scraper.clone(),
                &gp,
                ai_task_tx,
                searched_doktyps,
                only_latest,
                only_voteable,
                S::PARLIAMENT,
                if S::RUNS_PARALLEL {
                    LEGIS_INIT_MAX_INSERT_TASKS
                } else {
                    1
                },
            )
            .await
            {
                log::error!("could not scrape and insert legis inits: {e}");
            }
            log::info!("finished scraping LEGIS INIT once GP: {gp}");
        });
        if S::RUNS_PARALLEL {
            handles.push(handle);
        } else {
            if let Err(e) = handle.await {
                log::error!("join error: {e:?}");
            }
        }
        if only_latest {
            break;
        }
    }

    for handle in handles {
        if let Err(e) = handle.await {
            log::error!("join error: {e:?}");
        }
    }
    // new_legis_inits
    vec![]
}

impl StoredUpdatedAt for LegislativeInitiative {
    async fn stored_updated_at(&self, pg: &PgPool) -> Option<chrono::DateTime<Utc>> {
        query!("select raw_data_updated_at from legislative_initiatives where gp = $1 and ityp = $2 and inr = $3", self.gp, self.ityp, self.inr as i32)
            .fetch_optional(pg)
            .await
            .ok()
            .flatten()
            .map(|x| x.raw_data_updated_at).flatten()
    }
}

impl CompareHash for VoteResult {
    async fn stored_hash(&self, pg: &PgPool) -> Option<u64> {
        query_scalar!(
            "select hash from legislative_initiatives where gp = $1 and ityp = $2 and inr = $3",
            self.legislative_initiative.gp,
            self.legislative_initiative.ityp,
            self.legislative_initiative.inr as i32
        )
        .fetch_one(pg)
        .await
        .ok()
        .flatten()
        .map(|x| x as u64)
    }
}

pub async fn run_shut_latest_gp<S: VoteResultService>(
    mut shutdown: Shutdown,
    app_state: AppState,
    ai_task_tx: Sender<AiTask>,
    legis_init_sleep: u64,
    only_latest: bool,
    only_laws: bool,
) {
    #[cfg(feature = "legis_init_latest")]
    while !shutdown.is_shutdown() {
        let ai_task_ts = ai_task_tx.clone();
        tokio::select! {
            _ = shutdown.recv() => {
                println!("shutting down");
                // return ok?
                break;
            }
            _ = async {
                let app_state = app_state.clone();
                if let Err(e) = tokio::task::spawn(async move {
                    log::info!("Starting legislative initiative (latest) scraper");

                    legis_init_once::<S>(only_latest, only_laws, &app_state, ai_task_ts.clone()).await;
                }).await {
                    log::error!("Error in legislative initiative (latest) scraper: {:?}", e);
                }


                // legis_init_once().await;
                log::info!("Legislative initiative (latest) scraper service sleeps for {legis_init_sleep} seconds...");
                tokio::time::sleep(tokio::time::Duration::from_secs(legis_init_sleep)).await;
            } => {}
        }
    }
}

pub async fn run_shut_every_gp<S: VoteResultService>(
    mut shutdown: Shutdown,
    app_state: AppState,
    ai_task_tx: Sender<AiTask>,
    only_latest: bool,
) {
    if only_latest {
        return;
    }
    let legis_init_sleep = std::env::var("LEGISLATIVE_INITIATIVE_SLEEP")
        .unwrap_or_else(|_| "80000".to_string())
        .parse()
        .unwrap_or_else(|e| {
            log::warn!(
                "Could not parse LEGISLATIVE_INITIATIVE_SLEEP env var: {e}. Using default value of 80000 seconds"
            );
            80000
        });

    #[cfg(feature = "legis_init")]
    while !shutdown.is_shutdown() {
        let ai_task_tx = ai_task_tx.clone();
        tokio::select! {
            _ = shutdown.recv() => {
                println!("shutting down");
                // return ok?
                break;
            }
            _ = async {
                let app_state = app_state.clone();
                if let Err(e) = tokio::task::spawn(async move {
                    log::info!("Starting legislative initiative scraper");

                    legis_init_once::<S>(false, false, &app_state, ai_task_tx.clone()).await;
                }).await {
                    log::error!("Error in legislative initiative scraper: {:?}", e);
                }
                // legis_init_once().await;
                log::info!("Legislative initiative scraper service sleeps for {legis_init_sleep} seconds...");
                tokio::time::sleep(tokio::time::Duration::from_secs(legis_init_sleep)).await;
            } => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        hash::{DefaultHasher, Hash, Hasher},
        sync::Arc,
    };

    use chrono::{NaiveDate, NaiveDateTime};
    use combx::{Parliament, PostgresEnhancer, VoteResult};
    use rand::rand_core::le;
    use somes_scraper::{
        AustriaScraper, LegislativeInitiative, legis_init::LegislativeInitiatives,
    };
    use sqlx::postgres::PgPoolOptions;

    use crate::{CompareHash, DATABASE_URL, connect_pg2 as connect_pg};

    #[test]
    fn test_date_time_parse() {
        let x = NaiveDateTime::parse_from_str("2014-5-17T12:34:56", "%Y-%m-%dT%H:%M:%S").unwrap();
        println!("x: {x:?}");
    }

    #[tokio::test]
    async fn test_legis_init_empty_gp_scrape() {
        let mut scraper: AustriaScraper = AustriaScraper::default();
        let res = scraper
            .get_legis_init("/gegenstand/XXVII/I/2269")
            .await
            .unwrap();

        let pool = connect_pg().await;
        // crate::legislative_initiatives::update_legis_inits_in_db(&pool, &[res], "XXVII").await;

        // println!("s: {:?}", scraper.scrape_info.gp);

        // let out = crate::legislative_initiatives::scrape_new_legis_inits(&scraper).await.unwrap();
        // println!("out: {out:?}");
    }

    #[tokio::test]
    async fn test_legis_init_empty_gp_scrape_xxvi_a_110() {
        // let mut scraper: AustriaScraper = AustriaScraper::default();
        // let res = scraper
        //     .get_legis_init("/gegenstand/XXVI/A/238")
        //     .await
        //     .unwrap();

        // let pool = pollster::block_on(connect_pg());
        let pool = PgPoolOptions::new()
            .max_connections(150)
            .connect(DATABASE_URL)
            .await
            .unwrap();
        println!("ppoll");

        // let date = "18.05.2016";
        let date = NaiveDate::from_ymd_opt(2013, 11, 18).unwrap();
        // let parties = gen_code_to_parties_with_date(&pool, &date, "XXV").await;
        // println!("{parties:?}");

        // crate::legislative_initiatives::update_legis_inits_in_db(&pool, &[res], "XXVI").await;

        // println!("s: {:?}", scraper.scrape_info.gp);

        // let out = crate::legislative_initiatives::scrape_new_legis_inits(&scraper).await.unwrap();
        // println!("out: {out:?}");
    }
    #[tokio::test]
    async fn test_legis_init_empty_gp_scrape_xxvii_a_3431() {
        let scraper: AustriaScraper = AustriaScraper::default();
        let res = scraper
            .get_legis_init("/gegenstand/XXVII/A/3431")
            .await
            .unwrap();

        let pool = connect_pg().await;
        let app_state = crate::AppState {
            pg_pool: pool,
            redis_client: None,
            parliament_api_limiter: crate::ApiLimiter::new(32),
            cpu_task_tx: None,
            parliament: combx::Parliament::At,
        };
        crate::legislative_initiatives::update_or_insert_legis_init_in_db(
            &app_state,
            &res,
            None,
            None,
            Parliament::At,
        )
        .await;
    }

    #[tokio::test]
    async fn test_legis_init_empty_gp_scrape_xxvii_a_3945() {
        let mut scraper: AustriaScraper = AustriaScraper::default();
        let res = scraper
            .get_legis_init("/gegenstand/XXVII/A/3945")
            .await
            .unwrap();

        let pool = connect_pg().await;
        let app_state = crate::AppState {
            pg_pool: pool,
            redis_client: None,
            parliament_api_limiter: crate::ApiLimiter::new(32),
            cpu_task_tx: None,
            parliament: combx::Parliament::At,
        };
        crate::legislative_initiatives::update_or_insert_legis_init_in_db(
            &app_state,
            &res,
            None,
            None,
            Parliament::At,
        )
        .await;

        // println!("s: {:?}", scraper.scrape_info.gp);

        // let out = crate::legislative_initiatives::scrape_new_legis_inits(&scraper).await.unwrap();
        // println!("out: {out:?}");
    }

    #[tokio::test]
    async fn test_legis_init_empty_gp_scrape_xxvii_i_2711() {
        let scraper: AustriaScraper = AustriaScraper::default();
        let res = scraper
            .get_legis_init("/gegenstand/XXVII/I/2711")
            .await
            .unwrap();

        let pool = connect_pg().await;
        let app_state = crate::AppState {
            pg_pool: pool,
            redis_client: None,
            parliament_api_limiter: crate::ApiLimiter::new(32),
            cpu_task_tx: None,
            parliament: combx::Parliament::At,
        };
        crate::legislative_initiatives::update_or_insert_legis_init_in_db(
            &app_state,
            &res,
            None,
            None,
            Parliament::At,
        )
        .await;
    }

    #[tokio::test]
    async fn test_legis_init_empty_gp_scrape_xxvii_a_4141() {
        let scraper: AustriaScraper = AustriaScraper::default();
        let res = scraper
            .get_legis_init("/gegenstand/XXVII/I/1858")
            .await
            .unwrap();

        let pool = connect_pg().await;
        let app_state = crate::AppState {
            pg_pool: pool,
            redis_client: None,
            parliament_api_limiter: crate::ApiLimiter::new(32),
            cpu_task_tx: None,
            parliament: combx::Parliament::At,
        };
        crate::legislative_initiatives::update_or_insert_legis_init_in_db(
            &app_state,
            &res,
            None,
            None,
            Parliament::At,
        )
        .await;
    }

    #[tokio::test]
    async fn test_legis_init_empty_gp_scrape_xxvii_iii_835() {
        let scraper: AustriaScraper = AustriaScraper::default();
        let res = scraper
            .get_legis_init("/gegenstand/XXVII/III/85")
            .await
            .unwrap();

        let pool = connect_pg().await;
        let app_state = crate::AppState {
            pg_pool: pool,
            redis_client: None,
            parliament_api_limiter: crate::ApiLimiter::new(32),
            cpu_task_tx: None,
            parliament: combx::Parliament::At,
        };
        crate::legislative_initiatives::update_or_insert_legis_init_in_db(
            &app_state,
            &res,
            None,
            None,
            Parliament::At,
        )
        .await;
    }

    #[tokio::test]
    async fn test_legis_init_empty_gp_scrape_xxviii_uea_8() {
        simple_logger::init_with_level(log::Level::Info).unwrap();
        let scraper: AustriaScraper = AustriaScraper::default();
        let res = scraper
            .get_legis_init("/gegenstand/XXVIII/UEA/8")
            .await
            .unwrap();

        let pool = connect_pg().await;
        let app_state = crate::AppState {
            pg_pool: pool,
            redis_client: None,
            parliament_api_limiter: crate::ApiLimiter::new(32),
            cpu_task_tx: None,
            parliament: combx::Parliament::At,
        };
        crate::legislative_initiatives::update_or_insert_legis_init_in_db(
            &app_state,
            &res,
            None,
            None,
            Parliament::At,
        )
        .await;
    }

    #[tokio::test]
    async fn test_legis_init_empty_gp_scrape_xxviii_a_104() {
        simple_logger::init_with_level(log::Level::Info).unwrap();
        let scraper: AustriaScraper = AustriaScraper::default();
        let res = scraper
            .get_legis_init("/gegenstand/XXVIII/A/104")
            .await
            .unwrap();

        let pool = connect_pg().await;
        let app_state = crate::AppState {
            pg_pool: pool,
            redis_client: None,
            parliament_api_limiter: crate::ApiLimiter::new(32),
            cpu_task_tx: None,
            parliament: combx::Parliament::At,
        };
        crate::legislative_initiatives::update_or_insert_legis_init_in_db(
            &app_state,
            &res,
            None,
            None,
            Parliament::At,
        )
        .await;
    }

    #[tokio::test]
    async fn test_legis_init_empty_gp_scrape_xxviii_a_64() {
        simple_logger::init_with_level(log::Level::Info).unwrap();
        let scraper: AustriaScraper = AustriaScraper::default();
        let res = scraper
            .get_legis_init("/gegenstand/XXVIII/A/64")
            .await
            .unwrap();

        log::info!("legis init: {res:?}");

        let pool = connect_pg().await;
        let app_state = crate::AppState {
            pg_pool: pool,
            redis_client: None,
            parliament_api_limiter: crate::ApiLimiter::new(32),
            cpu_task_tx: None,
            parliament: combx::Parliament::At,
        };
        crate::legislative_initiatives::update_or_insert_legis_init_in_db(
            &app_state,
            &res,
            None,
            None,
            Parliament::At,
        )
        .await;
    }

    #[tokio::test]
    async fn test_legis_init_empty_gp_scrape_xxviii_a_77() {
        simple_logger::init_with_level(log::Level::Info).unwrap();
        let scraper: AustriaScraper = AustriaScraper::default();
        let res = scraper
            .get_legis_init("/gegenstand/XXVIII/A/77")
            .await
            .unwrap();

        log::info!("legis init: {res:?}");

        let pool = connect_pg().await;
        let app_state = crate::AppState {
            pg_pool: pool,
            redis_client: None,
            parliament_api_limiter: crate::ApiLimiter::new(32),
            cpu_task_tx: None,
            parliament: combx::Parliament::At,
        };
        crate::legislative_initiatives::update_or_insert_legis_init_in_db(
            &app_state,
            &res,
            None,
            None,
            Parliament::At,
        )
        .await;
    }

    #[tokio::test]
    async fn test_legis_init_empty_gp_scrape_xxviii_a_437() {
        simple_logger::init_with_level(log::Level::Info).unwrap();
        let scraper: AustriaScraper = AustriaScraper::default();
        let res = scraper
            .get_legis_init("/gegenstand/XXVIII/A/437")
            .await
            .unwrap();

        log::info!("legis init: {res:?}");

        let pool = connect_pg().await;
        let app_state = crate::AppState {
            pg_pool: pool,
            redis_client: None,
            parliament_api_limiter: crate::ApiLimiter::new(32),
            cpu_task_tx: None,
            parliament: combx::Parliament::At,
        };
        crate::legislative_initiatives::update_or_insert_legis_init_in_db(
            &app_state,
            &res,
            None,
            None,
            Parliament::At,
        )
        .await;
    }

    // check hashes of XXIII AA 95
    #[tokio::test]
    async fn test_compare_hashes() {
        let scraper = AustriaScraper::default();
        let mut hasher = DefaultHasher::new();
        let legis_init = scraper
            .get_legis_init("/gegenstand/XXIII/AA/95")
            .await
            .unwrap();
        legis_init.hash(&mut hasher);
        let hash = hasher.finish();
        let data = serde_json::to_string(&legis_init).unwrap();
        println!("data: {data}");
        dbg!(hash);

        // dbg!(&legis_init);

        let pool = connect_pg().await;
        let vote_result = VoteResult::from_legis_init(
            PostgresEnhancer(pool.clone()),
            legis_init,
            Parliament::At,
            None,
        )
        .await
        .unwrap();
        let data = serde_json::to_string(&vote_result).unwrap();
        println!("data: {data}");

        // 13891749994034791878
        dbg!(vote_result.hash_of());
        dbg!(vote_result.hash_of());
        dbg!(vote_result.hash_of());
        dbg!(vote_result.hash_of());

        // let (should_update, hash) = crate::should_update_by_hash(&vote_result, &pool).await;
        // assert!(!should_update, "Hashes should be the same, but they are not. Hash in db: {hash:?}");
    }
}
