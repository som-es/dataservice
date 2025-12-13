use crate::ai_task::TaskArgs;

pub async fn summarize_laws(task_args: TaskArgs) -> combx::Result<()> {
    // is emphasis already generated/set? -> skip
    let info = sqlx::query!(
        "select emphasis, is_emphasis_ai_generated from legislative_initiatives where id = $1",
        task_args.db_entry_id
    )
    .fetch_one(&task_args.pg_pool)
    .await?;
    if info.emphasis
    .is_some() 
    && !info.is_emphasis_ai_generated // remove this again!
    {
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

    let idx = output.summary.rfind("<|message|>").unwrap_or_default();
    let summary = &output.summary[idx..];
    sqlx::query!(
        "UPDATE legislative_initiatives SET emphasis = $1, is_emphasis_ai_generated = true WHERE id = $2",
        summary,
        task_args.db_entry_id
    )
    .execute(&task_args.pg_pool)
    .await?;
    Ok(())
}
