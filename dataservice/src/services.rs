use tokio::sync::mpsc;

use crate::{ai_task::AiTask, AppState, Shutdown};

pub mod ai_task_service;
pub mod dates;
pub mod delegates;
pub mod legislative_initiatives;
pub mod ministrial_decrees;
pub mod ministrial_proposals;
#[cfg(feature = "parliamentary_qa")]
pub mod parliamentary_qa;
pub mod parties;
pub mod plenar;
#[cfg(feature = "vectordb")]
pub mod political_analysis;

pub async fn political_analysis_service(
    _shutdown: Shutdown,
    _pg_pool: sqlx::PgPool,
    _ai_task_tx: mpsc::Sender<AiTask>,
) {
    #[cfg(feature = "vectordb")]
    political_analysis::run_shut(_shutdown, _pg_pool, _ai_task_tx).await
}

pub async fn parliamentary_qa_service(
    _shutdown: Shutdown,
    _app_state: AppState,
    _ai_task_tx: mpsc::Sender<AiTask>,
    _only_latest: bool,
) {
    #[cfg(feature = "parliamentary_qa")]
    parliamentary_qa::run_shut_latest_gp(
        _shutdown,
        _app_state.clone(),
        _ai_task_tx.clone(),
        _only_latest,
    )
    .await
}

pub async fn scrape_untill_success<T, O>(f: impl Fn() -> O) -> T
where
    O: std::future::Future<Output = crate::Result<T>>,
{
    let mut new_proposals = f().await;
    while let Err(e) = new_proposals {
        log::warn!(
            "Could not scrape data: {e}, INFO for me: {:?}",
            e.downcast_ref::<reqwest::Error>()
        );

        new_proposals = f().await;
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
    new_proposals.expect("Above loop always runs till Ok")
}
