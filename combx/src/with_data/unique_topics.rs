use std::collections::HashMap;

use somes_scraper::language::Language;
use sqlx::PgPool;

use crate::{Error::Custom, TopicWithId};

pub async fn unique_topics_for_lang(pg: &PgPool, language: Language) -> sqlx::Result<Vec<String>> {
    sqlx::query_scalar!(
        "select topic_name as topic from unique_eurovoc_topics where language = $1 order by topic",
        language.as_str()
    )
    .fetch_all(pg)
    .await
}

#[derive(Debug, Clone, Default)]
pub struct EurovocTopics {
    lang_to_topics: HashMap<Language, HashMap<String, TopicWithId>>,
    id_to_topic: HashMap<(i64, Language), TopicWithId>,
}

pub async fn eurovoc_topics_map(pg_pool: &PgPool) -> crate::Result<EurovocTopics> {
    let rows = sqlx::query!("select topic_name, language, id_as_hash from unique_eurovoc_topics")
        .fetch_all(pg_pool)
        .await?;

    let mut eurovoc_topics = EurovocTopics::default();

    for row in rows {
        let language = row
            .language
            .parse::<Language>()
            .map_err(|e| sqlx::Error::Configuration(Box::new(e)))?;
        let topic = TopicWithId {
            topic: row.topic_name.clone(),
            id: row.id_as_hash.map(|id| id.to_string()).ok_or_else(|| {
                Custom(format!(
                    "Missing topic id for {language} and topic '{}'",
                    row.topic_name
                ))
            })?,
        };

        eurovoc_topics
            .lang_to_topics
            .entry(language)
            .or_default()
            .insert(row.topic_name, topic.clone());

        if let Some(id_as_hash) = row.id_as_hash {
            eurovoc_topics
                .id_to_topic
                .insert((id_as_hash, language), topic);
        }
    }

    Ok(eurovoc_topics)
}
