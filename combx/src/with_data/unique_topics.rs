use std::collections::HashMap;

use somes_scraper::language::Language;
use sqlx::PgPool;

use crate::{Error::Custom, TopicWithId};

pub struct TopicsMapper {
    pub unique_eurovoc_topics: EurovocTopics,
    pub unique_topics: EurovocTopics,
}

impl TopicsMapper {
    pub async fn new(pg_pool: &PgPool) -> crate::Result<Self> {
        let unique_eurovoc_topics = crate::with_data::unique_topics::create_topic_mapping(
            &pg_pool,
            crate::with_data::unique_topics::TopicType::Eurovoc,
        )
        .await?;

        let unique_topics = crate::with_data::unique_topics::create_topic_mapping(
            &pg_pool,
            crate::with_data::unique_topics::TopicType::Parent,
        )
        .await?;
        Ok(Self {
            unique_eurovoc_topics,
            unique_topics,
        })
    }
}

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
    pub lang_to_topics: HashMap<Language, HashMap<String, TopicWithId>>,
    pub id_to_topic: HashMap<(i64, Language), TopicWithId>,
}

struct QueryTopic {
    topic_name: String,
    language: String,
    id_as_hash: Option<i64>,
}

pub enum TopicType {
    Parent,
    Eurovoc,
}

pub async fn create_topic_mapping(
    pg_pool: &PgPool,
    topic_type: TopicType,
) -> crate::Result<EurovocTopics> {
    let rows = match topic_type {
        TopicType::Eurovoc => {
            sqlx::query_as!(
                QueryTopic,
                "select topic_name, language, id_as_hash from unique_eurovoc_topics"
            )
            .fetch_all(pg_pool)
            .await?
        }
        TopicType::Parent => {
            sqlx::query_as!(
                QueryTopic,
                "select topic_name, language, id_as_hash from unique_topics"
            )
            .fetch_all(pg_pool)
            .await?
        }
    };

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
            .insert(row.topic_name.to_lowercase(), topic.clone());

        if let Some(id_as_hash) = row.id_as_hash {
            eurovoc_topics
                .id_to_topic
                .insert((id_as_hash, language), topic);
        }
    }

    Ok(eurovoc_topics)
}

pub fn translate_topics_with_eurovoc(
    eurovoc_topics: &EurovocTopics,
    language: Language,
    german_topics: &HashMap<String, TopicWithId>,
    topics: &mut Vec<String>,
) {
    let translated_topics = topics
        .iter()
        .flat_map(|topic| {
            let id = german_topics.get(&topic.to_lowercase())?;
            let translated_topic = eurovoc_topics
                .id_to_topic
                .get(&(id.id.parse::<i64>().ok()?, language))?;
            Some(translated_topic.topic.clone())
        })
        .collect::<Vec<String>>();
    *topics = translated_topics;
}
