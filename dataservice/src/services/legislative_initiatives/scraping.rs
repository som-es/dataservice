use std::sync::Arc;

use combx::VoteResult;
use somes_scraper::{legis_init::LegislativeInitiatives, AustriaScraper, LegislativeInitiative};
use sqlx::PgPool;
use tokio::{runtime::Handle, sync::mpsc::Sender};

use crate::{
    ai_task::AiTask,
    legislative_initiatives::{
        notify::{
            create_color_map, find_notifiers_legis_init, get_current_delegates,
            get_current_parties, get_current_seats, notify_users,
        },
        update_or_insert_legis_init_in_db,
    },
    AppState, LATEST_GP,
};

pub async fn scrape_new_legis_inits(
    state: &AppState,
    scraper: Arc<AustriaScraper>,
    gp: &str,
    ai_task_tx: Sender<AiTask>,
    searched_doktyps: Option<&[&str]>,
    only_latest: bool,
    only_voteable: bool,
) -> crate::Result<Vec<LegislativeInitiative>> {
    let legis_inits = scraper.post_legis_payload(searched_doktyps).await?;
    let legis_init_paths = scraper.legis_paths(legis_inits)?;

    let max_tasks = if gp != LATEST_GP {
        5 // cache hit rate MUST be high for this to work
    } else {
        1
    };
    let mut handles: Vec<tokio::task::JoinHandle<()>> = vec![];

    let (new_vote_result_tx, new_vote_result_rx) = if only_latest && only_voteable {
        let (new_vote_result_tx, new_vote_result_rx) =
            tokio::sync::mpsc::channel::<(i32, VoteResult)>(128);
        (Some(new_vote_result_tx), Some(new_vote_result_rx))
    } else {
        (None, None)
    };

    log::info!("Fetching and inserting ({gp}) now.");
    let mut info_print_counter = 0;
    for legis_init_path in legis_init_paths.iter().rev() {
        let legis_init_path = legis_init_path.clone();
        let scraper = scraper.clone();
        let pool = state.clone();

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

        // let new_legis_init = new_legis_init.clone();
        let new_vote_result_tx = new_vote_result_tx.clone();
        let ai_task_tx = ai_task_tx.clone();

        handles.push(tokio::task::spawn(async move {
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
                new_vote_result_tx,
                Some(ai_task_tx),
            )
            .await;
        }));
    }

    drop(new_vote_result_tx);

    let pg_pool = state.pg_pool.clone();
    let collect_mail_vote_results = tokio::task::spawn(async move {
        if let Some(mut new_vote_result_rx) = new_vote_result_rx {
            let mut all_vote_results = vec![];
            let mut proposals = vec![];
            while let Some(vote_result) = new_vote_result_rx.recv().await {
                if vote_result.1.votes.is_empty() {
                    proposals.push(vote_result);
                } else {
                    all_vote_results.push(vote_result);
                }
            }

            let seats = get_current_seats().await.unwrap_or_default();
            let delegates = get_current_delegates().await.unwrap_or_default();
            let colors = create_color_map(get_current_parties().await.unwrap_or_default());

            let notify_user_legis_inits =
                find_notifiers_legis_init(&pg_pool, &all_vote_results, true).await;
            log::info!(
                "Constructed notifier hashmap with {} entries",
                notify_user_legis_inits.len()
            );

            notify_users(
                &seats,
                &delegates,
                &notify_user_legis_inits,
                colors.clone(),
                true,
            );

            let notify_user_legis_inits =
                find_notifiers_legis_init(&pg_pool, &proposals, false).await;
            log::info!(
                "(proposals) Constructed notifier hashmap with {} entries",
                notify_user_legis_inits.len()
            );

            notify_users(
                &seats,
                &delegates,
                &notify_user_legis_inits,
                colors.clone(),
                false,
            );
        }
    });

    for handle in handles.drain(..) {
        if let Err(e) = handle.await {
            if e.is_panic() {
                log::warn!("join Error: {e:?}")
            }
        }
    }
    collect_mail_vote_results.await?;
    Ok(vec![])
}
