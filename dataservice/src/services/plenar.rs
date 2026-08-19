use crate::{
    ApiLimiter, GPS, Gps, Shutdown, ai_task::AiTask, scrape_untill_success,
    should_update::StoredUpdatedAt,
};
use combx::Parliament;
use somes_scraper::{
    AustriaScraper,
    eu_parliament::EuParliamentScraper,
    plenar::{Plenar, PlenarInfo},
};
use sqlx::PgPool;
use tokio::sync::mpsc::Sender;

mod scraping;
pub use scraping::*;

mod db;
pub use db::*;

mod inserts;
pub use inserts::*;

mod extract_interjection;
pub use extract_interjection::*;

mod ai_tasks;
pub use ai_tasks::*;

const MAX_TASKS: usize = 1;

pub async fn plenar_info_once(
    pg_pool: &PgPool,
    parliament_api_limiter: ApiLimiter,
    ai_task_tx: Option<Sender<AiTask>>,
    only_latest: bool,
) {
    log::info!("Try scraping plenar info");

    let mut handles = vec![];

    for council in ["NR", "BR"] {
        for gp in GPS {
            let pg_pool = pg_pool.clone();
            let parliament_api_limiter = parliament_api_limiter.clone();
            let ai_task_tx = ai_task_tx.clone();

            let handle = tokio::spawn(async move {
                let mut scraper = AustriaScraper::default();
                log::info!("PLENAR info once GP: {gp}, council: {council}");
                scraper.scrape_info.gp = vec![gp.into()];
                scraper.scrape_info.council_type = vec![council.into()];

                let new_plenar_infos = scrape_untill_success::<Vec<PlenarInfo>, _>(|| async {
                    scrape_new_plenar_infos(&scraper, parliament_api_limiter.clone()).await
                })
                .await;

                let mut inner_handles: Vec<tokio::task::JoinHandle<()>> = vec![];

                for plenar_info in new_plenar_infos.into_iter() {
                    if inner_handles.len() > MAX_TASKS {
                        log::info!("awaiting {MAX_TASKS} plenar handles");
                        for handle in inner_handles.drain(..) {
                            if let Err(e) = handle.await {
                                log::error!("could not await plenar info handle: {e:?}");
                            }
                        }
                    }

                    let pg_pool = pg_pool.clone();
                    let ai_task_tx = ai_task_tx.clone();

                    inner_handles.push(tokio::spawn(async move {
                        update_plenar_infos_in_db(
                            &plenar_info,
                            &pg_pool,
                            ai_task_tx.as_ref(),
                            Parliament::At,
                        )
                        .await;
                    }));
                }

                for handle in inner_handles.drain(..) {
                    if let Err(e) = handle.await {
                        log::error!("could not await plenar info handle: {e:?}");
                    }
                }
            });

            handles.push(handle);

            if only_latest {
                break;
            }
        }
    }

    for handle in handles {
        if let Err(e) = handle.await {
            log::error!("could not await plenar info handle: {e:?}");
        }
    }
}

pub async fn plenar_gps_once<S: Plenar + Gps + Default + Send + Sync>(
    pg_pool: &PgPool,
    parliament_api_limiter: ApiLimiter,
    ai_task_tx: Option<Sender<AiTask>>,
) {
    for gp in S::gps().iter().rev() {
        let mut scraper = S::default();
        log::info!("PLENAR info once GP: {gp}");
        scraper.scrape_info_mut().gp = vec![gp.into()];

        let new_plenar_infos = scrape_untill_success::<Vec<PlenarInfo>, _>(|| async {
            scrape_new_plenar_infos(&scraper, parliament_api_limiter.clone()).await
        })
        .await;

        for plenar_info in new_plenar_infos {
            update_plenar_infos_in_db(&plenar_info, pg_pool, ai_task_tx.as_ref(), S::PARLIAMENT)
                .await;
        }
    }
}

#[async_trait::async_trait]
pub trait PlenarService: Plenar + Send + Sync + Default + Gps {
    async fn run(
        pg_pool: &PgPool,
        parliament_api_limiter: ApiLimiter,
        ai_task_tx: Sender<AiTask>,
        only_latest: bool,
    );
}

