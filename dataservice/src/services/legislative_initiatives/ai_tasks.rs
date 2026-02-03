use combx::DbAiSummary;
use somes_common_lib::Document;

use crate::{ai_task::TaskArgs, ai_tasks::summarize_and_save};

pub async fn summarize_legis_init_documents(
    task_args: TaskArgs,
    docs: &[Document],
    summarization_of: &str,
) -> combx::Result<Option<DbAiSummary>> {
    // is emphasis already generated/set? -> skip
    let info = sqlx::query!(
        "select id from legislative_initiative_summaries where legis_init_id = $1 limit 1",
        task_args.db_entry_id
    )
    .fetch_optional(&task_args.pg_pool)
    .await?;
    if info.is_some() {
        log::info!(
            "Skipping legis init emphasis summarization for id {}, already set.",
            task_args.db_entry_id
        );
        return Ok(None);
    }

    summarize_and_save(task_args, docs, summarization_of, |task_args: TaskArgs, summary_id: i32| async move {
        sqlx::query!("insert into legislative_initiative_summaries (legis_init_id, summary_id) values ($1, $2)", task_args.db_entry_id, summary_id)
            .execute(&task_args.pg_pool)
            .await?;
        Ok(())
    }).await
}

#[cfg(test)]
mod tests {
    use combx::document_text::{combine_texts, extract_text_from_documents_html_first};
    use somai_client::SomAiClient;

    use crate::{ai_tasks::summary_prompt2, OCR_ENGINE};

    use super::*;

