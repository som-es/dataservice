use std::collections::HashSet;

use eurovoc::{Eurovoc, FileType};
use somes_scraper::language::Language;
use xxhash_rust::xxh3::Xxh3;

use crate::{AppState, CompareHash};

const ID_UNIQUE_EUROVOC_TOPICS_JSON_STR: &str =
    include_str!("./unique_topics/id_unique_eurovoc_topics.json");
const ID_UNIQUE_TOPICS_JSON_STR: &str = include_str!("./unique_topics/id_unique_topics.json");

impl CompareHash for eurovoc::Topic {
    async fn stored_hash(&self, _pg: &sqlx::PgPool) -> Option<u64> {
        None
    }
}

fn stable_hash(unique_topic_id: &str, ftype: &str) -> u64 {
    let mut hasher = Xxh3::new();

    hasher.update(unique_topic_id.as_bytes());
    hasher.update(b"\0");
    hasher.update(ftype.as_bytes());

    hasher.digest()
}

pub async fn insert_unique_topics(app_state: &AppState) -> crate::Result<()> {
    let unique_topics = setup().await?;
    let mut unique_translated_eurovoc_topics = HashSet::new();
    let mut unique_translated_topics = HashSet::new();
    for language in unique_topics.somes_languages {
        extend_unique_topics_for_language(
            &unique_topics.eurovoc,
            &unique_topics.id_unique_eurovoc_topics,
            &mut unique_translated_eurovoc_topics,
            language,
        )
        .await?;
        extend_unique_topics_for_language(
            &unique_topics.eurovoc,
            &unique_topics.id_unique_topics,
            &mut unique_translated_topics,
            language,
        )
        .await?;
    }

    for (ftype, unique_topic) in &unique_translated_eurovoc_topics {
        let hash = stable_hash(&unique_topic.id, eurovoc_type_str(ftype));
        sqlx::query!(
            r#"
            insert into unique_eurovoc_topics (topic_name, language, eurovoc_id, eurovoc_type, id_as_hash)
            values ($1, $2, $3, $4, $5)
            on conflict (topic_name) do update set
                language = excluded.language,
                eurovoc_id = excluded.eurovoc_id,
                eurovoc_type = excluded.eurovoc_type,
                id_as_hash = excluded.id_as_hash
            "#,
            unique_topic.label,
            unique_topic.language,
            unique_topic.id,
            eurovoc_type_str(ftype),
            hash as i64
        )
        .execute(&app_state.pg_pool)
        .await?;
    }

    for (ftype, unique_topic) in &unique_translated_eurovoc_topics {
        let hash = stable_hash(&unique_topic.id, eurovoc_type_str(ftype));
        sqlx::query!(
            r#"
            insert into unique_topics (topic_name, language, eurovoc_id, eurovoc_type, id_as_hash)
            values ($1, $2, $3, $4, $5)
            on conflict (topic_name) do update set
                language = excluded.language,
                eurovoc_id = excluded.eurovoc_id,
                eurovoc_type = excluded.eurovoc_type,
                id_as_hash = excluded.id_as_hash
            "#,
            unique_topic.label,
            unique_topic.language,
            unique_topic.id,
            eurovoc_type_str(ftype),
            hash as i64
        )
        .execute(&app_state.pg_pool)
        .await?;
    }

    Ok(())
}

fn eurovoc_type_str(ftype: &FileType) -> &'static str {
    match ftype {
        FileType::Descriptors => "Descriptors",
        FileType::Domain => "Domain",
        FileType::Thesaurus => "Thesaurus",
        FileType::Custom => "Custom",
    }
}