#[async_trait::async_trait]
impl PlenarService for EuParliamentScraper {
    async fn run(
        pg_pool: &PgPool,
        parliament_api_limiter: ApiLimiter,
        ai_task_tx: Sender<AiTask>,
        _only_latest: bool,
    ) {
        plenar_gps_once::<EuParliamentScraper>(pg_pool, parliament_api_limiter, Some(ai_task_tx))
            .await;
    }
}

#[async_trait::async_trait]
impl PlenarService for AustriaScraper {
    async fn run(
        pg_pool: &PgPool,
        parliament_api_limiter: ApiLimiter,
        ai_task_tx: Sender<AiTask>,
        only_latest: bool,
    ) {
        plenar_info_once(
            pg_pool,
            parliament_api_limiter,
            Some(ai_task_tx),
            only_latest,
        )
        .await;
    }
}

impl StoredUpdatedAt for PlenarInfo {
    async fn stored_updated_at(&self, pg: &PgPool) -> Option<chrono::DateTime<chrono::Utc>> {
        sqlx::query!(
            "select raw_data_updated_at from plenar_infos where inr = $1 and legislative_period = $2",
            self.inr as i32,
            self.gp,
        )
        .fetch_optional(pg)
        .await
        .ok()
        .flatten()
        .and_then(|x| x.raw_data_updated_at)
    }
}

pub async fn run_shut_latest<S: PlenarService>(
    mut shutdown: Shutdown,
    pg_pool: PgPool,
    parliament_api_limiter: ApiLimiter,
    ai_task_tx: Sender<AiTask>,
) {
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
                let parliament_api_limiter = parliament_api_limiter.clone();
                log::info!("Starting latest plenar info scraper");
                S::run(&pg_pool, parliament_api_limiter.clone(), ai_task_tx.clone(), true).await;
                // legis_init_once().await;
                log::info!("Latest Plenar info scraper service sleeps for {plenar_sleep} seconds...");
                tokio::time::sleep(tokio::time::Duration::from_secs(plenar_sleep)).await;
            } => {}
        }
    }
}

