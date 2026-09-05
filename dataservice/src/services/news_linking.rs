use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use chrono::{TimeDelta, Utc};
use qdrant_client::Qdrant;
use qdrant_client::qdrant::{
    CreateCollection, Distance, GetPoints, PointId, PointStruct, UpsertPoints, VectorParams,
    VectorsConfig, point_id::PointIdOptions, vectors_config::Config as VectorConfig,
};
use serde_json::json;
use sitemaps::{Config, Crawler, DateRange, KIND_ARCHIVE, Query, SitemapEntry};
use somai_client::SomAiClient;
use xxhash_rust::xxh3::xxh3_64;

use crate::{
    AppState, Shutdown,
    clients::{connect_ai_client, connect_qdrant_client},
};

pub const ARTICLE_URLS_COLLECTION: &str = "news_article_urls";

fn point_id(provider: &str, url: &str) -> u64 {
    xxh3_64(format!("{provider}\u{1}{url}").as_bytes())
}

async fn setup_collection(qdrant: &Qdrant, embedding_dim: u64) -> crate::Result<()> {
    if !qdrant.collection_exists(ARTICLE_URLS_COLLECTION).await? {
        qdrant
            .create_collection(CreateCollection {
                collection_name: ARTICLE_URLS_COLLECTION.to_string(),
                vectors_config: Some(VectorsConfig {
                    config: Some(VectorConfig::Params(VectorParams {
                        size: embedding_dim,
                        distance: Distance::Cosine.into(),
                        ..Default::default()
                    })),
                }),
                ..Default::default()
            })
            .await?;
    }
    Ok(())
}

async fn known_ids(qdrant: &Qdrant, ids: &[PointId]) -> crate::Result<HashSet<u64>> {
    let mut known = HashSet::new();
    for chunk in ids.chunks(512) {
        let response = qdrant
            .get_points(GetPoints {
                collection_name: ARTICLE_URLS_COLLECTION.into(),
                ids: chunk.to_vec(),
                ..Default::default()
            })
            .await?;
        for point in response.result {
            if let Some(PointId {
                point_id_options: Some(PointIdOptions::Num(id)),
            }) = point.id
            {
                known.insert(id);
            }
        }
    }
    Ok(known)
}

async fn upload_and_embed_urls(
    qdrant: &Qdrant,
    ai_client: &SomAiClient,
    embedding_model: &str,
    provider: &str,
    entries: &[SitemapEntry],
) -> crate::Result<()> {
    let ids: Vec<(u64, &SitemapEntry)> = entries
        .iter()
        .map(|entry| (point_id(provider, &entry.loc), entry))
        .collect();
    let known = known_ids(
        qdrant,
        &ids.iter()
            .map(|(id, _)| PointId::from(*id))
            .collect::<Vec<_>>(),
    )
    .await?;
    let new: Vec<(u64, &SitemapEntry)> = ids
        .into_iter()
        .filter(|(id, _)| !known.contains(id))
        .collect();
    log::info!(
        "{provider}: embed {} of {} urls ({} already embedded)",
        new.len(),
        entries.len(),
        known.len()
    );

    for chunk in new.chunks(128) {
        let mut batch = Vec::with_capacity(chunk.len());
        for (id, entry) in chunk {
            let embedding = ai_client
                .embedding_embed(entry.loc.to_string(), None)
                .await?
                .embedding;
            let mut payload: HashMap<String, qdrant_client::qdrant::Value> = HashMap::new();
            payload.insert("url".into(), json!(&entry.loc).into());
            payload.insert("provider".into(), json!(provider).into());
            payload.insert(
                "date".into(),
                json!(entry.date().map(|date| date.to_string())).into(),
            );
            payload.insert(
                "lastmod".into(),
                json!(entry.lastmod.map(|lastmod| lastmod.to_rfc3339())).into(),
            );
            payload.insert("embedding_model_name".into(), json!(embedding_model).into());
            payload.insert("add_date".into(), json!(Utc::now().to_rfc3339()).into());
            batch.push(PointStruct::new(
                PointId::from(*id),
                embedding
                    .iter()
                    .map(|value| *value as f32)
                    .collect::<Vec<f32>>(),
                payload,
            ));
        }
        qdrant
            .upsert_points(UpsertPoints {
                collection_name: ARTICLE_URLS_COLLECTION.into(),
                points: batch,
                ..Default::default()
            })
            .await?;
    }
    Ok(())
}

pub async fn upload_and_embed_article_urls(
    _app_state: AppState,
    ai_client: Arc<SomAiClient>,
    qdrant_client: &Qdrant,
    recent_articles: bool,
) -> crate::Result<()> {
    let config = Config::builtin();
    let crawler = Crawler::new(config.clone())?;
    setup_collection(
        qdrant_client,
        ai_client
            .embedding_embed("warmup".into(), None)
            .await?
            .embedding
            .len() as u64,
    )
    .await?;
    let embedding_model = ai_client.get_default_embedding_model_name().await?.result;

    let today = Utc::now().date_naive();
    let since = today - TimeDelta::days(2);
    let range = recent_articles.then(|| DateRange::between(since, today));

    for provider in config.provider_ids() {
        let mut query = Query::new(provider).kind(KIND_ARCHIVE);
        if let Some(range) = range.clone() {
            query = query.range(range);
        }
        let entries = match crawler.crawl(&query).await {
            Ok(crawl) => crawl.into_entries(),
            Err(e) => {
                log::warn!("Could not crawl urls of {provider}: {e}");
                continue;
            }
        };
        if let Err(e) = upload_and_embed_urls(
            qdrant_client,
            &ai_client,
            &embedding_model,
            provider,
            &entries,
        )
        .await
        {
            log::error!("Could not embed urls of {provider}: {e:?}");
        }
    }

    Ok(())
}

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
                if let Err(e) = upload_and_embed_article_urls(
                    app_state.clone(),
                    ai_client.clone(),
                    &qdrant_client,
                    only_recents,
                )
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