    #[tokio::test]
    async fn test_summary_prompt_xxviii_i_298() {
        let somai_client = SomAiClient::new_local("http://localhost:8000")
            .await
            .unwrap();
        /*
               -----------+----------------------------+------------------------+---------------+-----------------------------------------------
        291810000 |                   94523539 | Gesetzestext           | DigitalPDF    | /dokument/XXVIII/I/298/fname_1722547.pdf
        291810002 |                   94523539 | Erläuterungen         | DigitalPDF    | /dokument/XXVIII/I/298/fname_1722549.pdf
        291810001 |                   94523539 | Gesetzestext           | HTML          | /dokument/XXVIII/I/298/fnameorig_1722547.html
        291810003 |                   94523539 | Erläuterungen         | HTML          | /dokument/XXVIII/I/298/fnameorig_1722549.html
        291810005 |                   94523539 | Textgegenüberstellung | DigitalPDF    | /dokument/XXVIII/I/298/imfname_1722550.pdf
        291810004 |                   94523539 | Vorblatt und WFA       | DigitalPDF    | /dokument/XXVIII/I/298/imfname_1722551.pdf*/
        let docs = vec![
            Document {
                title: Some("Gesetzestext".into()),
                document_url: Some(
                    "https://parlament.gv.at/dokument/XXVIII/I/298/fname_1722547.pdf".into(),
                ),
                document_type: Some("DigitalPDF".into()),
            },
            Document {
                title: Some("Erläuterungen".into()),
                document_url: Some(
                    "https://parlament.gv.at/dokument/XXVIII/I/298/fname_1722549.pdf".into(),
                ),
                document_type: Some("DigitalPDF".into()),
            },
            Document {
                title: Some("Gesetzestext".into()),
                document_url: Some(
                    "https://parlament.gv.at/dokument/XXVIII/I/298/fnameorig_1722547.html".into(),
                ),
                document_type: Some("HTML".into()),
            },
            Document {
                title: Some("Erläuterungen".into()),
                document_url: Some(
                    "https://parlament.gv.at/dokument/XXVIII/I/298/fnameorig_1722549.html".into(),
                ),
                document_type: Some("HTML".into()),
            },
            Document {
                title: Some("Textgegenüberstellung".into()),
                document_url: Some(
                    "https://parlament.gv.at/dokument/XXVIII/I/298/imfname_1722550.pdf".into(),
                ),
                document_type: Some("DigitalPDF".into()),
            },
            Document {
                title: Some("Vorblatt und WFA".into()),
                document_url: Some(
                    "https://parlament.gv.at/dokument/XXVIII/I/298/imfname_1722551.pdf".into(),
                ),
                document_type: Some("DigitalPDF".into()),
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

    #[tokio::test]
    async fn test_summary_prompt_xxviii_i_308() {
        let somai_client = SomAiClient::new_local("http://localhost:8000")
            .await
            .unwrap();

        //                 id     | legislative_initiatives_id |         title          | document_type |                 document_url
        // -----------+----------------------------+------------------------+---------------+-----------------------------------------------
        //  291804716 |                   95729218 | Gesetzestext           | DigitalPDF    | /dokument/XXVIII/I/308/fname_1723244.pdf
        //  291804720 |                   95729218 | Erläuterungen         | DigitalPDF    | /dokument/XXVIII/I/308/fname_1723246.pdf
        //  291804717 |                   95729218 | Gesetzestext           | HTML          | /dokument/XXVIII/I/308/fnameorig_1723244.html
        //  291804721 |                   95729218 | Erläuterungen         | HTML          | /dokument/XXVIII/I/308/fnameorig_1723246.html
        //  291804723 |                   95729218 | Textgegenüberstellung | DigitalPDF    | /dokument/XXVIII/I/308/imfname_1723247.pdf
        //  291804722 |                   95729218 | Vorblatt und WFA       | DigitalPDF    | /dokument/XXVIII/I/308/imfname_1723248.pdf
        //  291804718 |                   95729218 | Anlage 1 zum NISG 2026 | DigitalPDF    | /dokument/XXVIII/I/308/imfname_1723249.pdf
        //  291804719 |                   95729218 | Anlage 2 zum NISG 2026 | DigitalPDF    | /dokument/XXVIII/I/308/imfname_1723250.pdf
        let docs = vec![
            Document {
                title: Some("Gesetzestext".into()),
                document_url: Some(
                    "https://parlament.gv.at/dokument/XXVIII/I/308/fname_1723244.pdf".into(),
                ),
                document_type: Some("DigitalPDF".into()),
            },
            Document {
                title: Some("Erläuterungen".into()),
                document_url: Some(
                    "https://parlament.gv.at/dokument/XXVIII/I/308/fname_1723246.pdf".into(),
                ),
                document_type: Some("DigitalPDF".into()),
            },
            Document {
                title: Some("Gesetzestext".into()),
                document_url: Some(
                    "https://parlament.gv.at/dokument/XXVIII/I/308/fnameorig_1723244.html".into(),
                ),
                document_type: Some("HTML".into()),
            },
            Document {
                title: Some("Erläuterungen".into()),
                document_url: Some(
                    "https://parlament.gv.at/dokument/XXVIII/I/308/fnameorig_1723246.html".into(),
                ),
                document_type: Some("HTML".into()),
            },
            Document {
                title: Some("Textgegenüberstellung".into()),
                document_url: Some(
                    "https://parlament.gv.at/dokument/XXVIII/I/308/imfname_1723247.pdf".into(),
                ),
                document_type: Some("DigitalPDF".into()),
            },
            Document {
                title: Some("Vorblatt und WFA".into()),
                document_url: Some(
                    "https://parlament.gv.at/dokument/XXVIII/I/308/imfname_1723248.pdf".into(),
                ),
                document_type: Some("DigitalPDF".into()),
            },
            Document {
                title: Some("Anlage 1 zum NISG 2026".into()),
                document_url: Some(
                    "https://parlament.gv.at/dokument/XXVIII/I/308/imfname_1723249.pdf".into(),
                ),
                document_type: Some("DigitalPDF".into()),
            },
            Document {
                title: Some("Anlage 2 zum NISG 2026".into()),
                document_url: Some(
                    "https://parlament.gv.at/dokument/XXVIII/I/308/imfname_1723250.pdf".into(),
                ),
                document_type: Some("DigitalPDF".into()),
            },
        ];
        let pg_pool = combx::connect_pg().await;
        let unique_topics = sqlx::query_scalar!(
            "select topic_name as topic from unique_eurovoc_topics order by topic"
        )
        .fetch_all(&pg_pool)
        .await
        .unwrap();
        let input =
            combine_texts(&extract_text_from_documents_html_first(&docs, OCR_ENGINE.clone()).await);

        let result = summary_prompt2(&somai_client, input, &unique_topics, "Antrag/Gesetz").await;
        assert!(result.is_ok());
        dbg!(result.unwrap());
    }

    #[tokio::test]
    async fn test_summary_prompt_xxviii_i_269() {
        let somai_client = SomAiClient::new_local("http://localhost:8000")
            .await
            .unwrap();

        let docs = [
            Document {
                title: Some("Gesetzestext".into()),
                document_url: Some(
                    "https://parlament.gv.at/dokument/XXVIII/I/269/fname_1718387.pdf".into(),
                ),
                document_type: Some("DigitalPDF".into()),
            },
            Document {
                title: Some("Erläuterungen".into()),
                document_url: Some(
                    "https://parlament.gv.at/dokument/XXVIII/I/269/fname_1718389.pdf".into(),
                ),
                document_type: Some("DigitalPDF".into()),
            },
            Document {
                title: Some("Textgegenüberstellung".into()),
                document_url: Some(
                    "https://parlament.gv.at/dokument/XXVIII/I/269/fname_1718390.pdf".into(),
                ),
                document_type: Some("DigitalPDF".into()),
            },
            Document {
                title: Some("Gesetzestext".into()),
                document_url: Some(
                    "https://parlament.gv.at/dokument/XXVIII/I/269/fnameorig_1718387.html".into(),
                ),
                document_type: Some("HTML".into()),
            },
            Document {
                title: Some("Erläuterungen".into()),
                document_url: Some(
                    "https://parlament.gv.at/dokument/XXVIII/I/269/fnameorig_1718389.html".into(),
                ),
                document_type: Some("HTML".into()),
            },
            Document {
                title: Some("Textgegenüberstellung".into()),
                document_url: Some(
                    "https://parlament.gv.at/dokument/XXVIII/I/269/fnameorig_1718390.html".into(),
                ),
                document_type: Some("HTML".into()),
            },
            Document {
                title: Some("Vorblatt und WFA".into()),
                document_url: Some(
                    "https://parlament.gv.at/dokument/XXVIII/I/269/imfname_1718391.pdf".into(),
                ),
                document_type: Some("DigitalPDF".into()),
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
    // XXVIII/A/568
    // -----------+----------------------------+------------------------------------------------------------+---------------+--------------------------------------------
    // 291805810 |                   95143976 | Selbständiger Entschließungsantrag (gescanntes Original) | OriginalPDF   | /dokument/XXVIII/A/568/imfname_1722828.pdf
    #[tokio::test]
    async fn test_summary_prompt_xxviii_a_568() {
        let somai_client = SomAiClient::new_local("http://localhost:8000")
            .await
            .unwrap();

        let docs = [Document {
            title: Some("Selbständiger Entschließungsantrag (gescanntes Original)".into()),
            document_url: Some(
                "https://parlament.gv.at/dokument/XXVIII/A/568/imfname_1722828.pdf".into(),
            ),
            document_type: Some("OriginalPDF".into()),
        }];

        let pg_pool = combx::connect_pg().await;
        let unique_topics = sqlx::query_scalar!(
            "select topic_name as topic from unique_eurovoc_topics order by topic"
        )
        .fetch_all(&pg_pool)
        .await
        .unwrap();

        let input =
            combine_texts(&extract_text_from_documents_html_first(&docs, OCR_ENGINE.clone()).await);
        let result =
            summary_prompt2(&somai_client, input, &unique_topics, "Entschließungsantrag").await;
        assert!(result.is_ok());
        dbg!(result.unwrap());
    }

    #[tokio::test]
    async fn test_summary_prompt_xxviii_bi_28() {
        let somai_client = SomAiClient::new_local("http://localhost:8000")
            .await
            .unwrap();

        let docs = [Document {
            title: Some("Bürgerinitiative (gescanntes Original)".into()),
            document_url: Some(
                "https://parlament.gv.at/dokument/XXVIII/BI/28/imfname_1728360.pdf".into(),
            ),
            document_type: Some("OriginalPDF".into()),
        }];

        let pg_pool = combx::connect_pg().await;
        let unique_topics = sqlx::query_scalar!(
            "select topic_name as topic from unique_eurovoc_topics order by topic"
        )
        .fetch_all(&pg_pool)
        .await
        .unwrap();

        let input =
            combine_texts(&extract_text_from_documents_html_first(&docs, OCR_ENGINE.clone()).await);
        // let result = summary_prompt2(&somai_client, input, &unique_topics).await;
        // assert!(result.is_ok());
        // dbg!(result.unwrap());
    }

    #[tokio::test]
    async fn test_summary_prompt_xxviii_aa_47() {
        let somai_client = SomAiClient::new_local("http://localhost:8000")
            .await
            .unwrap();

        /*
                   id     | legislative_initiatives_id |                  title                   | document_type |                             document_url
        -----------+----------------------------+------------------------------------------+---------------+-----------------------------------------------------------------------
         300238067 |                  101032523 | Abänderungsantrag (gescanntes Original) | OriginalPDF   | https://www.parlament.gv.at/dokument/XXVIII/AA/47/imfname_1730548.pdf
                 */
        let docs = [Document {
            title: Some("Abänderungsantrag (gescanntes Original)".into()),
            document_url: Some(
                "https://www.parlament.gv.at/dokument/XXVIII/AA/47/imfname_1730548.pdf".into(),
            ),
            document_type: Some("OriginalPDF".into()),
        }];
        let pg_pool = combx::connect_pg().await;

        let unique_topics = sqlx::query_scalar!(
            "select topic_name as topic from unique_eurovoc_topics order by topic"
        )
        .fetch_all(&pg_pool)
        .await
        .unwrap();
        let input =
            combine_texts(&extract_text_from_documents_html_first(&docs, OCR_ENGINE.clone()).await);
        let result =
            summary_prompt2(&somai_client, input, &unique_topics, "Abänderungsantrag").await;
        assert!(result.is_ok());
        dbg!(result.unwrap());
    }
}
