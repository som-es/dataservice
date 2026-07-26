use crate::Result;
use crate::political_analysis::EmbeddingProvider;
use chrono::{Datelike, Utc};
use qdrant_client::Qdrant;
use qdrant_client::qdrant::r#match::MatchValue;
use qdrant_client::qdrant::points_selector::PointsSelectorOneOf;
use qdrant_client::qdrant::{
    Condition, CreateCollection, Distance, FieldCondition, Filter, Match, VectorParams,
    VectorsConfig, condition::ConditionOneOf, vectors_config::Config,
};
use qdrant_client::qdrant::{DeletePoints, PointStruct, PointsSelector, UpsertPoints};
use serde_json::json;
use sqlx::PgPool;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Delegate {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Speech {
    pub id: i32,
    pub spoken_text: String,
    pub about: Option<String>,
    pub add_date: Option<chrono::DateTime<Utc>>,
    pub legislative_period: Option<String>,
    pub inr: i32,
}

pub async fn get_delegate_ids(pool: &PgPool) -> Result<Vec<Delegate>> {
    let delegates = sqlx::query_as!(
        Delegate,
        "SELECT id, name FROM delegates WHERE is_active = true"
    )
    .fetch_all(pool)
    .await?;

    Ok(delegates)
}

/// Fetch speech texts for a specific delegate
pub async fn get_speech_texts(pool: &PgPool, delegate_id: i32) -> Result<Vec<Speech>> {
    let speeches = sqlx::query_as!(
        Speech,
        r#"
        SELECT
            ps.id,
            ps.spoken_text as "spoken_text!",
            debates.about,
            raw_data_created_at as add_date,
            legislative_period,
            pi.inr
        FROM plenar_speeches ps
        INNER JOIN debates ON debates.id = ps.debate_id
        INNER JOIN plenar_infos pi ON pi.id = debates.plenar_id
        WHERE ps.delegate_id = $1 AND ps.spoken_text IS NOT NULL
        "#,
        delegate_id
    )
    .fetch_all(pool)
    .await?;

    Ok(speeches)
}

/// Idempotent setup for the Qdrant Collection
pub async fn setup_qdrant_collection(
    qdrant: &Qdrant,
    collection_name: &str,
    embedding_dim: u64,
) -> Result<()> {
    if !qdrant.collection_exists(collection_name).await? {
        qdrant
            .create_collection(CreateCollection {
                collection_name: collection_name.to_string(),
                vectors_config: Some(VectorsConfig {
                    config: Some(Config::Params(VectorParams {
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

/// Pure function to chunk speech text with upper and lower padding contexts
pub fn chunk_speech(
    text: &str,
    chunk_size: usize,
    padding_size: usize,
) -> Vec<(String, Vec<String>, Vec<String>)> {
    let words: Vec<&str> = text.split(' ').collect();
    let mut chunks = Vec::new();

    for (idx, chunk) in words.chunks(chunk_size).enumerate() {
        let start_idx = idx * chunk_size;
        let end_idx = start_idx + chunk.len();

        let upper_start = start_idx.saturating_sub(padding_size);
        let upper_padding: Vec<String> = words[upper_start..start_idx]
            .iter()
            .map(|s| s.to_string())
            .collect();

        let lower_end = (end_idx + padding_size).min(words.len());
        let lower_padding: Vec<String> = words[end_idx..lower_end]
            .iter()
            .map(|s| s.to_string())
            .collect();

        let chunk_text = chunk.join(" ");
        chunks.push((chunk_text, upper_padding, lower_padding));
    }

    chunks
}

pub async fn process_delegate(
    pool: &PgPool,
    qdrant: &Qdrant,
    embedder: &impl EmbeddingProvider,
    collection_name: &str,
    delegate: &Delegate,
    update_entries: bool,
) -> crate::Result<()> {
    // 1. Check if delegate already exists in Qdrant (Scroll with filter)
    let filter = Filter {
        must: vec![Condition {
            condition_one_of: Some(ConditionOneOf::Field(FieldCondition {
                key: "delegate_id".to_string(),
                r#match: Some(Match {
                    match_value: Some(MatchValue::Integer(delegate.id as i64)),
                }),
                ..Default::default()
            })),
        }],
        ..Default::default()
    };

    let scroll_res = qdrant
        .scroll(qdrant_client::qdrant::ScrollPoints {
            collection_name: collection_name.to_string(),
            filter: Some(filter.clone()),
            limit: Some(1),
            ..Default::default()
        })
        .await?;

    if !scroll_res.result.is_empty() && !update_entries {
        log::info!("Skipping delegate {} (already in Qdrant)", delegate.id);
        return Ok(());
    }

    let speeches: Vec<Speech> = get_speech_texts(pool, delegate.id).await?;
    if speeches.is_empty() {
        return Ok(());
    }

    let mut points: Vec<PointStruct> = Vec::new();

    let mut year_speech_map = HashMap::<i32, u32>::new();

    for speech in &speeches {
        if let Some(add_date) = speech.add_date {
            year_speech_map
                .entry(add_date.year())
                .and_modify(|count| *count += 1)
                .or_insert(1);
        }
        let chunked_data = chunk_speech(&speech.spoken_text, 90, 100);

        for (chunk_text, upper_padding, lower_padding) in chunked_data {
            let chunk_len = chunk_text.len();
            let upper_len: usize = upper_padding.iter().map(|s| s.len()).sum();
            let lower_len: usize = lower_padding.iter().map(|s| s.len()).sum();

            if chunk_len >= 1900 || upper_len >= 32030 || lower_len >= 32030 {
                log::warn!("Chunk length exceeded limits, skipping chunk.");
                continue;
            }

            let passage_line = format!("passage: {}", chunk_text);

            let vector: Vec<f64> = embedder.embed_text(&passage_line).await?;

            let mut payload: HashMap<String, qdrant_client::qdrant::Value> = HashMap::new();
            payload.insert("delegate_id".to_string(), json!(delegate.id).into());
            payload.insert("text".to_string(), json!(chunk_text).into());
            payload.insert("lower_padding".to_string(), json!(lower_padding).into());
            payload.insert("upper_padding".to_string(), json!(upper_padding).into());
            payload.insert("speech_id".to_string(), json!(speech.id).into());

            if let Some(about) = &speech.about {
                payload.insert("about".to_string(), json!(about).into());
            }
            if let Some(date) = &speech.add_date {
                payload.insert("date".to_string(), json!(date.to_string()).into());
            }
            if let Some(gp) = &speech.legislative_period {
                payload.insert("gp".to_string(), json!(gp).into());
            }
            payload.insert("inr".to_string(), json!(speech.inr).into());

            points.push(PointStruct::new(
                uuid::Uuid::new_v4().to_string(),
                vector.into_iter().map(|val| val as f32).collect::<Vec<_>>(),
                payload,
            ));
        }
    }

    log::info!(
        "speech count: {} by year: {year_speech_map:?}",
        speeches.len()
    );

    if update_entries {
        let delete_res = qdrant
            .delete_points(DeletePoints {
                collection_name: collection_name.to_string(),
                wait: Some(true),
                points: Some(PointsSelector {
                    points_selector_one_of: Some(PointsSelectorOneOf::Filter(filter)),
                }),
                ..Default::default()
            })
            .await?;
        log::info!(
            "Successfully deleted points of delegate: {}, {delete_res:?}",
            delegate.id
        );
    }

    let batch_size = 500;
    for batch in points.chunks(batch_size) {
        let upsert_points = UpsertPoints {
            collection_name: collection_name.into(),
            points: batch.to_vec(),
            ..Default::default()
        };
        qdrant.upsert_points(upsert_points).await?;
    }

    sqlx::query!("insert into last_vector_speech_update(delegate_id) values ($1) on conflict (delegate_id) do update set updated_at = now()", delegate.id).execute(pool).await?;

    Ok(())
}

pub async fn process_all_delegates(
    pool: &PgPool,
    qdrant: &Qdrant,
    embedder: &impl EmbeddingProvider,
    collection_name: &str,
) -> Result<()> {
    // Ensure collection setup
    setup_qdrant_collection(qdrant, collection_name, embedder.embedding_dim().await?).await?;

    let delegates = get_delegate_ids(pool).await?;

    for delegate in delegates {
        log::info!("Processing delegate ID: {}", delegate.id);
        if let Err(e) =
            process_delegate(pool, qdrant, embedder, collection_name, &delegate, false).await
        {
            log::error!("Failed processing delegate {}: {:?}", delegate.id, e);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use qdrant_client::Qdrant;
    use somai_client::SomAiClient;

    use crate::{connect_pg, political_analysis::embed_speeches::process_all_delegates};

    #[tokio::test]
    async fn test_delegate_speeches_upload() {
        simple_logger::init_with_level(log::Level::Info).unwrap();
        let ai_client = SomAiClient::new("http://10.30.0.164:8000", "API_USER", "supersicher123!")
            .await
            .unwrap();

        let qdrant_client = Qdrant::from_url("http://10.30.0.164:6334").build().unwrap();

        let pg_pool = connect_pg().await;

        process_all_delegates(&pg_pool, &qdrant_client, &ai_client, "delegates")
            .await
            .unwrap();
    }
}
