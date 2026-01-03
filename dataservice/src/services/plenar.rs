use std::thread::JoinHandle;

use crate::{scrape_untill_success, should_update::StoredUpdatedAt, Shutdown, GPS};
use somes_scraper::{plenar::PlenarInfo, AustriaScraper};
use sqlx::PgPool;
use tokio::runtime::Handle;

mod scraping;
pub use scraping::*;

mod db;
pub use db::*;

mod inserts;
pub use inserts::*;

pub async fn plenar_info_once(pg_pool: &PgPool, only_latest: bool) {
    log::info!("Try scraping plenar info");

    let tok_handle = Handle::current();

    let mut handles = vec![];

    for gp in GPS {
        let pg_pool = pg_pool.clone();
        let tok_handle = tok_handle.clone();

        // tokio::task::spawn_local(future)

        handles.push(std::thread::spawn(move || {
            tok_handle.block_on(async move {
                let mut scraper = AustriaScraper::default();
                log::info!("PLENAR info once GP: {gp}");
                scraper.scrape_info.gp = vec![gp.into()];

                let new_plenar_infos = scrape_untill_success::<Vec<PlenarInfo>, _>(|| async {
                    scrape_new_plenar_infos(&scraper).await
                })
                .await;

                let mut handles: Vec<JoinHandle<()>> = vec![];

                for plenar_info in new_plenar_infos.into_iter() {
                    if handles.len() > 32 {
                        log::info!("awaiting 32 plenar handles");
                        for handle in handles.drain(..) {
                            if let Err(e) = handle.join() {
                                log::error!("could not await plenar info handle: {e:?}");
                            }
                        }
                    }
                    let pg_pool = pg_pool.clone();

                    let tok_handle = Handle::current();
                    handles.push(std::thread::spawn(move || {
                        tok_handle.block_on(async move {
                            update_plenar_infos_in_db(&plenar_info, &pg_pool).await;
                        })
                    }));
                }
                for handle in handles.drain(..) {
                    if let Err(e) = handle.join() {
                        log::error!("could not await plenar info handle: {e:?}");
                    }
                }
            })
        }));
        if only_latest {
            break;
        }
    }
    for handle in handles.drain(..) {
        if let Err(e) = handle.join() {
            log::error!("could not await plenar info handle: {e:?}");
        }
    }
}

impl StoredUpdatedAt for PlenarInfo {
    async fn stored_updated_at(&self, pg: &PgPool) -> Option<chrono::DateTime<chrono::Utc>> {
        sqlx::query!(
            "select raw_data_updated_at from plenar_infos where inr = $1",
            self.inr as i32
        )
        .fetch_optional(pg)
        .await
        .ok()
        .flatten()
        .map(|x| x.raw_data_updated_at)
        .flatten()
    }
}

pub async fn run_shut_latest(mut shutdown: Shutdown, pg_pool: PgPool) {
    // insert_all_serially().await;
    // return;
    let plenar_sleep = std::env::var("PLENAR_SLEEP_LATEST")
        .unwrap_or_else(|_| "800".to_string())
        .parse()
        .unwrap_or_else(|e| {
            log::warn!(
                "Could not parse PLENAR_SLEEP_LATEST env var: {e}. Using default value of 800 seconds"
            );
            800
        });

    #[cfg(feature = "plenar_latest")]
    while !shutdown.is_shutdown() {
        tokio::select! {
            _ = shutdown.recv() => {
                println!("shutting down");
                // return ok?
                break;
            }
            _ = async {
                let pg_pool = pg_pool.clone();
                if let Err(e) = tokio::task::spawn_blocking(move || Handle::current().block_on(async move {
                    log::info!("Starting latest plenar info scraper");
                    plenar_info_once(&pg_pool, true).await;
                })).await {
                    log::error!("Error in plenar info scraper: {:?}", e);
                }
                // legis_init_once().await;
                log::info!("Latest Plenar info scraper service sleeps for {plenar_sleep} seconds...");
                tokio::time::sleep(tokio::time::Duration::from_secs(plenar_sleep)).await;
            } => {}
        }
    }
}

pub async fn run_shut(mut shutdown: Shutdown, pg_pool: PgPool, only_latest: bool) {
    if only_latest {
        return;
    }
    // insert_all_serially().await;
    // return;
    let plenar_sleep = std::env::var("PLENAR_SLEEP")
        .unwrap_or_else(|_| "80000".to_string())
        .parse()
        .unwrap_or_else(|e| {
            log::warn!(
                "Could not parse PLENAR_SLEEP env var: {e}. Using default value of 80000 seconds"
            );
            80000
        });

    #[cfg(feature = "plenar")]
    while !shutdown.is_shutdown() {
        tokio::select! {
            _ = shutdown.recv() => {
                println!("shutting down");
                // return ok?
                break;
            }
            _ = async {
                let pg_pool = pg_pool.clone();
                if let Err(e) = tokio::task::spawn_blocking(move || Handle::current().block_on(async move {
                    log::info!("Starting plenar info scraper");
                    plenar_info_once(&pg_pool, false).await;
                })).await {
                    log::error!("Error in plenar info scraper: {:?}", e);
                }
                // legis_init_once().await;
                log::info!("Plenar info scraper service sleeps for {plenar_sleep} seconds...");
                tokio::time::sleep(tokio::time::Duration::from_secs(plenar_sleep)).await;
            } => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashSet, io::Read};

    use chrono::NaiveDateTime;
    use rand::Rng;
    use somes_scraper::{plenar::Plenar, AustriaScraper, Scraper, TextExtractor};

    use crate::{
        connect_pg,
        plenar::{insert_plenar_info, update_plenar_infos_in_db},
    };

    #[test]
    fn test_date_time_parse() {
        let x = NaiveDateTime::parse_from_str("2014-5-17T12:34:56", "%Y-%m-%dT%H:%M:%S").unwrap();
        println!("x: {x:?}");
    }

    #[tokio::test]
    async fn test_duplicate_speeches() {
        let pool = connect_pg().await;

        println!("stating selecting");
        let speeches = sqlx::query!("select * from plenar_speeches")
            .fetch_all(&pool)
            .await
            .unwrap();
        println!("finished selecting speeches");

        let mut unique_speeches = HashSet::new();
        for speech in &speeches {
            if let Some(spoken_text) = &speech.spoken_text {
                if !unique_speeches.insert(spoken_text) {
                    // println!("speech: {speech:?}");
                }
            }
        }

        let mut rng = rand::thread_rng();
        loop {
            let idx = rng.gen_range(0..speeches.len());
            let speech = &speeches[idx];
            println!();
            println!(
                "speech: {}",
                speech.spoken_text.clone().unwrap_or("".to_string())
            );
            println!("{:?}", speech);

            println!();

            let mut input = String::new();
            std::io::stdin().read_to_string(&mut input).unwrap();
            println!("input: {input}")
        }
    }

    #[tokio::test]
    async fn test_speeches_lijnks8() {
        unsafe { std::env::set_var("MIND_UPDATE_TIME", "false") };
        simple_logger::init_with_level(log::Level::Info).unwrap();
        let scraper: AustriaScraper = AustriaScraper::default();
        let res = scraper
            .get_plenar("/gegenstand/XXVIII/NRSITZ/37")
            .await
            .unwrap();

        log::info!("legis init: {res:?}");

        let pool = connect_pg().await;

        update_plenar_infos_in_db(&res, &pool).await;
    }
}
