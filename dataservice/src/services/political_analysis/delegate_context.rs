use crate::{Result, political_analysis::EmbeddingProvider};
use chrono::{Datelike, NaiveDate, Utc};
use combx::Error;
use qdrant_client::{
    Qdrant,
    qdrant::{
        Condition, FieldCondition, Filter, Match, PayloadIncludeSelector, PointId, ScoredPoint,
        SearchPoints, WithPayloadSelector, condition::ConditionOneOf, r#match::MatchValue,
        with_payload_selector::SelectorOptions,
    },
};
use reqwest::Client as HttpClient;
use serde_json::Value as JsonValue;
use std::collections::HashMap;

pub async fn score_search_results_qdrant(
    qdrant_client: &Qdrant,
    embedder: &impl EmbeddingProvider,
    collection_name: &str,
    delegate_id: i32,
    question: &str,
    max_per: f32,
    limit: u64,
) -> Result<Vec<ScoredPoint>> {
    let query_vector: Vec<f64> = embedder.embed_text(question).await?;

    let filter = Filter {
        must: vec![Condition {
            condition_one_of: Some(ConditionOneOf::Field(FieldCondition {
                key: "delegate_id".to_string(),
                r#match: Some(Match {
                    match_value: Some(MatchValue::Integer(delegate_id as i64)),
                }),
                ..Default::default()
            })),
        }],
        ..Default::default()
    };

    let payload_selector = WithPayloadSelector {
        selector_options: Some(SelectorOptions::Include(PayloadIncludeSelector {
            fields: vec![
                "text".to_string(),
                "speech_id".to_string(),
                "upper_padding".to_string(),
                "lower_padding".to_string(),
                "gp".to_string(),
                "date".to_string(),
                "inr".to_string(),
                "about".to_string(),
            ],
        })),
    };

    let search_result = qdrant_client
        .search_points(SearchPoints {
            collection_name: collection_name.to_string(),
            vector: query_vector
                .into_iter()
                .map(|val| val as f32)
                .collect::<Vec<_>>(),
            limit,
            filter: Some(filter),
            with_payload: Some(payload_selector),
            with_vectors: None,
            ..Default::default()
        })
        .await?;

    let searches = search_result.result;

    if searches.is_empty() {
        log::warn!("No speech returned! KONTEXT KANN NICHT FUNKTIONIEREN");
        return Ok(Vec::new());
    }

    let max_score = searches.first().map(|s| s.score).unwrap_or(0.0);
    let mut capped_results = Vec::new();

    for search in searches {
        if max_score > 0.0 && (search.score / max_score) >= max_per {
            capped_results.push(search);
        } else {
            break;
        }
    }

    Ok(capped_results)
}

fn extract_qdrant_number(val: Option<&qdrant_client::qdrant::Value>) -> Option<i64> {
    if let Some(v) = val
        && let Some(qdrant_client::qdrant::value::Kind::IntegerValue(s)) = &v.kind {
            return Some(*s);
        }
    None
}

fn extract_qdrant_string(val: Option<&qdrant_client::qdrant::Value>) -> String {
    if let Some(v) = val
        && let Some(qdrant_client::qdrant::value::Kind::StringValue(s)) = &v.kind {
            return s.clone();
        }
    String::new()
}

fn extract_qdrant_string_array(val: Option<&qdrant_client::qdrant::Value>) -> Vec<String> {
    let mut result = Vec::new();
    if let Some(v) = val
        && let Some(qdrant_client::qdrant::value::Kind::ListValue(list)) = &v.kind {
            for item in &list.values {
                if let Some(qdrant_client::qdrant::value::Kind::StringValue(s)) = &item.kind {
                    result.push(s.clone());
                }
            }
        }
    result
}

pub fn construct_information_from_entity(
    payload: &HashMap<String, qdrant_client::qdrant::Value>,
    id: Option<&PointId>,
) -> Option<String> {
    let id = id.and_then(|id| id.point_id_options.as_ref());
    let upper_padding = extract_qdrant_string_array(payload.get("upper_padding"));
    let upper_padding = if upper_padding.len() as i32 - 30 > 1 {
        &upper_padding[30..]
    } else {
        &upper_padding
    }
    .join(" ");

    let lower_padding = extract_qdrant_string_array(payload.get("lower_padding"));
    let lower_padding = lower_padding[..std::cmp::min(30, lower_padding.len())].join(" ");

    let text = extract_qdrant_string(payload.get("text"));
    let about = extract_qdrant_string(payload.get("about"));
    let date = extract_qdrant_string(payload.get("date"));
    let gp = extract_qdrant_string(payload.get("gp"));
    let inr = extract_qdrant_number(payload.get("inr"))?;
    let speech_id = extract_qdrant_number(payload.get("speech_id"))?;

    Some(format!(
        "Redesnippet zur Debatte '{about}': '{upper_padding} {text} {lower_padding}' vom {date}, in der Nationalratssitzung {gp}/{inr}. Rede-ID: {speech_id}, Point-ID: {id:?}"
    ))
}

