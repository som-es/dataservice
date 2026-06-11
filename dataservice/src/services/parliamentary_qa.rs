pub mod ai_tasks;
pub use ai_tasks::*;

mod patch_references;
pub use patch_references::*;
use tokio::sync::mpsc::Sender;

use crate::{ai_task::AiTask, AppState, Shutdown};

pub async fn run_shut_latest_gp(
    mut shutdown: Shutdown,
    app_state: AppState,
    ai_task_tx: Sender<AiTask>,
    only_latest: bool,
) {
    let pqa_sleep =
    std::env::var("PARLIAMENTARY_QA_SLEEP")
            .unwrap_or_else(|_| "400000".to_string())
            .parse()
            .unwrap_or_else(|e| {
                log::warn!(
                    "Could not parse POLITICAL_ANALYSIS_SLEEP env var: {e}. Using default value of 400000 seconds"
                );
                400000
            });
    let pqa_latest_sleep =
        std::env::var("PARLIAMENTARY_QA_SLEEP_LATEST")
                .unwrap_or_else(|_| "800".to_string())
                .parse()
                .unwrap_or_else(|e| {
                    log::warn!(
                        "Could not parse POLITICAL_ANALYSIS_SLEEP_LATEST env var: {e}. Using default value of 800 seconds"
                    );
                    800
                });

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
                    log::info!("Starting legislative initiative (latest) scraper");

                    // legis_init_once(only_latest, only_laws, &app_state, ai_task_tx.clone()).await;
                }).await {
                    log::error!("Error in legislative initiative (latest) scraper: {:?}", e);
                }


                // legis_init_once().await;
                // log::info!("Legislative initiative (latest) scraper service sleeps for {legis_init_sleep} seconds...");
                // tokio::time::sleep(tokio::time::Duration::from_secs(legis_init_sleep)).await;
            } => {}
        }
    }
}
