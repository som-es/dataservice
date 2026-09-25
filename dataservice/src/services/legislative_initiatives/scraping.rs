use std::{sync::Arc, time::Duration};

use combx::{Parliament, VoteResult};
use somes_scraper::{LegislativeInitiative, error::ScraperError};
use tokio::sync::mpsc::Sender;

use crate::{
    AppState,
    ai_task::AiTask,
    legislative_initiatives::{
        VoteResultService,
        notify::{
            create_color_map, find_notifiers_legis_init, find_push_notifiers_legis_init,
            get_current_delegates, get_current_parties, get_current_seats, notify_users,
            send_push_for_legis_init,
        },
        update_or_insert_legis_init_in_db,
    },
    scrape_untill_success,
};

pub const LEGIS_INIT_MAX_INSERT_TASKS: usize = 8;

pub async fn scrape_new_legis_inits<S: VoteResultService>(
    state: &AppState,
    scraper: Arc<S>,
    gp: &str,
    ai_task_tx: Sender<AiTask>,
    searched_doktyps: Option<&[&str]>,
    only_latest: bool,
    only_voteable: bool,
    parliament: Parliament,
    max_tasks: usize,
) -> crate::Result<Vec<LegislativeInitiative>> {
    let legis_inits = scrape_untill_success(async || {
        let legis_inits = if only_latest {
            state
                .parliament_api_limiter
                .run(scraper.post_legis_payload_latest(searched_doktyps))
                .await?
        } else {
            state
                .parliament_api_limiter
                .run(scraper.post_legis_payload(searched_doktyps))
                .await?
        };
        Ok(legis_inits)
    })
    .await;
    let legis_init_paths = scraper.legis_paths(legis_inits)?;
    let mut handles: Vec<tokio::task::JoinHandle<()>> = vec![];

    let (new_vote_result_tx, new_vote_result_rx) = if only_latest && only_voteable {
        let (new_vote_result_tx, new_vote_result_rx) =
            tokio::sync::mpsc::channel::<(i32, VoteResult)>(1024);
        (Some(new_vote_result_tx), Some(new_vote_result_rx))
    } else {
        (None, None)
    };

    log::info!("Fetching and inserting ({gp}) now.");
    for legis_init_path in legis_init_paths.iter().rev() {
        let legis_init_path = legis_init_path.clone();
        let scraper = scraper.clone();
        let pool = state.clone();

        // let new_legis_init = new_legis_init.clone();
        let new_vote_result_tx = new_vote_result_tx.clone();
        let ai_task_tx = ai_task_tx.clone();

        if handles.len() >= max_tasks {
            for handle in handles.drain(..max_tasks) {
                if let Err(e) = handle.await
                    && e.is_panic()
                {
                    log::error!("join Error: {e:?}")
                }
            }
        }

        handles.push(tokio::task::spawn(async move {
            // skipping legis inits that can't be processed right now
            // mainly many I and A
            // https://www.parlament.gv.at/gegenstand/XXVII/I/1494
            // https://www.parlament.gv.at/gegenstand/XXVII/A/2686

            const MAX_RETRIES: u32 = 3;
            let mut attempt = 0;
            let legis_init = loop {
                match pool.parliament_api_limiter
                    .run(scraper.get_legis_init(&legis_init_path.clone()))
                    .await
                {
                    Ok(legis_init) => break Some(legis_init),

                    // Err(ScraperError::Forbidden) => return Err(Box::new(ScraperError::Forbidden)),
                    Err(ScraperError::Forbidden) => return,

                    Err(ScraperError::TooManyRequests(retry_after)) if attempt < MAX_RETRIES => {
                        attempt += 1;
                        let wait = retry_after.unwrap_or(Duration::from_secs(1 << attempt));
                        log::warn!(
                            "429 on {legis_init_path:?}, waiting {wait:?} (attempt {attempt}/{MAX_RETRIES})",
                        );
                        tokio::time::sleep(wait).await;
                    }

                    Err(e) => {
                        log::error!("Cannot process legislative initiative: '{legis_init_path}' because of e: {e:?}");
                        break None;
                    }
                }
            };

            let Some(legis_init) = legis_init else { return  };

            if let Err(e) = update_or_insert_legis_init_in_db(
                &pool,
                &legis_init,
                new_vote_result_tx,
                Some(ai_task_tx),
                parliament,
            )
            .await {
                log::error!("could not update or insert {}/{}/{} because of {e:?}", legis_init.gp, legis_init.ityp, legis_init.inr)
            }
        }));
    }

    drop(new_vote_result_tx);

    let pg_pool = state.pg_pool.clone();
    let collect_mail_vote_results = tokio::task::spawn(async move {
        if let Some(mut new_vote_result_rx) = new_vote_result_rx {
            let mut all_vote_results = vec![];
            let mut proposals = vec![];
            while let Some(vote_result) = new_vote_result_rx.recv().await {
                if vote_result.1.legislative_initiative.accepted.is_none() {
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

            let notify_push_legis_inits =
                find_push_notifiers_legis_init(&pg_pool, &all_vote_results, true).await;
            send_push_for_legis_init(&pg_pool, parliament, &notify_push_legis_inits, true).await;

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

            let notify_push_legis_inits =
                find_push_notifiers_legis_init(&pg_pool, &proposals, false).await;
            send_push_for_legis_init(&pg_pool, parliament, &notify_push_legis_inits, false).await;
        }
    });

    for handle in handles.drain(..) {
        if let Err(e) = handle.await
            && e.is_panic()
        {
            log::warn!("join Error: {e:?}")
        }
    }
    collect_mail_vote_results.await?;
    Ok(vec![])
}
