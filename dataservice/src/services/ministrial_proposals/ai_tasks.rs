use crate::{
    ai_task::TaskArgs,
    ai_tasks::{insert_ai_summary_sqlx, summarize_documents},
};
use somes_common_lib::Document;

pub async fn summarize_ministerial_proposal_documents(
    task_args: TaskArgs,
    docs: &[Document],
) -> combx::Result<()> {
    // is emphasis already generated/set? -> skip
    let info = sqlx::query!(
        "select id from ministerial_proposal_summaries where ministerial_proposal_id = $1 limit 1",
        task_args.db_entry_id
    )
    .fetch_optional(&task_args.pg_pool)
    .await?;
    if info.is_some() {
        log::info!(
            "Skipping ministerial proposal summarization for id {}, already set.",
            task_args.db_entry_id
        );
        return Ok(());
    }

    let summary = summarize_documents(&task_args, docs, "Ministerialentwurf").await?;
    if let Some(summary) = summary.summary {
        let summary_id = insert_ai_summary_sqlx(&task_args.pg_pool, &summary).await?;
        sqlx::query!("insert into ministerial_proposal_summaries (ministerial_proposal_id, summary_id) values ($1, $2)", task_args.db_entry_id, summary_id)
                    .execute(&task_args.pg_pool)
                    .await?;
    } else {
        log::error!(
            "No summary generated for ministerial proposal {}, documents: {:?}, abort reason: {:?}",
            task_args.db_entry_id,
            docs,
            summary.abort_reason
        );
    }

    Ok(())
}