pub fn combine_all_speech_snippets(searches: &[ScoredPoint]) -> Option<String> {
    let mut spoken_texts = String::new();
    for search in searches {
        let spoken_text =
            construct_information_from_entity(&search.payload, search.id.as_ref()).unwrap();
        spoken_texts.push_str(&format!("{}\n\n", spoken_text));
    }
    Some(spoken_texts)
}

/// Main orchestration function for extracting text segments answering the query
pub async fn collect_spoken_texts(
    qdrant_client: &Qdrant,
    embedder: &impl EmbeddingProvider,
    delegate_id: i32,
    question: &str,
) -> Result<String> {
    let searches =
        search_points_for_entry(qdrant_client, embedder, delegate_id, question, 25).await?;

    combine_all_speech_snippets(&searches).ok_or(Box::new(Error::Custom(
        "could not extract speech snippets".into(),
    )))
}

pub async fn search_points_for_entry(
    qdrant_client: &Qdrant,
    embedder: &impl EmbeddingProvider,
    delegate_id: i32,
    question: &str,
    limit: u64,
) -> Result<Vec<ScoredPoint>> {
    let rag_idx = question.find("[RAG]").unwrap_or(question.len());
    let processed_question = format!("query: {}", &question[..rag_idx]);
    log::info!("RAG QUESTION: {}", processed_question);
    let collection_name = "delegates";
    let searches = score_search_results_qdrant(
        qdrant_client,
        embedder,
        collection_name,
        delegate_id,
        &processed_question,
        0.87,
        limit,
    )
    .await?;
    Ok(searches)
}

pub async fn collect_delegate_profile(
    http_client: &HttpClient,
    delegate_id: i32,
) -> Result<String> {
    let delegate_url = format!("https://somes.at/api/at/v1/delegates/id/{}", delegate_id);
    let mut delegate_info: JsonValue = http_client.get(&delegate_url).send().await?.json().await?;

    // let qa_url = format!(
    //     "https://somes.at/api/delegate_qa?delegate_id={}",
    //     delegate_id
    // );
    // let qa_info: JsonValue = http_client.get(&qa_url).send().await?.json().await.unwrap();

    if let Some(info_obj) = delegate_info.as_object_mut()
        && let Some(birthdate_str) = info_obj.get("birthdate").and_then(|v| v.as_str())
            && let Ok(birth_date) = NaiveDate::parse_from_str(birthdate_str, "%Y-%m-%d") {
                let today = Utc::now().naive_utc().date();
                let mut age = today.year() - birth_date.year();

                if (today.month(), today.day()) < (birth_date.month(), birth_date.day()) {
                    age -= 1;
                }

                info_obj.insert("age".to_string(), serde_json::json!(age));
            }

        // let has_qa = match &qa_info {
        //     JsonValue::Null => false,
        //     JsonValue::Array(arr) => !arr.is_empty(),
        //     JsonValue::Object(obj) => !obj.is_empty(),
        //     _ => true,
        // };

        // if has_qa {
        //     info_obj.insert("vorstellung".to_string(), qa_info);
        // }

    Ok(serde_json::to_string(&delegate_info)?)
}

#[cfg(test)]
mod tests {
    use qdrant_client::Qdrant;
    use somai_client::SomAiClient;

    use crate::{connect_pg, political_analysis::delegate_context::collect_spoken_texts};

    #[tokio::test]
    async fn test_delegate_rag() {
        simple_logger::init_with_level(log::Level::Info).unwrap();
        let ai_client = SomAiClient::new("http://10.30.0.164:8000", "API_USER", "supersicher123!")
            .await
            .unwrap();

        let qdrant_client = Qdrant::from_url("http://10.30.0.164:6334").build().unwrap();

        let spoken_texts = collect_spoken_texts(
            &qdrant_client,
            &ai_client,
            // 83148,
            8242,
            "Erbschaften sollten besteuert werden.",
        )
        .await
        .unwrap();
        log::info!(
            "spoken_texts: {spoken_texts}, length: {}",
            spoken_texts.len()
        );
    }
}