pub async fn run_shut<S: PlenarService>(
    mut shutdown: Shutdown,
    pg_pool: PgPool,
    parliament_api_limiter: ApiLimiter,
    ai_task_tx: Sender<AiTask>,
    only_latest: bool,
) {
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
                let parliament_api_limiter = parliament_api_limiter.clone();
                log::info!("Starting plenar info scraper");
                S::run(&pg_pool, parliament_api_limiter.clone(), ai_task_tx.clone(), false).await;
                // legis_init_once().await;
                log::info!("Plenar info scraper service sleeps for {plenar_sleep} seconds...");
                tokio::time::sleep(tokio::time::Duration::from_secs(plenar_sleep)).await;
            } => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::{HashMap, HashSet},
        io::Read,
    };

    use chrono::NaiveDateTime;
    use combx::GPS;
    use rand::Rng;
    use somes_scraper::{AustriaScraper, plenar::Plenar};

    use crate::{
        connect_pg2 as connect_pg,
        plenar::{scrape_new_plenar_infos, update_plenar_infos_in_db},
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

        update_plenar_infos_in_db(&res, &pool, None, combx::Parliament::At).await;
    }

    #[tokio::test]
    async fn test_speeches_links_sum() {
        // simple_logger::init_with_level(log::Level::Info).unwrap();
        let mut scraper: AustriaScraper = AustriaScraper::default();
        let mut full_count = 0;
        let mut unique_links = HashMap::new();
        for gp in GPS {
            scraper.scrape_info.gp = vec![gp.into()];
            let start = tokio::time::Instant::now();
            println!("Scraping plenar infos for GP: {gp}");
            let plenars = scrape_new_plenar_infos(&scraper, crate::ApiLimiter::new(32))
                .await
                .unwrap();
            let duration = start.elapsed();
            println!(
                "Scraped {} plenar infos for GP: {gp} in {:?}",
                plenars.len(),
                duration
            );

            for plenar in plenars {
                let count: u32 = plenar
                    .debates
                    .iter()
                    .map(|debate| {
                        debate
                            .speeches
                            .iter()
                            .map(|speech| {
                                let mut url_here = HashSet::new();
                                for url in &speech.speech.spoken_text_html_urls {
                                    if url_here.contains(&url.document_url) {
                                        println!("Url collision: {:?}", speech.speech.spoken_text_html_urls)
                                    }
                                    url_here.insert(url.document_url.clone());
                                    if !url.document_url.starts_with("http") {
                                        println!("no HTTP URL: {}", url.document_url);
                                    }
                                    if url.document_url.is_empty() {
                                        println!("empty URL in speech: {:?}", speech);
                                    }
                                    if let Some(col_speech) = unique_links.get(&url.document_url)  {
                                        // println!("collision of speeches!: {col_speech:?} collided with {speech:?} - {}", plenar.inr);
                                        // println!();
                                    }
                                    if url.document_url == "https://www.parlament.gv.at/dokument/XXVIII/NRSITZ/32/C_-_16_55_19_00810278.html#350" {
                                        println!("speech: {speech:?} - {:?} - {}", debate.info, plenar.inr);
                                    }
                                    unique_links.insert(url.document_url.clone(), (speech.clone(), plenar.inr));
                                }
                                speech.speech.spoken_text_html_urls.len() as u32
                            })
                            .sum::<u32>()
                    })
                    .sum();
                full_count += count;
            }
            println!("current count: {full_count}");
            println!("current unique_links count: {}", unique_links.len());
        }
        println!("full_count: {full_count}");
        println!("unique_links count: {}", unique_links.len());
    }

    #[tokio::test]
    async fn test_xxii_nrsitz_12_db_insertion() {
        unsafe { std::env::set_var("MIND_UPDATE_TIME", "false") };
        unsafe { std::env::set_var("DISABLE_SPEECH_TEXT", "false") };
        simple_logger::init_with_level(log::Level::Info).unwrap();
        let scraper: AustriaScraper = AustriaScraper::default();
        let res = scraper
            .get_plenar("/gegenstand/XXII/NRSITZ/12")
            .await
            .unwrap();

        let pool = connect_pg().await;

        update_plenar_infos_in_db(&res, &pool, None, combx::Parliament::At).await;
    }

    #[tokio::test]
    async fn test_xxviii_nrsitz_79_db_insertion() {
        unsafe { std::env::set_var("MIND_UPDATE_TIME", "false") };
        unsafe { std::env::set_var("DISABLE_SPEECH_TEXT", "false") };
        simple_logger::init_with_level(log::Level::Info).unwrap();
        let scraper: AustriaScraper = AustriaScraper::default();
        let res = scraper
            .get_plenar("/gegenstand/XXVIII/NRSITZ/66")
            .await
            .unwrap();

        let pool = connect_pg().await;

        update_plenar_infos_in_db(&res, &pool, None, combx::Parliament::At).await;
    }
    #[tokio::test]
    async fn test_brsitz_979_db_insertion() {
        unsafe { std::env::set_var("MIND_UPDATE_TIME", "false") };
        unsafe { std::env::set_var("DISABLE_SPEECH_TEXT", "false") };
        let _ = simple_logger::init_with_level(log::Level::Info);
        let scraper: AustriaScraper = AustriaScraper::default();
        let res = scraper
            .get_plenar("/gegenstand/BR/BRSITZ/979")
            .await
            .unwrap();

        let pool = connect_pg().await;

        update_plenar_infos_in_db(&res, &pool, None, combx::Parliament::At).await;
    }

    #[tokio::test]
    async fn test_xxviii_nrsitz_32() {
        let scraper: AustriaScraper = AustriaScraper::default();
        let res = scraper
            .get_plenar("/gegenstand/XXVIII/NRSITZ/32")
            .await
            .unwrap();

        let mut unique_links = HashMap::new();
        for debate in res.debates {
            println!("debate: {:?}", debate.info);
            println!();
            for speech in debate.speeches {
                for url in &speech.speech.spoken_text_html_urls {
                    if let Some(col_speech) = unique_links.get(&url.document_url) {
                        println!(
                            "collision of speeches!: {col_speech:?} collided with {speech:?} ",
                        );
                        println!();
                    }
                    unique_links.insert(url.document_url.clone(), (speech.clone()));
                }

                println!("speech: {speech:?}");
            }
        }
    }
}
