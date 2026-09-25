use chrono::Days;
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
            let gesetzesnummern = sqlx::query_scalar!(
                "select gesetzesnummer from ris_amendments ra inner join ministrial_decrees md on md.id = norm_id where decree_id = $1",
                entry_id
            ).fetch_all(&pool).await?;

            let Some(gesetzesnummer) = gesetzesnummern.first().cloned().flatten() else {
                return Ok(vec![]);
            };

            let date = sqlx::query_scalar!(
                "select publication_date from ministrial_decrees where id = $1",
                entry_id
            )
            .fetch_one(&pool)
            .await?
            .checked_sub_days(Days::new(1))
            .unwrap();

            let date = date.format("%Y-%m-%d").to_string();
            let url = format!(
                "https://www.ris.bka.gv.at/GeltendeFassung/Bundesnormen/{gesetzesnummer}/file.pdf?FassungVom={date}"
            );

            Ok(vec![Document {
                title: Some(format!(
                    "Fassung vom {date} (Erweitert durch das RIS – RIS-ANREICHERUNG WURDE AUFGERUFEN)"
                )),
                document_url: Some(url),
                document_type: Some("DigitalPDF".into()),
            }])
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
    async fn test_ris_doc_extension() {
        // https://www.ris.bka.gv.at/GeltendeFassung/Bundesnormen/20011636/file.pdf?FassungVom=2026-09-06
        let id = 1;
        let pg_pool = combx::connect_pg().await;
        let documents = Decree::extend_docs(&pg_pool, id).await.unwrap();
        dbg!(&documents);
        let input =
            combine_texts(&extract_text_from_documents_html_first_stripped(&documents, None).await);
        assert_eq!(input.len(), 134386);
        dbg!(input.len());
    }

    #[tokio::test]
    async fn test_summary_prompt_decree_bgbla_2025_ii_74() {
        let scraper = AustriaScraper::default();

        let decree = scraper
            .fetch_single_ris_entry("BGBLA_2025_II_74")
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
        let mut documents = decree
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

        let extend_docs = Decree::extend_docs(&pg_pool, 1).await;
        documents.extend(extend_docs.unwrap().into_iter());

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
