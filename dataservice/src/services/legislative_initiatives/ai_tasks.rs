use crate::ai_task::TaskArgs;

pub async fn summarize_laws(task_args: TaskArgs) -> combx::Result<()> {
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
    let output = task_args
        .som_ai_client
        .summarize_input(task_args.ai_task_input, None)
        .await?;

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
    Ok(())
}
