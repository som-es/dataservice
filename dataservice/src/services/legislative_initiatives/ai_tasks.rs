use combx::document_text::{combine_texts, extract_text_from_documents_html_first};
use somes_common_lib::Document;

use crate::{ai_task::TaskArgs, OCR_ENGINE};

pub async fn summarize_laws(task_args: TaskArgs, docs: &[Document]) -> combx::Result<()> {
    // is emphasis already generated/set? -> skip
    let info = sqlx::query!(
        "select ai_emphasis, is_emphasis_ai_generated from legislative_initiatives where id = $1",
        task_args.db_entry_id
    )
    .fetch_one(&task_args.pg_pool)
    .await?;
    if info.ai_emphasis.is_some() {
        log::info!(
            "Skipping legis init emphasis summarization for id {}, already set.",
            task_args.db_entry_id
        );
        return Ok(());
    }

    let inner_docs = docs.to_vec();
    match tokio::spawn(async move {
        combine_texts(&extract_text_from_documents_html_first(&inner_docs, OCR_ENGINE.clone()).await)
    })
    .await
    {
        Ok(input) => {
            if input.len() < 50 {
                log::error!(
            "Cannot summarize legis init {}, input is too short / empty for documents: {:?}",
            task_args.db_entry_id, docs
        );
                return Ok(());
            }
            let output = task_args.som_ai_client.summarize_input(input, None).await?;

            let find = "<|message|>";
            let idx = output.summary.rfind(find).unwrap_or_default();
            let summary = &output.summary[idx + find.len()..];
            sqlx::query!(
        "UPDATE legislative_initiatives SET ai_emphasis = $1, is_emphasis_ai_generated = true WHERE id = $2",
        summary,
        task_args.db_entry_id
    )
    .execute(&task_args.pg_pool)
    .await?;
        }
        Err(e) => log::error!("cannot extract text input from {:?} because of {e:?}", docs),
    }

    Ok(())
}
