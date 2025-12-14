use std::sync::Arc;

use somes_scraper::{AustriaScraper, LegislativeInitiative, legis_init::LegislativeInitiatives};
use sqlx::PgPool;
use tokio::{runtime::Handle, sync::mpsc::Sender};

use crate::{LATEST_GP, ai_task::AiTask, legislative_initiatives::update_or_insert_legis_init_in_db};

pub async fn scrape_new_legis_inits(
    pool: &PgPool,
    scraper: Arc<AustriaScraper>,
    gp: &str,
    new_legis_init: Option<Sender<Option<(i32, LegislativeInitiative)>>>,
    ai_task_tx: Sender<AiTask>,
    searched_doktyps: Option<&[&str]>,
) -> crate::Result<Vec<LegislativeInitiative>> {
    let legis_inits = scraper.post_legis_payload(searched_doktyps).await?;
    let legis_init_paths = scraper.legis_paths(legis_inits)?;

    let max_tasks = if gp != LATEST_GP {
        5 // cache hit rate MUST be high for this to work
    } else {
        1
    };
    let mut handles: Vec<tokio::task::JoinHandle<()>> = vec![];

    log::info!("Fetching and inserting ({gp}) now.");
    let mut info_print_counter = 0;
    for legis_init_path in legis_init_paths.iter().rev() {
        let legis_init_path = legis_init_path.clone();
        let scraper = scraper.clone();
        let pool = pool.clone();

        if handles.len() > max_tasks {
            if info_print_counter % 16 == 0 {
                log::info!("Awaiting handles ({gp}) now.");
            }
            for handle in handles.drain(..max_tasks) {
                if let Err(e) = handle.await {
                    if e.is_panic() {
                        log::error!("join Error: {e:?}")
                    }
                }
            }
            info_print_counter += 1;
        }
        let new_legis_init = new_legis_init.clone();
        let ai_task_tx = ai_task_tx.clone();

        handles.push(tokio::task::spawn_blocking(move || {
            Handle::current().block_on(async move {
                // skipping legis inits that can't be processed right now
                // mainly many I and A
                // https://www.parlament.gv.at/gegenstand/XXVII/I/1494
                // https://www.parlament.gv.at/gegenstand/XXVII/A/2686
                let Ok(legis_init) = scraper.get_legis_init(&legis_init_path).await else {
                    log::debug!("Cannot process legislative initiative: '{legis_init_path}'");
                    return;
                };

                update_or_insert_legis_init_in_db(
                    &pool,
                    &legis_init,
                    new_legis_init,
                    Some(ai_task_tx),
                )
                .await;
            })
        }));
    }
    for handle in handles.drain(..) {
        if let Err(e) = handle.await {
            if e.is_panic() {
                log::warn!("join Error: {e:?}")
            }
        }
    }

    Ok(vec![])
}