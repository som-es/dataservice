use clap::ValueEnum;
use combx::DbAiSummary;
use sqlx::PgPool;
use std::collections::{HashMap, HashSet};

/// Which set of "used topics" (the reference vocabulary the AI topics are compared
/// against) to load from the database.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum TopicSource {
    /// `eurovoc_topics_legis_init` — the canonical Eurovoc vocabulary.
    Eurovoc,
    /// `topics_legis_init` — the free-form (non-Eurovoc) topics.
    LegisInit,
}

impl TopicSource {
    /// Human-readable label for the selected source.
    #[allow(dead_code)]
    pub fn label(self) -> &'static str {
        match self {
            TopicSource::Eurovoc => "Eurovoc",
            TopicSource::LegisInit => "Legis-init",
        }
    }
}

#[allow(dead_code)]
pub async fn all_unique_ai_topics(pg: &PgPool) -> sqlx::Result<HashSet<String>> {
    let summaries = sqlx::query_as::<_, DbAiSummary>(
        r#"
        select
            id,
            complexity_scope_of_proposal,
            detailed_summary,
            generated_at,
            version,
            model_used,
            short_title,
            short_summary,
            full_summary
        from summaries
        "#,
    )
    .fetch_all(pg)
    .await?;

    Ok(summaries
        .into_iter()
        .flat_map(|row| row.full_summary.0.topics)
        .collect())
}

pub async fn ai_topics_by_model(pg: &PgPool) -> sqlx::Result<HashMap<String, HashSet<String>>> {
    let summaries = sqlx::query_as::<_, DbAiSummary>(
        r#"
        select
            id,
            complexity_scope_of_proposal,
            detailed_summary,
            generated_at,
            version,
            model_used,
            short_title,
            short_summary,
            full_summary
        from summaries
        "#,
    )
    .fetch_all(pg)
    .await?;

    let mut topics_by_model = HashMap::<String, HashSet<String>>::new();

    for row in summaries {
        let model = row.model_used;

        topics_by_model
            .entry(model)
            .or_default()
            .extend(row.full_summary.0.topics);
    }

    Ok(topics_by_model)
}

/// Distinct "used topics" for the given [`TopicSource`] table.
pub async fn all_used_topics(pg: &PgPool, source: TopicSource) -> sqlx::Result<HashSet<String>> {
    match source {
        TopicSource::Eurovoc => {
            sqlx::query_scalar!(
                r#"
                select distinct topic from eurovoc_topics_legis_init
                "#,
            )
            .fetch_all(pg)
            .await
            .map(|topics| topics.into_iter().collect())
        }
        TopicSource::LegisInit => {
            sqlx::query_scalar!(
                r#"
                select distinct topic from topics_legis_init
                "#,
            )
            .fetch_all(pg)
            .await
            .map(|topics| topics.into_iter().collect())
        }
    }
}

/// Distinct topics from all known [`TopicSource`] tables combined.
pub async fn all_used_topics_combined(pg: &PgPool) -> sqlx::Result<HashSet<String>> {
    let mut topics = all_used_topics(pg, TopicSource::Eurovoc).await?;
    topics.extend(all_used_topics(pg, TopicSource::LegisInit).await?);
    Ok(topics)
}

pub async fn hallucinated_ai_topics_by_model(
    pg: &PgPool,
    source: TopicSource,
) -> sqlx::Result<Vec<(String, usize, HashSet<String>)>> {
    let topics_by_model = ai_topics_by_model(pg).await?;
    let used_topics = all_used_topics(pg, source).await?;

    let mut ranked = topics_by_model
        .into_iter()
        .map(|(model, topics)| {
            let hallucinated = topics
                .difference(&used_topics)
                .cloned()
                .collect::<HashSet<_>>();

            let count = hallucinated.len();

            (model, count, hallucinated)
        })
        .collect::<Vec<_>>();

    ranked.sort_by(|a, b| b.1.cmp(&a.1));

    Ok(ranked)
}

#[allow(dead_code)]
pub async fn compare_ai_topics_with_used_topics(
    pg: &PgPool,
    source: TopicSource,
) -> sqlx::Result<()> {
    let ranked = hallucinated_ai_topics_by_model(pg, source).await?;

    for (model, count, topics) in ranked {
        dbg!(model, count, topics);
    }

    Ok(())
}
