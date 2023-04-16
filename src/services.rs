use std::future::Future;

use crate::Shutdown;

pub mod delegates;
pub mod proposals;

pub async fn scrape_untill_success<T, O>(f: impl Fn() -> O) -> T
where
    O: std::future::Future<Output = crate::Result<T>>,
{
    let mut new_proposals = f().await;
    while let Err(e) = new_proposals {
        new_proposals = f().await;
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        println!("error {e}")
        // log e
    }
    new_proposals.expect("Above loop always runs till Ok")
}
