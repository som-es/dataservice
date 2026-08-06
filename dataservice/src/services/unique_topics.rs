use std::collections::HashSet;

use eurovoc::{Eurovoc, FileType};
use somes_scraper::language::Language;

use crate::AppState;

const ID_UNIQUE_EUROVOC_TOPICS_JSON_STR: &str =
    include_str!("./unique_topics/id_unique_eurovoc_topics.json");

pub async fn unique_topics_insert(app_state: &AppState) -> crate::Result<()> {
    let (eurovoc, id_unique_topics, languages) = setup().await?;
    let mut unique_topics = HashSet::new();
    for language in languages {
        extend_unique_topics_for_language(
            &eurovoc,
            &id_unique_topics,
            &mut unique_topics,
            language,
        )
        .await?;
    }

    for (ftype, unique_topic) in &unique_topics {
        sqlx::query!(
            r#"
            insert into unique_eurovoc_topics (topic_name, language, eurovoc_id, eurovoc_type)
            values ($1, $2, $3, $4)
            on conflict (topic_name) do update set
                language = excluded.language,
                eurovoc_id = excluded.eurovoc_id,
                eurovoc_type = excluded.eurovoc_type
            "#,
            unique_topic.label,
            unique_topic.language,
            unique_topic.id,
            eurovoc_type_str(ftype),
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
    }
}

async fn setup() -> crate::Result<(Eurovoc, Vec<(FileType, eurovoc::Topic)>, [&'static str; 24])> {
    let eurovoc = Eurovoc::new(None, None)?;
    eurovoc.downloader().ensure_data().await?;
    let id_unique_topics =
        serde_json::from_str::<Vec<(FileType, eurovoc::Topic)>>(ID_UNIQUE_EUROVOC_TOPICS_JSON_STR)?;
    let languages = Language::ALL.map(|lang| lang.as_str());
    Ok((eurovoc, id_unique_topics, languages))
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
                    .map(|topic| (FileType::Domain, topic)),
            )
            .filter(|(ftype, topic)| {
                for (unique_topic_ftype, unique_topic) in id_unique_topics {
                    if ftype == unique_topic_ftype && topic.id == unique_topic.id {
                        return true;
                    }
                }
                false
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
    async fn test_unique_topics_insert() -> crate::Result<()> {
        let (eurovoc, id_unique_topics, _languages) = setup().await?;
        let mut unique_topics = HashSet::new();
        extend_unique_topics_for_language(
            &eurovoc,
            &id_unique_topics,
            &mut unique_topics,
            "en",
        )
        .await?;
        let (_ftype, topic) = unique_topics.iter().find(|topic| topic.1.id == "7355").unwrap();
        assert_eq!(topic.label, "pharmacy");
        let (_ftype, topic) = unique_topics.iter().find(|topic| topic.1.id == "3984").unwrap();
        assert_eq!(topic.label, "parliamentary sitting");
        dbg!(unique_topics);
        Ok(())
    }
}
