use combx::{AnswerEntry, QuestionEntry};
use somes_scraper::ParliamentaryQAEntity;
use sqlx::{PgPool, types::Json};

pub async fn insert_pqa(pg: &PgPool, pqa: &ParliamentaryQAEntity) -> sqlx::Result<i64> {
    let proposal = &pqa.base_info;

    let issuer_ids: Vec<i64> = proposal.issuer_ids.iter().map(|&id| id as i64).collect();
    let receiver_ids: Vec<i64> = proposal.receiver_ids.iter().map(|&id| id as i64).collect();

    let mut tx = pg.begin().await?;

    let rec = sqlx::query!(
        r#"
        insert into pqa_meta
            (gp, inr, ityp, title, description, raw_data_created_at, raw_data_updated_at,
             topics, eurovoc_topics, other_keyword_topics, issuer_ids, reciever_ids)
        values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        on conflict (gp, inr, ityp) do update set
            title = excluded.title,
            description = excluded.description,
            updated_at = now(),
            raw_data_updated_at = excluded.raw_data_updated_at,
            topics = excluded.topics,
            eurovoc_topics = excluded.eurovoc_topics,
            other_keyword_topics = excluded.other_keyword_topics,
            issuer_ids = excluded.issuer_ids,
            reciever_ids = excluded.reciever_ids
        returning id
        "#,
        proposal.gp,
        proposal.inr as i32,
        proposal.ityp,
        proposal.title,
        proposal.description.as_deref().unwrap_or(""),
        proposal.created_at,
        proposal.updated_at,
        &proposal.topics,
        &proposal.eurovoc_topics,
        &proposal.other_keyword_topics,
        &issuer_ids,
        &receiver_ids,
    )
    .fetch_one(&mut *tx)
    .await?;

    let pqa_meta_id = rec.id;

    sqlx::query!(
        "delete from pqa_documents where pqa_meta_id = $1",
        pqa_meta_id,
    )
    .execute(&mut *tx)
    .await?;

    for doc in &proposal.documents {
        sqlx::query!(
            r#"
            insert into pqa_documents (pqa_meta_id, title, document_type, document_url)
            values ($1, $2, $3, $4)
            on conflict (pqa_meta_id, document_url) do nothing
            "#,
            pqa_meta_id,
            doc.title,
            doc.ftype.to_string(),
            &doc.link,
        )
        .execute(&mut *tx)
        .await?;
    }

    for qa_ref in &pqa.references {
        sqlx::query!(
            r#"
            insert into pqa_references (pqa_meta_id, pqa_gp, pqa_inr, pqa_ityp)
            values ($1, $2, $3, $4)
            on conflict (pqa_gp, pqa_inr, pqa_ityp) do nothing
            "#,
            pqa_meta_id,
            qa_ref.reference.gp,
            qa_ref.reference.inr as i32,
            qa_ref.reference.ityp,
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    Ok(rec.id)
}

pub async fn insert_question_entry<'c, E>(
    executor: E,
    pqa_meta_id: i64,
    entry: &QuestionEntry,
    model_used: &str,
    version: &str,
) -> sqlx::Result<i64>
where
    E: sqlx::PgExecutor<'c>,
{
    let rec = sqlx::query!(
        r#"
        insert into pqa_questions (
            short_title,
            pqa_meta_id,
            once_sentence_short_info_summary,
            very_short_info_summary,
            short_info_summary,
            info_summary,
            detailed_info_summary,
            question_summary,
            detailed_question_summary,
            full_question_entry,
            model_used,
            version
        )
        values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        returning id
        "#,
        entry.short_title,
        pqa_meta_id,
        entry.once_sentence_short_info_summary,
        entry.very_short_info_summary,
        entry.short_info_summary,
        entry.info_summary,
        entry.detailed_info_summary,
        entry.question_summary,
        entry.detailed_question_summary,
        Json(entry) as Json<&QuestionEntry>,
        model_used,
        version,
    )
    .fetch_one(executor)
    .await?;

    Ok(rec.id)
}

pub async fn insert_answer_entry<'c, E>(
    executor: E,
    pqa_meta_id: i64,
    entry: &AnswerEntry,
    model_used: &str,
    version: &str,
) -> sqlx::Result<i64>
where
    E: sqlx::PgExecutor<'c>,
{
    let rec = sqlx::query!(
        r#"
        insert into pqa_answers (
            pqa_meta_id,
            short_title,
            full_answer_summary,
            detailed_full_answer_summary,
            full_answer_entry,
            model_used,
            version
        )
        values ($1, $2, $3, $4, $5, $6, $7)
        returning id
        "#,
        pqa_meta_id,
        entry.short_title,
        entry.full_answer_summary,
        entry.detailed_full_answer_summary,
        Json(entry) as Json<&AnswerEntry>,
        model_used,
        version,
    )
    .fetch_one(executor)
    .await?;

    Ok(rec.id)
}
