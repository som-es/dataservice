use combx::DbAiSummary;
use sqlx::PgPool;
use std::collections::{HashMap, HashSet};

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

pub async fn all_unique_eurovoc_topics(pg: &PgPool) -> sqlx::Result<HashSet<String>> {
    sqlx::query_scalar::<_, String>(
        r#"
        select topic_name
        from unique_eurovoc_topics
        "#,
    )
    .fetch_all(pg)
    .await
    .map(|topics| topics.into_iter().collect())
}

pub async fn hallucinated_ai_topics_by_model(
    pg: &PgPool,
) -> sqlx::Result<Vec<(String, usize, HashSet<String>)>> {
    let topics_by_model = ai_topics_by_model(pg).await?;
    let unique_topics = all_unique_eurovoc_topics(pg).await?;

    let mut ranked = topics_by_model
        .into_iter()
        .map(|(model, topics)| {
            let hallucinated = topics
                .difference(&unique_topics)
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
pub async fn compare_ai_topics_with_unique_topics(pg: &PgPool) -> sqlx::Result<()> {
    let ranked = hallucinated_ai_topics_by_model(pg).await?;

    for (model, count, topics) in ranked {
        dbg!(model, count, topics);
    }

    Ok(())
}
