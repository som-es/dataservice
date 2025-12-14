use chrono::Days;
use tokio::{runtime::Handle, sync::mpsc::Sender};
mod ai_tasks;
mod db;
pub mod notify;
pub use db::*;

mod inserts;
mod scraping;
use scraping::*;

use somes_scraper::{
    legis_init::LEGIS_IS_VOTEABLE_ON_DOKTYPES, AustriaScraper, LegislativeInitiative,
};
use sqlx::{query, PgPool};
use std::sync::{Arc, LazyLock};

use crate::{
    ai_task::AiTask, should_update::StoredUpdatedAt, today_and_time, Shutdown, GPS, LATEST_GP,
};

use dotenvy_macro::dotenv;

pub(self) const LEGIS_INIT_STILL_SEND_DAYS: LazyLock<i64> = LazyLock::new(|| {
    dotenv!("LEGIS_INIT_STILL_SEND_DAYS")
        .parse::<i64>()
        .unwrap_or(10)
});

async fn legis_init_once(
    only_latest: bool,
    only_laws: bool,
    pg_pool: &PgPool,
    new_legis_init: Option<Sender<Option<(i32, LegislativeInitiative)>>>,
    ai_task_ts: Sender<AiTask>,
) -> Vec<LegislativeInitiative> {
    log::info!("Try scraping legislative initiatives");

    let mut handles = vec![];
    for gp in GPS {
        if only_latest && gp != LATEST_GP {
            continue;
        }
        let pg_pool = pg_pool.clone();
        let new_legis_init = new_legis_init.clone();
        let ai_task_ts = ai_task_ts.clone();
        let handle = tokio::task::spawn_blocking(move || {
            Handle::current().block_on(async move {
                let mut scraper = AustriaScraper::default();
                log::info!("LEGIS INIT once GP: {gp}");
                scraper.scrape_info.gp = vec![gp.into()];
                if only_latest {
                    let days = if only_laws {
                        // this number is a magic number -> maybe reason it with average completion time + n or something
                        // could still be possible that very old (+1001 days old) newly completed legis inits are never sent as email!
                        333
                    } else {
                        100
                    };
                    scraper.scrape_info.date_start =
                        today_and_time().checked_sub_days(Days::new(days));
                } else {
                    scraper.scrape_info.date_start = None;
                }

                let scraper = Arc::new(scraper);

                let searched_doktyps = if only_laws {
                    Some(LEGIS_IS_VOTEABLE_ON_DOKTYPES.as_slice())
                } else {
                    None
                };

                if let Err(e) = scrape_new_legis_inits(
                    &pg_pool,
                    scraper.clone(),
                    gp,
                    new_legis_init,
                    ai_task_ts,
                    searched_doktyps,
                )
                .await
                {
                    log::error!("could not scrape and insert legis inits: {e}");
                }
                log::info!("finished scraping LEGIS INIT once GP: {gp}");
            })
        });
        handles.push(handle);
        if only_latest {
            break;
        }
    }

    for handle in handles {
        if let Err(e) = handle.await {
            if e.is_panic() {
                log::error!("join error: {e:?}");
            }
        }
    }
    // new_legis_inits
    vec![]
}

impl StoredUpdatedAt for LegislativeInitiative {
    async fn stored_updated_at(&self, pg: &PgPool) -> Option<chrono::NaiveDateTime> {
        query!("select updated_at from legislative_initiatives where gp = $1 and ityp = $2 and inr = $3", self.gp, self.ityp, self.inr as i32)
            .fetch_optional(pg)
            .await
            .ok()
            .flatten()
            .map(|x| x.updated_at).flatten()
    }
}

pub async fn run_shut_latest_gp(
    mut shutdown: Shutdown,
    pg_pool: PgPool,
    new_legis_init: Sender<Option<(i32, LegislativeInitiative)>>,
    ai_task_ts: Sender<AiTask>,
    legis_init_sleep: u64,
    only_latest: bool,
    only_laws: bool,
) {
    #[cfg(feature = "legis_init_latest")]
    while !shutdown.is_shutdown() {
        let new_legis_init = new_legis_init.clone();
        let ai_task_ts = ai_task_ts.clone();
        tokio::select! {
            _ = shutdown.recv() => {
                println!("shutting down");
                // return ok?
                break;
            }
            _ = async {
                let pg_pool = pg_pool.clone();
                if let Err(e) = tokio::task::spawn(async move {
                    log::info!("Starting legislative initiative (latest) scraper");

                    legis_init_once(only_latest, only_laws, &pg_pool, Some(new_legis_init.clone()), ai_task_ts.clone()).await;

                    // triggers the email service
                    let res = new_legis_init.send(None).await;
                    if let Err(e) = res {
                        log::error!("Could not send trigger value to email service: {e:?}");
                    }
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

pub async fn run_shut_every_gp(
    mut shutdown: Shutdown,
    pg_pool: PgPool,
    ai_task_ts: Sender<AiTask>,
) {
    let legis_init_sleep = std::env::var("LEGISLATIVE_INITIATIVE_SLEEP")
        .unwrap_or_else(|_| "1000".to_string())
        .parse()
        .unwrap_or_else(|e| {
            log::warn!(
                "Could not parse LEGISLATIVE_INITIATIVE_SLEEP env var: {e}. Using default value of 1000 seconds"
            );
            1000
        });

    #[cfg(feature = "legis_init")]
    while !shutdown.is_shutdown() {
        let ai_task_ts = ai_task_ts.clone();
        tokio::select! {
            _ = shutdown.recv() => {
                println!("shutting down");
                // return ok?
                break;
            }
            _ = async {
                let pg_pool = pg_pool.clone();
                if let Err(e) = tokio::task::spawn(async move {
                    log::info!("Starting legislative initiative scraper");

                    legis_init_once(false, true, &pg_pool, None, ai_task_ts.clone()).await;
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
    use std::sync::Arc;

    use chrono::{NaiveDate, NaiveDateTime};
    use somes_scraper::{legis_init::LegislativeInitiatives, AustriaScraper};
    use sqlx::postgres::PgPoolOptions;

    use crate::{connect_pg, DATABASE_URL};

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
        crate::legislative_initiatives::update_or_insert_legis_init_in_db(&pool, &res, None, None)
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
        crate::legislative_initiatives::update_or_insert_legis_init_in_db(&pool, &res, None, None)
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
        crate::legislative_initiatives::update_or_insert_legis_init_in_db(&pool, &res, None, None)
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
        crate::legislative_initiatives::update_or_insert_legis_init_in_db(&pool, &res, None, None)
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
        crate::legislative_initiatives::update_or_insert_legis_init_in_db(&pool, &res, None, None)
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
        crate::legislative_initiatives::update_or_insert_legis_init_in_db(&pool, &res, None, None)
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
        crate::legislative_initiatives::update_or_insert_legis_init_in_db(&pool, &res, None, None)
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
        crate::legislative_initiatives::update_or_insert_legis_init_in_db(&pool, &res, None, None)
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
        crate::legislative_initiatives::update_or_insert_legis_init_in_db(&pool, &res, None, None)
            .await;
    }
}
