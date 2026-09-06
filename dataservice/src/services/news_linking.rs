pub mod linking;
mod upload_and_embed;

use std::sync::Arc;

use qdrant_client::Qdrant;
use somai_client::SomAiClient;

use crate::news_linking::linking::run_news_linking;
use crate::news_linking::upload_and_embed::upload_and_embed_article_urls;
use crate::{
    AppState, Shutdown,
    clients::{connect_ai_client, connect_qdrant_client},
};

pub const ARTICLE_URLS_COLLECTION: &str = "news_article_urls_2";

fn sleep_seconds() -> u64 {
    std::env::var("NEWS_LINKING_SLEEP")
        .unwrap_or_else(|_| "10800".to_string())
        .parse()
        .unwrap_or_else(|e| {
            log::warn!(
                "Could not parse NEWS_LINKING_SLEEP env var: {e}. Using default value of 10800 seconds"
            );
            10800
        })
}

pub async fn run_news_linking_and_embed(
    app_state: AppState,
    ai_client: Arc<SomAiClient>,
    qdrant_client: &Qdrant,
    only_recents: bool,
) -> crate::Result<()> {
    upload_and_embed_article_urls(
        app_state.clone(),
        ai_client.clone(),
        &qdrant_client,
        only_recents,
    )
    .await?;

    run_news_linking(app_state, qdrant_client, &ai_client, only_recents).await?;
    Ok(())
}

pub async fn run_shut(mut shutdown: Shutdown, app_state: AppState, only_recents: bool) {
    let news_linking_sleep = sleep_seconds();

    let Some(ai_client) = connect_ai_client().await else {
        log::error!("news linking service stops because it has no somes ai client");
        return;
    };
    let Some(qdrant_client) = connect_qdrant_client().await else {
        log::error!("news linking service stops because it has no qdrant client");
        return;
    };

    while !shutdown.is_shutdown() {
        tokio::select! {
            _ = shutdown.recv() => {
                log::info!("shutting down news linking service");
                break;
            }
            _ = async {
                log::info!("Starting news linking service");
                if let Err(e) = run_news_linking_and_embed(app_state.clone(), ai_client.clone(), &qdrant_client, only_recents)
                .await
                {
                    log::error!("could not complete news linking run: {e:?}")
                }
                log::info!("news linking service sleeps for {news_linking_sleep} seconds...");
                tokio::time::sleep(tokio::time::Duration::from_secs(news_linking_sleep)).await;
            } => {}
        }
        if only_recents {
            break;
        }
    }
}

#[cfg(test)]
mod tests;
