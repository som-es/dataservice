use somes_scraper::{ministrial_proposals::MinistrialProposal, LegislativeInitiative};
use sqlx::PgPool;
use tokio::sync::mpsc::Receiver;

use crate::{ministrial_proposals::find_notifiers_ministrial_props, Shutdown};

pub async fn notify_on_new_ministrial_props(
    pg_pool: &PgPool,
    new_ministrial_prop_rx: &mut Receiver<Option<(i32, MinistrialProposal)>>,
) {
    loop {
        let mut acc_ministrial_props = Vec::new();
        loop {
            let new_ministrial_prop = new_ministrial_prop_rx.recv().await;
            match new_ministrial_prop {
                Some(Some(ministrial_prop)) => {
                    acc_ministrial_props.push(ministrial_prop);
                }

                Some(None) => break,
                None => {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    continue;
                }
            }
        }

        let notify_user_legis_inits = find_notifiers_ministrial_props(pg_pool, &acc_ministrial_props).await;
        log::info!(
            "Constructed notifier hashmap with {} entries",
            notify_user_legis_inits.len()
        );

        // notify_users(&seats, &delegates, &notify_user_legis_inits);
    }
}

pub async fn notify_on_new_vote_results_service(
    mut shutdown: Shutdown,
    mut new_vote_result_rx: Receiver<Option<(i32, MinistrialProposal)>>,
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
                notify_on_new_ministrial_props(&pg_pool, &mut new_vote_result_rx).await
            } => {}
        }
    }
}
