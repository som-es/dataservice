use combx::{PostgresEnhancer, VoteResult};
use somes_scraper::LegislativeInitiative;
use sqlx::PgPool;
use tokio::sync::mpsc::Receiver;

use crate::Shutdown;

use crate::legislative_initiatives::notify::{
    create_color_map, find_notifiers_legis_init, get_current_delegates, get_current_parties,
    get_current_seats, notify_users,
};

pub async fn notify_on_new_vote_results(
    pg_pool: &PgPool,
    new_vote_result_rx: &mut Receiver<Option<(i32, LegislativeInitiative)>>,
) {
    let _legis_init_accumulate_dur = std::env::var("LEGIS_INIT_ACCUMULATE_DUR")
        .unwrap_or_else(|_| "60".to_string())
        .parse()
        .unwrap_or_else(|e| {
            log::warn!(
                "Could not parse LEGIS_INIT_ACCUMULATE_DUR env var: {e}. Using default value of 60 seconds"
            );
            60
        });

    let seats = get_current_seats().await.unwrap_or_default();
    let delegates = get_current_delegates().await.unwrap_or_default();
    let colors = create_color_map(get_current_parties().await.unwrap_or_default());

    loop {
        let mut acc_legis_inits = Vec::new();
        loop {
            let new_legis_init = new_vote_result_rx.recv().await;
            match new_legis_init {
                Some(Some((id, legis_init))) => {
                    let pg_pool_enhancer = PostgresEnhancer(pg_pool.clone());
                    let legis_init =
                        VoteResult::from_legis_init(pg_pool_enhancer, legis_init).await;
                    acc_legis_inits.push((id, legis_init));
                }

                Some(None) => break,
                None => {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    continue;
                }
            }
        }

        let notify_user_legis_inits = find_notifiers_legis_init(pg_pool, &acc_legis_inits).await;
        log::info!(
            "Constructed notifier hashmap with {} entries",
            notify_user_legis_inits.len()
        );

        notify_users(&seats, &delegates, &notify_user_legis_inits, colors.clone());
    }
}

pub async fn notify_on_new_vote_results_service(
    mut shutdown: Shutdown,
    mut new_vote_result_rx: Receiver<Option<(i32, LegislativeInitiative)>>,
    pg_pool: PgPool,
) {
    while !shutdown.is_shutdown() {
        tokio::select! {
            _ = shutdown.recv() => {
                println!("shutting down");
                // return ok?
                break;
            }
            _ = async {
                notify_on_new_vote_results(&pg_pool, &mut new_vote_result_rx).await
            } => {}
        }
    }
}
