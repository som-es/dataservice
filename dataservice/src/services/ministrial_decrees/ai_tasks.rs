use combx::{DbAiSummary, Decree};
use somes_common_lib::Document;
use somes_scraper::language::Language;

use crate::{
    ai_task::TaskArgs,
    ai_tasks::{RisExtension, summarize_and_save},
};

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

    summarize_and_save::<Decree>(
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
        Language::De,
    )
    .await
}

impl RisExtension for Decree {
    fn extend_docs(
        pool: &sqlx::PgPool,
        entry_id: i32,
    ) -> impl Future<Output = sqlx::Result<Vec<Document>>> {
        let pool = pool.clone();
        async move {
            sqlx::query_as!(
                Document,
                "select title, document_type, document_url from ris_amendments ra inner join ministrial_decrees_documents mdd on mdd.ministrial_decree_id = norm_id where decree_id = $1",
                entry_id
            ).fetch_all(&pool).await
        }
    }
}

#[cfg(test)]
mod tests {
    use combx::{
        document_text::{combine_texts, extract_text_from_documents_html_first_stripped},
        with_data::unique_topics::unique_topics_for_lang,
    };
    use somai_client::SomAiClient;
    use somes_scraper::{AustriaScraper, decree::RisScraper};

    use crate::{OCR_ENGINE, ai_tasks::summary_prompt};

    use super::*;

    #[tokio::test]
    async fn test_summary_prompt_decree_bgbla_2025_ii_74() {
        let scraper = AustriaScraper::default();

        let decree = scraper
            .fetch_single_ris_entry("BGBLA_2026_II_261")
            // .fetch_single_ris_entry("BGBLA_2026_II_273")
            // .fetch_single_ris_entry("BGBLA_2025_II_74")
            .await
            .unwrap();

        dbg!(&decree);

        simple_logger::init_with_level(log::Level::Info).unwrap();
        let somai_client =
            SomAiClient::new("http://10.30.0.164:8000", "API_USER", "supersicher123!")
                .await
                .unwrap();

        let pg_pool = combx::connect_pg().await;
        let unique_topics = unique_topics_for_lang(&pg_pool, Language::De)
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

        let result = summary_prompt(
            &somai_client,
            input,
            &unique_topics,
            "Verordnung",
            Language::De,
        )
        .await;
        dbg!(result.unwrap());
    }

    #[tokio::test]
    async fn test_summary_prompt_decree_any_first() {
        let scraper = AustriaScraper::default();
        let decree = scraper.fetch_and_parse_ministrial_decrees(1).await.unwrap()[0].clone();

        dbg!(&decree);

        simple_logger::init_with_level(log::Level::Info).unwrap();
        let somai_client =
            SomAiClient::new("http://10.30.0.164:8000", "API_USER", "supersicher123!")
                .await
                .unwrap();

        let pg_pool = combx::connect_pg().await;
        let unique_topics = unique_topics_for_lang(&pg_pool, Language::De)
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

        let result = summary_prompt(
            &somai_client,
            input,
            &unique_topics,
            "Verordnung",
            Language::De,
        )
        .await;
        dbg!(result.unwrap());
        // println!("input: {}", &input[..std::cmp::min(5000, input.len())]);
    }
}