pub struct UniqueTopics {
    pub eurovoc: Eurovoc,
    pub id_unique_eurovoc_topics: Vec<(FileType, eurovoc::Topic)>,
    pub id_unique_topics: Vec<(FileType, eurovoc::Topic)>,
    pub somes_languages: [&'static str; 24],
}

async fn setup() -> crate::Result<UniqueTopics> {
    let eurovoc = Eurovoc::new(None, None)?;
    eurovoc.downloader().ensure_data().await?;
    let id_unique_eurovoc_topics =
        serde_json::from_str::<Vec<(FileType, eurovoc::Topic)>>(ID_UNIQUE_EUROVOC_TOPICS_JSON_STR)?;
    let id_unique_topics =
        serde_json::from_str::<Vec<(FileType, eurovoc::Topic)>>(ID_UNIQUE_TOPICS_JSON_STR)?;
    let languages = Language::ALL.map(|lang| lang.as_str());
    Ok(UniqueTopics {
        eurovoc,
        id_unique_eurovoc_topics,
        id_unique_topics,
        somes_languages: languages,
    })
}

async fn extend_unique_topics_for_language(
    eurovoc: &Eurovoc,
    id_unique_topics: &Vec<(FileType, eurovoc::Topic)>,
    unique_topics: &mut HashSet<(FileType, eurovoc::Topic)>,
    language: &str,
) -> crate::Result<()> {
    unique_topics.extend(
        eurovoc
            .get_topics(FileType::Descriptors, &[language])
            .await?
            .into_iter()
            .map(|topic| (FileType::Descriptors, topic))
            .chain(
                eurovoc
                    .get_topics(FileType::Domain, &[language])
                    .await?
                    .into_iter()
                    .map(|topic| (FileType::Domain, topic)),
            )
            .chain(
                eurovoc
                    .get_topics(FileType::Thesaurus, &[language])
                    .await?
                    .into_iter()
                    .map(|topic| (FileType::Thesaurus, topic)),
            )
            .chain(
                eurovoc
                    .get_topics(FileType::Custom, &[language])
                    .await?
                    .into_iter()
                    .map(|topic| (FileType::Custom, topic)),
            )
            .filter_map(|(ftype, mut topic)| {
                for (unique_topic_ftype, unique_topic) in id_unique_topics {
                    if &ftype == unique_topic_ftype && topic.id == unique_topic.id {
                        if language == Language::De.as_str() && ftype == FileType::Domain {
                            topic.label = unique_topic.label.clone()
                        }
                        return Some((ftype, topic));
                    }
                }
                None
            })
            .map(|(ftype, mut topic)| {
                if language == Language::En.as_str() {
                    topic.label = topic.label.to_lowercase();
                }
                (ftype, topic)
            }),
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::unique_topics::{extend_unique_topics_for_language, setup};

    #[tokio::test]
    async fn test_unique_eurovoc_topics_insert() -> crate::Result<()> {
        let unique_topics = setup().await?;
        let mut unique_eurovoc_topics = HashSet::new();
        extend_unique_topics_for_language(
            &unique_topics.eurovoc,
            &unique_topics.id_unique_eurovoc_topics,
            &mut unique_eurovoc_topics,
            "en",
        )
        .await?;
        let (_ftype, topic) = unique_eurovoc_topics
            .iter()
            .find(|topic| topic.1.id == "7355")
            .unwrap();
        assert_eq!(topic.label, "pharmacy");
        let (_ftype, topic) = unique_eurovoc_topics
            .iter()
            .find(|topic| topic.1.id == "3984")
            .unwrap();
        assert_eq!(topic.label, "parliamentary sitting");
        dbg!(unique_eurovoc_topics);
        Ok(())
    }
    #[tokio::test]
    async fn test_unique_topics_insert() -> crate::Result<()> {
        let unique_topics = setup().await?;
        let mut unique_translated_topics = HashSet::new();
        extend_unique_topics_for_language(
            &unique_topics.eurovoc,
            &unique_topics.id_unique_topics,
            &mut unique_translated_topics,
            "en",
        )
        .await?;
        let (_ftype, topic) = unique_translated_topics
            .iter()
            .find(|topic| topic.1.id == "0")
            .unwrap();
        assert_eq!(topic.label, "interior and justice");
        let (_ftype, topic) = unique_translated_topics
            .iter()
            .find(|topic| topic.1.id == "3206")
            .unwrap();
        assert_eq!(topic.label, "education");
        dbg!(unique_translated_topics);
        Ok(())
    }
}
