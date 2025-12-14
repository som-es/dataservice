pub mod ai_task_service;
pub mod dates;
pub mod delegates;
pub mod delete_was_updated;
pub mod legislative_initiatives;
pub mod mailer;
pub mod ministrial_decrees;
pub mod ministrial_proposals;
pub mod parties;
pub mod plenar;

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
