use sqlx::{Executor, Postgres, Transaction};

pub async fn generate_delegate_introduction(
    delegate_id: i32,
    somai_client: &somai_client::SomAiClient,
) -> combx::Result<Vec<somai_client::QAItem>> {
    let data = somai_client
        .extract_delegate_preview_video_qa(delegate_id)
        .await?;
    Ok(data.qa)
}

pub async fn insert_delegate_qa<'a>(
    transcription_id: i32,
    qa: &[somai_client::QAItem],
    tx: &mut Transaction<'a, Postgres>,
) -> sqlx::Result<()> {
    for item in qa {
        sqlx::query!(
            "INSERT INTO transcriptionsQA (transcription_id, question, answer) VALUES ($1, $2, $3)",
            transcription_id,
            item.question,
            item.answer
        )
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

pub(crate) async fn delegate_introduction_id_sqlx<'a>(
    exec: impl Executor<'a, Database = Postgres>,
    delegate_id: i32,
) -> sqlx::Result<Option<i32>> {
    let id = sqlx::query_scalar!(
        "SELECT id FROM introduction_transcriptions WHERE delegate_id = $1",
        delegate_id
    )
    .fetch_optional(exec)
    .await?;
    Ok(id)
}

pub async fn insert_delegate_introduction<'a>(
    delegate_id: i32,
    qa: &[somai_client::QAItem],
    tx: &mut Transaction<'a, Postgres>,
) -> sqlx::Result<()> {
    let id = sqlx::query_scalar!(
        "SELECT id FROM introduction_transcriptions WHERE delegate_id = $1",
        delegate_id
    )
    .fetch_optional(&mut **tx)
    .await?;

    let id =
        match id {
            Some(id) => {
                return Ok(());
                /*sqlx::query!(
                    "DELETE FROM transcriptionsQA WHERE transcription_id = $1",
                    id
                )
                .execute(&mut **tx)
                .await?;
                id*/
            }
            None => sqlx::query_scalar!(
                "INSERT INTO introduction_transcriptions (delegate_id) VALUES ($1) RETURNING id",
                delegate_id
            )
            .fetch_one(&mut **tx)
            .await?,
        };

    insert_delegate_qa(id, qa, tx).await?;

    Ok(())
}
