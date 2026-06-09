#[cfg(test)]
mod tests {
    use combx::{
        document_text::{combine_texts, extract_text_from_documents_html_first},
        QuestionEntry,
    };
    use somai_client::SomAiClient;

    use somes_common_lib::Document;

    #[tokio::test]
    async fn test_parliamentary_question_extraction() {
        simple_logger::init_with_level(log::Level::Info).unwrap();
        let somai_client =
            SomAiClient::new("http://10.30.0.164:8000", "API_USER", "supersicher123!")
                .await
                .unwrap();

        let docs = vec![Document {
            title: Some("Anfrage".into()),
            document_url: Some(
                "https://www.parlament.gv.at/dokument/XXVII/J/14815/fnameorig_1550801.html".into(),
            ),
            document_type: Some("HTML".into()),
        }];
        let pg_pool = combx::connect_pg().await;
        let unique_topics = sqlx::query_scalar!(
            "select topic_name as topic from unique_eurovoc_topics order by topic"
        )
        .fetch_all(&pg_pool)
        .await
        .unwrap();

        let input = combine_texts(&extract_text_from_documents_html_first(&docs, None).await);

        let result = somai_client
            .make_llm_request_with_schema::<QuestionEntry>(
                input,
                format!(
                    r##"Bringe die folgende Anfrage in das angegebene JSON-Schema.
                    Klassifiziere das Thema anhand dieser Liste (max 10): {unique_topics:?}
                    Schreibe eine Zusammenfassung der Fragen in 'question_summary', und eine ausführlichere in 'detailed_question_summary'.
                    Die 'info' summaries sollen Zusammenfassungen über den Anlass der Fragestellung beinhalten.
                    Die verwendeten Quellen müssen in `info_sources_urls` verwendet werden.
                    Trage bei 'nth_question' immer die konkrete Identifikation ein.
                        z.B. 3. Wird Rust dominieren?
                                a) wenn ja, gehe kurz auf die Vorteile ein
                                b) wenn nein, erwähne kurz die Alternativen
                                    i) systems programming languages
                                    ii) OOP languages

                    Jede einzelne Frage oder Punkt wird anschließend in `questions' eingetragen.
                        z.B. Question {{ question: 'OOP languages', nth_level: [Number(1), String('a'), String('ii')] }}
                    "##
                ),
                None,
                false,
                Some(|schema: &mut serde_json::Value| {
                    // TODO: activate topic patching when #143 (https://github.com/som-es/somes/issues/143) is done
                    crate::ai_tasks::patch_topics("/properties/topics", schema, &unique_topics);
                }),
            )
            .await;
        dbg!(&result);
        assert!(result.is_ok());
    }
}
