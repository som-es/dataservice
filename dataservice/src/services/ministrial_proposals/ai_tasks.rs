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

#[cfg(test)]
mod tests {
    use combx::document_text::{combine_texts, extract_text_from_documents_html_first};
    use somai_client::SomAiClient;

    use crate::{ai_tasks::summary_prompt2, OCR_ENGINE};

    use super::*;

    #[tokio::test]
    async fn test_summary_prompt_xxviii_me_71() {
        let somai_client = SomAiClient::new_local("http://localhost:8000")
            .await
            .unwrap();
        /*
                   id     | ministrial_proposal_id |                        title                        | document_type |                 document_url
        -----------+------------------------+-----------------------------------------------------+---------------+-----------------------------------------------
         103549067 |               15561773 | Kurzinformation                                     | DigitalPDF    | /dokument/XXVIII/ME/71/imfname_1729116.pdf
         103549068 |               15561773 | Begleitschreiben                                    | DigitalPDF    | /dokument/XXVIII/ME/71/imfname_1728995.pdf
         103549069 |               15561773 | Gesetzestext                                        | DigitalPDF    | /dokument/XXVIII/ME/71/fname_1728991.pdf
         103549070 |               15561773 | Gesetzestext                                        | HTML          | /dokument/XXVIII/ME/71/fnameorig_1728991.html
         103549071 |               15561773 | Vorblatt und wirkungsorientierte Folgenabschätzung | DigitalPDF    | /dokument/XXVIII/ME/71/imfname_1728993.pdf
         103549072 |               15561773 | Erläuterungen                                      | DigitalPDF    | /dokument/XXVIII/ME/71/fname_1728992.pdf
         103549073 |               15561773 | Erläuterungen                                      | HTML          | /dokument/XXVIII/ME/71/fnameorig_1728992.html
         103549074 |               15561773 | Textgegenüberstellung                              | DigitalPDF    | /dokument/XXVIII/ME/71/fname_1728994.pdf
         103549075 |               15561773 | Textgegenüberstellung                              | HTML          | /dokument/XXVIII/ME/71/fnameorig_1728994.html
                 */

        let docs = vec![
            Document {
                title: Some("Kurzinformation".to_string()),
                document_type: Some("DigitalPDF".to_string()),
                document_url: Some("/dokument/XXVIII/ME/71/imfname_1729116.pdf".to_string()),
            },
            Document {
                title: Some("Begleitschreiben".to_string()),
                document_type: Some("DigitalPDF".to_string()),
                document_url: Some("/dokument/XXVIII/ME/71/imfname_1728995.pdf".to_string()),
            },
            Document {
                title: Some("Erläuterungen".to_string()),
                document_type: Some("HTML".to_string()),
                document_url: Some("/dokument/XXVIII/ME/71/fnameorig_1728992.html".to_string()),
            },
            Document {
                title: Some("Gesetzestext".to_string()),
                document_type: Some("HTML".to_string()),
                document_url: Some("/dokument/XXVIII/ME/71/fnameorig_1728991.html".to_string()),
            },
            Document {
                title: Some("Textgegenüberstellung".to_string()),
                document_type: Some("HTML".to_string()),
                document_url: Some("/dokument/XXVIII/ME/71/fnameorig_1728994.html".to_string()),
            },
            Document {
                title: Some("Vorblatt und wirkungsorientierte Folgenabschätzung".to_string()),
                document_type: Some("DigitalPDF".to_string()),
                document_url: Some("/dokument/XXVIII/ME/71/imfname_1728993.pdf".to_string()),
            },
            Document {
                title: Some("Textgegenüberstellung".to_string()),
                document_type: Some("DigitalPDF".to_string()),
                document_url: Some("/dokument/XXVIII/ME/71/fname_1728994.pdf".to_string()),
            },
            Document {
                title: Some("Erläuterungen".to_string()),
                document_type: Some("DigitalPDF".to_string()),
                document_url: Some("/dokument/XXVIII/ME/71/fname_1728992.pdf".to_string()),
            },
            Document {
                title: Some("Gesetzestext".to_string()),
                document_type: Some("DigitalPDF".to_string()),
                document_url: Some("/dokument/XXVIII/ME/71/fname_1728991.pdf".to_string()),
            },
        ];
        let pg_pool = combx::connect_pg().await;
        let unique_topics = sqlx::query_scalar!(
            "select topic_name as topic from unique_eurovoc_topics order by topic"
        )
        .fetch_all(&pg_pool)
        .await
        .unwrap();

        let input = combine_texts(&extract_text_from_documents_html_first(&docs, None).await);

        let result = summary_prompt2(&somai_client, input, &unique_topics, "Antrag/Gesetz").await;
        assert!(result.is_ok());
        dbg!(result.unwrap());
    }
}
