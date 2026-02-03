mod summarization;
use combx::{AiSummary, DbAiSummary};
use somes_common_lib::Document;
use sqlx::types::Json;
pub use summarization::*;

use crate::ai_task::TaskArgs;

pub(crate) async fn summarize_and_save(
    task_args: TaskArgs,
    docs: &[Document],
    summarization_of: &str,
    summariation_of_save_cb: impl AsyncFnOnce(TaskArgs, i32) -> combx::Result<()>,
) -> combx::Result<Option<DbAiSummary>> {
    let summary = summarize_documents(&task_args, docs, summarization_of).await?;
    if let Some(summary) = summary.summary {
        let summary_id = insert_ai_summary_sqlx(&task_args.pg_pool, &summary).await?;

        let db_ai_summary = sqlx::query_as!(
            DbAiSummary,
            r#"select 
                id,
        full_summary as "full_summary: Json<AiSummary>",
        short_title,
        short_summary,
        detailed_summary,
        complexity_scope_of_proposal,
        model_used,
        version,
        generated_at from summaries where id = $1"#,
            summary_id
        )
        .fetch_one(&task_args.pg_pool)
        .await?;

        summariation_of_save_cb(task_args, summary_id).await?;

        Ok(Some(db_ai_summary))
    } else {
        log::error!(
            "No summary generated for {summarization_of} {}, documents: {:?}, abort reason: {:?}",
            task_args.db_entry_id,
            docs,
            summary.abort_reason
        );
        Ok(None)
    }
}
