use std::collections::HashSet;

use chrono::{DateTime, Utc};
use combx::Parliament;
use qdrant_client::Qdrant;
use qdrant_client::qdrant::{GetPoints, PointId};
use somai_client::SomAiClient;
use somes_scraper::language::Language;

use crate::{
    Result,
    ai_task::TaskArgs,
    political_analysis::stance_system::{Citation, Stance, StanceDetection, rag_pipeline},
};

use super::config::update_delta;

pub async fn recently_answered_pairs(
    pg_pool: &sqlx::PgPool,
    today: DateTime<Utc>,
) -> Result<HashSet<(i32, i32)>> {
    let answers = sqlx::query!(
        r#"
        SELECT delegate_id, question_id
        FROM political_answers
        WHERE delegate_id IS NOT NULL
            AND is_strong_reference IS NOT TRUE
            AND created_at > $1
        "#,
        today - update_delta()
    )
    .fetch_all(pg_pool)
    .await?;

    Ok(answers
        .iter()
        .filter_map(|answer| Some((answer.delegate_id?, answer.question_id)))
        .collect())
}

pub async fn process_delegate_stance(
    task_args: &TaskArgs,
    ai_client: &SomAiClient,
    qdrant_client: &Qdrant,
    delegate_id: i32,
    question_id: i32,
    question_text: &str,
    used_llm: &str,
    language: Language,
    parliament: Parliament,
) -> Result<()> {
    let stance = rag_pipeline::<Stance>(
        ai_client,
        qdrant_client,
        delegate_id,
        question_text,
        language,
        parliament,
    )
    .await?;

    let stance_llm = match stance.stance_detection {
        StanceDetection::Postive => "Positive",
        StanceDetection::Neutral => "Neutral",
        StanceDetection::Negative => "Negative",
    };

    let answer_id = sqlx::query_scalar!(
        r#"
        INSERT INTO political_answers(question_id, delegate_id, answer, stance_llm, model_used, full_stance)
        VALUES ($1, $2, $3, $4, $5, $6) RETURNING id
        "#,
        question_id,
        delegate_id,
        stance.political_stance,
        stance_llm,
        used_llm,
        serde_json::to_value(&stance)?,
    )
    .fetch_one(&task_args.pg_pool)
    .await?;

    save_citations(
        &task_args.pg_pool,
        qdrant_client,
        answer_id,
        &stance.citations,
    )
    .await?;

    evaluate_and_save_opinion(
        &task_args.pg_pool,
        ai_client,
        delegate_id,
        question_id,
        answer_id,
        stance_llm,
        &stance.political_stance,
    )
    .await?;

    Ok(())
}

async fn save_citations(
    pg_pool: &sqlx::PgPool,
    qdrant_client: &Qdrant,
    answer_id: i32,
    citations: &[Citation],
) -> Result<()> {
    for citation in citations {
        let point_id = PointId::from(citation.point_uuid.clone());

        let get_points_req = GetPoints {
            collection_name: "delegates".to_string(),
            ids: vec![point_id],
            with_payload: Some(true.into()),
            ..Default::default()
        };

        let qdrant_res = qdrant_client.get_points(get_points_req).await?;

        let mut text_snippet: Option<String> = None;

        if let Some(point) = qdrant_res.result.first()
            && let Some(text_value) = point.payload.get("text")
            && let Some(qdrant_client::qdrant::value::Kind::StringValue(text)) = &text_value.kind
        {
            text_snippet = Some(text.clone());
        }

        let Ok(parsed_uuid) = uuid::Uuid::parse_str(&citation.point_uuid) else {
            log::warn!("Skipping citation insertion because of malformed uuid: {citation:?}");
            continue;
        };

        sqlx::query!(
            r#"
            INSERT INTO stance_citations
                (answer_id, speech_id, point_uuid, text_snippet)
            VALUES
                ($1, $2, $3, $4)
            ON CONFLICT (answer_id, point_uuid) DO NOTHING
            "#,
            answer_id,
            citation.speech_id,
            parsed_uuid,
            text_snippet
        )
        .execute(pg_pool)
        .await?;
    }

    Ok(())
}

async fn evaluate_and_save_opinion(
    pg_pool: &sqlx::PgPool,
    ai_client: &SomAiClient,
    delegate_id: i32,
    question_id: i32,
    answer_id: i32,
    stance_llm: &str,
    political_stance: &str,
) -> Result<()> {
    let ref_answers = sqlx::query!(
        r#"
        SELECT stance_llm, answer, question_id
        FROM political_answers
        WHERE question_id = $1 AND is_strong_reference
        ORDER BY stance_llm;
        "#,
        question_id
    )
    .fetch_all(pg_pool)
    .await?;

    let negative_ref = &ref_answers[0].answer;
    let positive_ref = &ref_answers[1].answer;

    let pro_sim = ai_client
        .embedding_embed_and_cosine(positive_ref.to_string(), political_stance.to_string())
        .await?
        .cosine_similarity;

    let contra_sim = ai_client
        .embedding_embed_and_cosine(negative_ref.to_string(), political_stance.to_string())
        .await?
        .cosine_similarity;

    let score = 10.0 * (pro_sim - contra_sim).powi(2);

    sqlx::query!(
        r#"
        INSERT INTO political_opinions
        (delegate_id, question_id, answer_id, stance, pro_strong_ref_score, contra_strong_ref_score, ref_score)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
        delegate_id,
        question_id,
        answer_id,
        stance_llm,
        pro_sim,
        contra_sim,
        score
    )
    .execute(pg_pool)
    .await?;

    Ok(())
}
