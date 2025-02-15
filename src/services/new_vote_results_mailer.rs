use somes_scraper::LegislativeInitiative;
use sqlx::PgPool;
use tokio::{sync::mpsc::Receiver, time::Instant};

use crate::Shutdown;

use super::legislative_initiatives::notify::find_notifiers;

pub async fn notify_on_new_vote_results(pg_pool: &PgPool, new_vote_result_rx: &mut Receiver<LegislativeInitiative>) {
    let legis_init_accumulate_dur = std::env::var("LEGIS_INIT_ACCUMULATE_DUR")
        .unwrap_or_else(|_| "60".to_string())
        .parse()
        .unwrap_or_else(|e| {
            log::warn!(
                "Could not parse LEGIS_INIT_ACCUMULATE_DUR env var: {e}. Using default value of 60 seconds"
            );
            60
        });

    loop {
        let Some(new_legis_init) = new_vote_result_rx.recv().await else {
            continue
        };

        let mut acc_legis_inits = vec![new_legis_init];

        let now = Instant::now();

        while (Instant::now() - now).as_secs_f32() <= legis_init_accumulate_dur as f32 {
            if let Ok(legis_init) = new_vote_result_rx.try_recv() {
                acc_legis_inits.push(legis_init);
            }
        }

        let notify_user_legis_inits = find_notifiers(pg_pool, &acc_legis_inits).await;
    }
}

pub async fn notify_on_new_vote_results_service(
    mut shutdown: Shutdown,
    mut new_vote_result_rx: Receiver<LegislativeInitiative>,
    pg_pool: PgPool
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
