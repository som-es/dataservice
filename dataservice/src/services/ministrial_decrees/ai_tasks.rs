use combx::DbAiSummary;
use somes_common_lib::Document;

use crate::{ai_task::TaskArgs, ai_tasks::summarize_and_save};

pub async fn summarize_decrees_documents(
    task_args: TaskArgs,
    docs: &[Document],
) -> combx::Result<Option<DbAiSummary>> {
    // is emphasis already generated/set? -> skip
    let info = sqlx::query!(
        "select id from decree_summaries where decree_id = $1 limit 1",
        task_args.db_entry_id
    )
    .fetch_optional(&task_args.pg_pool)
    .await?;
    if info.is_some() {
        log::info!(
            "Skipping decree summarization for id {}, already set.",
            task_args.db_entry_id
        );
        return Ok(None);
    }

    summarize_and_save(
        task_args,
        docs,
        "Verordnung",
        |task_args: TaskArgs, summary_id: i32| async move {
            sqlx::query!(
                "insert into decree_summaries (decree_id, summary_id) values ($1, $2)",
                task_args.db_entry_id,
                summary_id
            )
            .execute(&task_args.pg_pool)
            .await?;
            Ok(())
        },
    )
    .await
}

#[cfg(test)]
mod tests {
    use combx::document_text::{combine_texts, extract_text_from_documents_html_first_stripped};
    use somai_client::SomAiClient;
    use somes_scraper::{decree::RisScraper, AustriaScraper};

    use crate::{ai_tasks::summary_prompt2, OCR_ENGINE};

    use super::*;

    #[tokio::test]
    async fn test_summary_prompt_xxviii_me_49() {
        let scraper = AustriaScraper::default();
        let decree = scraper.fetch_and_parse_ministrial_decrees(1).await.unwrap()[0].clone();

        dbg!(&decree);
        let somai_client = SomAiClient::new_local("http://localhost:8000")
            .await
            .unwrap();
        let pg_pool = combx::connect_pg().await;
        let unique_topics = sqlx::query_scalar!(
            "select topic_name as topic from unique_eurovoc_topics order by topic"
        )
        .fetch_all(&pg_pool)
        .await
        .unwrap();
        let documents = decree
            .documents
            .iter()
            .map(|doc| Document {
                title: Some(doc.title.clone()),
                document_url: Some(doc.link.clone()),
                document_type: Some(doc.ftype.to_string()),
            })
            .collect::<Vec<_>>();

        let input =
            combine_texts(&extract_text_from_documents_html_first_stripped(&documents, None).await);
        println!("input length: {}", input.len());

        let result =
            summary_prompt2(&somai_client, input, &unique_topics, "Ministerialentwurf").await;
        dbg!(result.unwrap());
        // println!("input: {}", &input[..std::cmp::min(5000, input.len())]);
    }
}
