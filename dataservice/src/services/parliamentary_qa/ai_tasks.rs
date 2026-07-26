use combx::{AbortReason, DbAiInquiry};
use combx::{
    AnswerEntryOutput, QuestionEntryOutput,
    document_text::{combine_texts, extract_text_from_documents_html_first},
};
use somai_client::SomAiClient;
use somes_common_lib::Document;
use somes_scraper::ParliamentaryQAEntity;
use sqlx::types::Json;

use crate::{
    OCR_ENGINE,
    ai_task::TaskArgs,
    parliamentary_qa::{insert_answer_entry, insert_question_entry},
};

pub async fn summarize_and_extract_qa(
    task_args: TaskArgs,
    question: &ParliamentaryQAEntity,
    question_id: i64,
    answers: &[(i64, ParliamentaryQAEntity)],
) -> crate::Result<()> {
    use combx::QuestionEntry;
    let question_entry = sqlx::query_as!(
        DbAiInquiry,
        r#"
            select
                id,
                pqa_meta_id,
                short_title,
                once_sentence_short_info_summary,
                very_short_info_summary,
                short_info_summary,
                info_summary,
                detailed_info_summary,
                question_summary,
                detailed_question_summary,
                full_question_entry as "full_question_entry: Json<QuestionEntry>",
                model_used,
                version,
                generated_at
            from pqa_questions
            where pqa_meta_id = $1
                            "#,
        question_id
    )
    .fetch_optional(&task_args.pg_pool)
    .await?;

    let unique_topics =
        sqlx::query_scalar!("select topic_name as topic from unique_eurovoc_topics order by topic")
            .fetch_all(&task_args.pg_pool)
            .await?;

    let questions = match question_entry {
        Some(entry) => entry.full_question_entry.questions.clone(),
        None => {
            let extracted_question = extract_question(
                &task_args.som_ai_client,
                &unique_topics,
                &question
                    .documents
                    .iter()
                    .map(|doc| somes_common_lib::Document {
                        title: Some(doc.title.clone()),
                        document_url: Some(doc.link.clone()),
                        document_type: Some(doc.ftype.to_string()),
                    })
                    .collect::<Vec<_>>(),
            )
            .await?;
            if extracted_question.abort_reason != AbortReason::None
                || extracted_question.complete_inquiry_extraction.is_none()
            {
                log::error!(
                    "(question) No documents? provided in context for question {}, documents: {:?}, abort reason: {:?}, skipping",
                    question_id,
                    question.documents,
                    extracted_question.abort_reason
                );
                Err(combx::Error::Custom(
                    "No documents provided in context for question, skipping".to_string(),
                ))?;
            }
            let extracted_question = extracted_question.complete_inquiry_extraction.unwrap();

            insert_question_entry(
                &task_args.pg_pool,
                question_id,
                &extracted_question,
                &task_args.som_ai_client.get_model().await?.result,
                "1",
            )
            .await?;
            extracted_question.questions
        }
    };

    let mut extracted_answers = vec![];
    for (answer_id, answer) in answers {
        if sqlx::query_scalar!(
            "select id from pqa_answers where pqa_meta_id = $1",
            answer_id
        )
        .fetch_optional(&task_args.pg_pool)
        .await?
        .is_some()
        {
            continue;
        }
        let extracted_answer = extract_answer(
            &task_args.som_ai_client,
            &unique_topics,
            &answer
                .documents
                .iter()
                .map(|doc| somes_common_lib::Document {
                    title: Some(doc.title.clone()),
                    document_url: Some(doc.link.clone()),
                    document_type: Some(doc.ftype.to_string()),
                })
                .collect::<Vec<_>>(),
            &questions,
        )
        .await?;

        if extracted_answer.abort_reason != AbortReason::None
            || extracted_answer.answer_entry.is_none()
        {
            log::error!(
                "(answer) No documents? provided in context for answer {}, documents: {:?}, abort reason: {:?}, skipping",
                answer_id,
                answer.documents,
                extracted_answer.abort_reason
            );
            continue;
        }
        let extracted_answer = extracted_answer.answer_entry.unwrap();
        log::info!("extracted answer: {}", extracted_answer.short_title);
        insert_answer_entry(
            &task_args.pg_pool,
            *answer_id,
            &extracted_answer,
            &task_args.som_ai_client.get_model().await?.result,
            "1",
        )
        .await?;
        extracted_answers.push(extracted_answer);
    }
    Ok(())
}

pub async fn extract_question(
    somai_client: &SomAiClient,
    unique_topics: &[String],
    docs: &[Document],
) -> crate::Result<QuestionEntryOutput> {
    let input = extract_pqa_input(docs).await;

    somai_client
        .make_llm_request_with_schema::<QuestionEntryOutput>(
            input,
            format!(
                r##"Bringe die folgende Anfrage in das angegebene maschinenlesbare JSON-Schema (complete_inquiry_extraction), daher musst du Felder nicht erklären. Extrahiere ALLE Fragen.
                Klassifiziere das Thema anhand dieser Liste (max 10): {unique_topics:?}
                Schreibe eine Zusammenfassung der Fragen in 'question_summary', und eine ausführlichere in 'detailed_question_summary'.
                Die 'info' summaries sollen Zusammenfassungen über den Anlass der Fragestellung beinhalten.
                'short_title' muss einen sprechenden, user-facing Titel entsprechen.
                Die verwendeten Quellen müssen in `info_sources_urls` verwendet werden.
                Die rohe Frage muss in 'raw_question' abgespeichert werden. Entferne dabei allerdings die Numerierung am Anfang. Achte darauf, dass alle Fragen verarbeitet werden.
                Trage bei 'nth_level' immer die konkrete Identifikation der Ebenen-Referenz einer Frage ein. NIEMALS die Frage selbst, sondern nur die Referenz! Das Array darf nicht leer sein! Dabei ist die erste Ebene immer eine numerische Numerierung, weitere Ebenen vom folgenden Beispiel ableiten.
                    z.B. 3. Wird Rust dominieren?
                            a) wenn ja, gehe kurz auf die Vorteile ein
                            b) wenn nein, erwähne kurz die Alternativen
                            i) systems programming languages
                            ii) OOP languages

                Jede einzelne Frage oder Punkt wird anschließend in `questions' eingetragen.
                    z.B. Question {{ raw_question: 'OOP languages', nth_level: [Number(3), String('b'), String('ii')] }} // beachte wie 'nth_level' eingetragen wurde - auschließlich eine konkrete Ebenen-Referenz!
                        Im JSON sieht die Referenz vom oberen Beispiel wie folgt aus: (
                        "nth_level": [
                                {{
                                    "kind": "Number",
                                    "value": 3
                                }},
                                {{
                                    "kind": "String",
                                    "value": "b"
                                }},
                                {{
                                    "kind": "String",
                                    "value": "ii"
                                }}
                                ],
                            )
                        Question {{ raw_question: 'Wird Rust dominieren?', nth_level: [Number(3)] }}
                        Question {{ raw_question: 'wenn nein, erwähne kurz die Alternativen', nth_level: [Number(3), String('b')] }} // Siehst du? In `nth_level` werden AUSCHLIESSLICH die Referenz-Ebenen eingetragen und KEINE Fragen oder Teile davon!
                    und so weiter ...
                Nächstes Beispiel
                z.B. 42. Kann es sein, dass LLMs für diese Aufgabe nicht so toll sind?
                    wenn ja, was sind die Gründe?

                Darauf folgt => (Rust-Darstellung, du musst mit JSON-Format arbeiten)
                    Question {{ raw_question: 'Kann es sein, dass LLMs für diese Aufgabe nicht so toll sind?', nth_level: [Number(42)] }}
                    Question {{ raw_question: 'wenn ja, was sind die Gründe?', nth_level: [Number(42), String('a')] }}

                Falls KEINE relevanten Dokumente, die die Anfrage beschreiben, im Kontext gefunden wurden: Setze `complete_inquiry_extraction` auf null und `abort_reason` auf 'NoDocumentsProvidedInContext'.
                `complete_inquiry_extraction` muss deine Extraktion enthalten!
                Ansonsten, falls `complete_inquiry_extraction` NICHT null ist - das ist der Normalfall - , ist `abort_reason` 'None'.
                "##
            ),
            None,
            false,
            Some(|schema: &mut serde_json::Value| {
                // TODO: activate topic patching when #143 (https://github.com/som-es/somes/issues/143) is done
                crate::ai_tasks::patch_topics("/properties/topics", schema, &unique_topics);
            }),
        )
        .await
}

async fn extract_pqa_input(docs: &[Document]) -> String {
    let docs = docs.iter().filter(|doc| {
        doc.document_type
            .as_deref()
            .map(|doc| doc.contains("PDF"))
            .unwrap_or_default()
    });

    let engine = OCR_ENGINE.clone();
    let input = combine_texts(&extract_text_from_documents_html_first(docs, engine).await);
    input
}

pub async fn extract_answer(
    somai_client: &SomAiClient,
    unique_topics: &[String],
    docs: &[Document],
    questions: &[combx::Question],
) -> crate::Result<AnswerEntryOutput> {
    let input = extract_pqa_input(docs).await;
    somai_client
        .make_llm_request_with_schema::<AnswerEntryOutput>(
            input,
            format!(
                r##"Bringe die folgende Anfragebeantwortung in das angegebene maschinenlesbare JSON-Schema, daher musst du deine Felder nicht erklären.
                Klassifiziere das Thema anhand dieser Liste (max 10): {unique_topics:?}
                Schreibe eine Zusammenfassung der Fragen in 'question_summary', und eine ausführlichere in 'detailed_question_summary'.
                Die 'full_answer' summaries sollen einen Überblick über die Fragebeantwortung geben.
                Speichere immer eine Roh-Version, die ausschließlich der gegebenen Antwort entspricht, in 'raw_answer' ab. Übernehme dabei alles GENAU VOM VORLIEGENDEN DOKUMENT. Kürze nichts! Sei NICHT faul!
                'short_title' muss einen sprechenden, user-facing Titel entsprechen.
                Jede Antwort soll auch entsprechende Zusammenfassungen erhalten (Felder mit 'answer_summary').
                Trage bei den Referenzen zu einer 'affected_question' immer die konkrete Identifikation ein.
                Es können bei einer Antwort mehrere Referenzen betroffen sein 'answering_questions_references'.
                Falls der Fragebeantworter andere parlamentarische Antworten für eine Fragebeantwortung referenziert, dann speichere diese in 'other_written_answer_refs' ab. 'gp' soll dabei nur die römische Zahl beinhalten
                Achte zwingend auf eine saubere und konsequente Trennung der gegebenen Fragen und Antworten und vermische sie nicht .
                Beispiel-Fragen:
                    z.B. 3. Wird Rust dominieren?
                            a) wenn ja, gehe kurz auf die Vorteile ein
                            b) wenn nein, erwähne kurz die Alternativen
                                i) systems programming languages
                                ii) OOP languages
                Antwort:
                    z.B. Ich kann Ihnen nur ausdrücklich sagen, dass Java einfach deutlich besser ist.
                Daraus ergeben sich folgende Referenzen. Gib dabei immer den absoluten Pfad an!
                    z.B.
                        QuestionRef {{ question_assignment_confidence: VeryConfident, affected_question: [Number(3)] }}
                        QuestionRef {{ question_assignment_confidence: VeryConfident, affected_question: [Number(3), String('b')] }}
                        QuestionRef {{ question_assignment_confidence: VeryConfident, affected_question: [Number(3), String('b'), String('ii')] }}
                        Im JSON sieht die Referenz vom oberen Beispiel wie folgt aus: (
                    "affected_question": [
                            {{
                                "kind": "Number",
                                "value": 3
                            }},
                            {{
                                "kind": "String",
                                "value": "b"
                            }},
                            {{
                                "kind": "String",
                                "value": "ii"
                            }}
                            ],
                        )

                Falls KEINE relevanten Dokumente, die die Antwort beschreiben, im Kontext gefunden wurden: Setze `answer_entry` auf null und `abort_reason` auf 'NoDocumentsProvidedInContext'.
                Ansonsten, falls `answer_entry` NICHT null ist - das ist der Normalfall - ist `abort_reason` 'None'.
                `answer_entry` muss zwingend deine Extraktion enthalten!
                Hier sind die tasächlichen Fragen aus der Anfrage, die du den Antworten zuordnen musst:
                {questions:?}
                "##
            ),
            None,
            false,
            Some(|schema: &mut serde_json::Value| {
                // TODO: activate topic patching when #143 (https://github.com/som-es/somes/issues/143) is done
                crate::ai_tasks::patch_topics("/properties/topics", schema, &unique_topics);
                // patch_reference_possibility(schema, &questions);
            }),
        )
        .await
}

#[cfg(test)]
mod tests {
    use combx::{Count, Question};
    use somai_client::SomAiClient;

    use somes_common_lib::Document;

    use crate::parliamentary_qa::{extract_answer, extract_question, patch_reference_possibility};

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
                // "https://www.parlament.gv.at/dokument/XXVII/J/14815/fname_1550801.pdf".into(),
                "https://www.parlament.gv.at/dokument/XXVIII/J/5810/fname_1756270.pdf".into(),
            ),
            document_type: Some("DigitalPDF".into()), // using a DigitalPDF here because HTML list items are not counted accordingly (all html tags get simply removed)
        }];
        let pg_pool = combx::connect_pg().await;
        let unique_topics = sqlx::query_scalar!(
            "select topic_name as topic from unique_eurovoc_topics order by topic"
        )
        .fetch_all(&pg_pool)
        .await
        .unwrap();
        let result = extract_question(&somai_client, &unique_topics, &docs).await;
        dbg!(&result);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_parliamentary_answer_extraction() {
        simple_logger::init_with_level(log::Level::Info).unwrap();
        let somai_client =
            SomAiClient::new("http://10.30.0.164:8000", "API_USER", "supersicher123!")
                .await
                .unwrap();

        let docs = vec![Document {
            title: Some("Anfragebeantwortung".into()),
            document_url: Some(
                "https://www.parlament.gv.at/dokument/XXVII/AB/14326/imfname_1568379.pdf".into(),
            ),
            document_type: Some("DigitalPDF".into()),
        }];
        let pg_pool = combx::connect_pg().await;
        let unique_topics = sqlx::query_scalar!(
            "select topic_name as topic from unique_eurovoc_topics order by topic"
        )
        .fetch_all(&pg_pool)
        .await
        .unwrap();

        #[rustfmt::skip]
        let questions =
            vec![Question { raw_question: "Waren Sie bzw. Ihr Ministerium bezüglich dieses Falles in Kontakt mit der ukrainischen Botschaft bzw. dem ukrainischen Botschafter?".into(), nth_level: vec![Count::Number(1)] }, Question { raw_question: "Wenn ja, wann wer mit wem?".into(), nth_level: vec![Count::Number(1), Count::String("a".into())] }, Question { raw_question: "Wenn ja, Gespräche welchen Inhalts wurden geführt?".into(), nth_level: vec![Count::Number(1), Count::String("b".into())] }, Question { raw_question: "Mit welchem Ergebnis?".into(), nth_level: vec![Count::Number(1), Count::String("b".into()), Count::String("i".into())] }, Question { raw_question: "Wer suchte für welches Gespräch jeweils den Kontakt: Sie bzw. Ihr Ministerium oder das Gegenüber?".into(), nth_level: vec![Count::Number(1), Count::String("c".into())] }, Question { raw_question: "Wenn nein, haben Sie bzw. Ihr Ministerium den Kontakt gesucht?".into(), nth_level: vec![Count::Number(1), Count::String("d".into())] }, Question { raw_question: "Wenn ja, inwiefern wann?".into(), nth_level: vec![Count::Number(1), Count::String("d".into()), Count::String("i".into())] }, Question { raw_question: "Wenn nein, warum nicht?".into(), nth_level: vec![Count::Number(1), Count::String("d".into()), Count::String("ii".into())] }, Question { raw_question: "Waren Sie bzw. Ihr Ministerium bezüglich dieses Falles in Kontakt mit der russischen Botschaft bzw. dem russischen Botschafter?".into(), nth_level: vec![Count::Number(2)] }, Question { raw_question: "Wenn ja, wann wer mit wem?".into(), nth_level: vec![Count::Number(2), Count::String("a".into())] }, Question { raw_question: "Wenn ja, Gespräche welchen Inhalts wurden geführt?".into(), nth_level: vec![Count::Number(2), Count::String("b".into())] }, Question { raw_question: "Mit welchem Ergebnis?".into(), nth_level: vec![Count::Number(2), Count::String("b".into()), Count::String("i".into())] }, Question { raw_question: "Wer suchte für welches Gespräch jeweils den Kontakt: Sie bzw. Ihr Ministerium oder das Gegenüber?".into(), nth_level: vec![Count::Number(2), Count::String("c".into())] }, Question { raw_question: "Wenn nein, haben Sie bzw. Ihr Ministerium den Kontakt gesucht?".into(), nth_level: vec![Count::Number(2), Count::String("d".into())] }, Question { raw_question: "Wenn ja, inwiefern wann?".into(), nth_level: vec![Count::Number(2), Count::String("d".into()), Count::String("i".into())] }, Question { raw_question: "Wenn nein, warum nicht?".into(), nth_level: vec![Count::Number(2), Count::String("d".into()), Count::String("ii".into())] }, Question { raw_question: "Waren Sie bzw. Ihr Ministerium bezüglich dieses Falles in Kontakt mit dem Europäischen Ombudsmann-Institut?".into(), nth_level: vec![Count::Number(3)] }, Question { raw_question: "Wenn ja, wann wer mit wem?".into(), nth_level: vec![Count::Number(3), Count::String("a".into())] }, Question { raw_question: "Wenn ja, Gespräche welchen Inhalts wurden geführt?".into(), nth_level: vec![Count::Number(3), Count::String("b".into())] }, Question { raw_question: "Mit welchem Ergebnis?".into(), nth_level: vec![Count::Number(3), Count::String("b".into()), Count::String("i".into())] }, Question { raw_question: "Wer suchte für welches Gespräch jeweils den Kontakt: Sie bzw. Ihr Ministerium oder das Gegenüber?".into(), nth_level: vec![Count::Number(3), Count::String("c".into())] }, Question { raw_question: "Wenn nein, haben Sie bzw. Ihr Ministerium den Kontakt gesucht?".into(), nth_level: vec![Count::Number(3), Count::String("d".into())] }, Question { raw_question: "Wenn ja, inwiefern wann?".into(), nth_level: vec![Count::Number(3), Count::String("d".into()), Count::String("i".into())] }, Question { raw_question: "Wenn nein, warum nicht?".into(), nth_level: vec![Count::Number(3), Count::String("d".into()), Count::String("ii".into())] }, Question { raw_question: "Waren Sie bzw. Ihr Ministerium bezüglich dieses Falles in Kontakt mit der russischen Ombudsfrau, Tatjana Moskalkowa?".into(), nth_level: vec![Count::Number(4)] }, Question { raw_question: "Wenn ja, wann wer mit wem?".into(), nth_level: vec![Count::Number(4), Count::String("a".into())] }, Question { raw_question: "Wenn ja, Gespräche welchen Inhalts wurden geführt?".into(), nth_level: vec![Count::Number(4), Count::String("b".into())] }, Question { raw_question: "Mit welchem Ergebnis?".into(), nth_level: vec![Count::Number(4), Count::String("b".into()), Count::String("i".into())] }, Question { raw_question: "Wer suchte für welches Gespräch jeweils den Kontakt: Sie bzw. Ihr Ministerium oder das Gegenüber?".into(), nth_level: vec![Count::Number(4), Count::String("c".into())] }, Question { raw_question: "Wenn nein, haben Sie bzw. Ihr Ministerium den Kontakt gesucht?".into(), nth_level: vec![Count::Number(4), Count::String("d".into())] }, Question { raw_question: "Wenn ja, inwiefern wann?".into(), nth_level: vec![Count::Number(4), Count::String("d".into()), Count::String("i".into())] }, Question { raw_question: "Wenn nein, warum nicht?".into(), nth_level: vec![Count::Number(4), Count::String("d".into()), Count::String("ii".into())] }, Question { raw_question: "Waren Sie bzw. Ihr Ministerium bezüglich dieses Falles in Kontakt mit den betroffenen Jugendlichen bzw. einem/einer der Betroffenen?".into(), nth_level: vec![Count::Number(5)] }, Question { raw_question: "Wenn ja, wann wer mit wem?".into(), nth_level: vec![Count::Number(5), Count::String("a".into())] }, Question { raw_question: "Wenn ja, Gespräche welchen Inhalts wurden geführt?".into(), nth_level: vec![Count::Number(5), Count::String("b".into())] }, Question { raw_question: "Mit welchem Ergebnis?".into(), nth_level: vec![Count::Number(5), Count::String("b".into()), Count::String("i".into())] }, Question { raw_question: "Wer suchte für welches Gespräch jeweils den Kontakt: Sie bzw. Ihr Ministerium oder das Gegenüber?".into(), nth_level: vec![Count::Number(5), Count::String("c".into())] }, Question { raw_question: "Wenn nein, haben Sie bzw. Ihr Ministerium den Kontakt gesucht?".into(), nth_level: vec![Count::Number(5), Count::String("d".into())] }, Question { raw_question: "Wenn ja, inwiefern wann?".into(), nth_level: vec![Count::Number(5), Count::String("d".into()), Count::String("i".into())] }, Question { raw_question: "Wenn nein, warum nicht?".into(), nth_level: vec![Count::Number(5), Count::String("d".into()), Count::String("ii".into())] }, Question { raw_question: "Waren Sie bzw. Ihr Ministerium bezüglich dieses Falles in Kontakt mit den betroffenen Müttern bzw. einer der betroffenen Mütter?".into(), nth_level: vec![Count::Number(6)] }, Question { raw_question: "Wenn ja, wann wer mit wem?".into(), nth_level: vec![Count::Number(6), Count::String("a".into())] }, Question { raw_question: "Wenn ja, Gespräche welchen Inhalts wurden geführt?".into(), nth_level: vec![Count::Number(6), Count::String("b".into())] }, Question { raw_question: "Mit welchem Ergebnis?".into(), nth_level: vec![Count::Number(6), Count::String("b".into()), Count::String("i".into())] }, Question { raw_question: "Wer suchte für welches Gespräch jeweils den Kontakt: Sie bzw. Ihr Ministerium oder das Gegenüber?".into(), nth_level: vec![Count::Number(6), Count::String("c".into())] }, Question { raw_question: "Wenn nein, haben Sie bzw. Ihr Ministerium den Kontakt gesucht?".into(), nth_level: vec![Count::Number(6), Count::String("d".into())] }, Question { raw_question: "Wenn ja, inwiefern wann?".into(), nth_level: vec![Count::Number(6), Count::String("d".into()), Count::String("i".into())] }, Question { raw_question: "Wenn nein, warum nicht?".into(), nth_level: vec![Count::Number(6), Count::String("d".into()), Count::String("ii".into())] }, Question { raw_question: "War bisher in der Praxis eine schriftliche Zustimmung der/s Obsorgeberechtigten bzw. gesetzlichen Vertreter:in in Österreich ausreichend, um festzustellen, dass die Rückkehr dem Kindeswohl entspricht?".into(), nth_level: vec![Count::Number(7)] }, Question { raw_question: "Gibt es - in jenen Fällen, in denen IOM kein Family Assessment durchführt - keine weitere Prüfung zur Sicherstellung des Kindeswohls?".into(), nth_level: vec![Count::Number(7), Count::String("a".into())] }, Question { raw_question: "Anhand welcher Kriterien wird beurteilt, ob „Bedarf“ für ein Family Assessment durch das IOM besteht oder nicht?".into(), nth_level: vec![Count::Number(8)] }, Question { raw_question: "Durch wen bzw. welche Behörde?".into(), nth_level: vec![Count::Number(8), Count::String("a".into())] }, Question { raw_question: "Wie verläuft das Family Assessment und was wird konkret geprüft?".into(), nth_level: vec![Count::Number(8), Count::String("b".into())] }, Question { raw_question: "Inwiefern wurde in diesem konkreten Fall geprüft, ob die Notwendigkeit eines Family Assessment durch das IOM besteht?".into(), nth_level: vec![Count::Number(9)] }, Question { raw_question: "Wann und durch wen bzw. welche Behörde?".into(), nth_level: vec![Count::Number(9), Count::String("a".into())] }, Question { raw_question: "Wie kam wer zu dem Schluss, dass diese Notwendigkeit nicht besteht?".into(), nth_level: vec![Count::Number(9), Count::String("b".into())] }, Question { raw_question: "Wer war in diese Entscheidung eingebunden?".into(), nth_level: vec![Count::Number(9), Count::String("c".into())] }, Question { raw_question: "Wie viele Fälle der freiwilligen Rückkehr von unbegleiteten Minderjährigen gab es im Jahr 2022? Bitte um Aufschlüsselung nach Staatsangehörigkeit, Destination und mündig/unmündig.".into(), nth_level: vec![Count::Number(10)] }, Question { raw_question: "In wie vielen Fällen reichte hierfür lediglich die schriftliche Zustimmung des Obsorgeberechtigten bzw. gesetzlichen Vertreters in Österreich?".into(), nth_level: vec![Count::Number(10), Count::String("a".into())] }, Question { raw_question: "In wie vielen Fällen wurde seitens IOM ein Family Assessment durchgeführt?".into(), nth_level: vec![Count::Number(10), Count::String("b".into())] }, Question { raw_question: "In wie vielen Fällen wurden die betroffenen Minderjährigen bei der Rückkehr begleitet?".into(), nth_level: vec![Count::Number(10), Count::String("c".into())] }, Question { raw_question: "Wurde aufgrund dieses Falles in Ihrem Ressort Zeit aufgewendet, mögliche Defizite vonseiten des Ressorts".into(), nth_level: vec![Count::Number(11)] }, Question { raw_question: "im konkreten Fall".into(), nth_level: vec![Count::Number(11), Count::String("a".into())] }, Question { raw_question: "im grundsätzlichen Ablauf des bestehenden Verfahrens der Familienzusammenführung zu identifizieren?".into(), nth_level: vec![Count::Number(11), Count::String("b".into())] }, Question { raw_question: "Wenn ja, wann durch Gespräche welcher Verantwortlichen zu welchem Inhalt?".into(), nth_level: vec![Count::Number(11), Count::String("c".into())] }, Question { raw_question: "Wenn ja, mit welchem wann vorliegenden Ergebnis?".into(), nth_level: vec![Count::Number(11), Count::String("d".into())] }, Question { raw_question: "Wenn nein, warum nicht?".into(), nth_level: vec![Count::Number(11), Count::String("e".into())] }, Question { raw_question: "Welche konkreten Maßnahmen haben Sie bzw. Ihr Ressort gesetzt, damit künftig - mit Blick auf diesen Fall - welche Vorfälle verhindert werden können?".into(), nth_level: vec![Count::Number(12)] }, Question { raw_question: "Welche konkreten Maßnahmen planen Sie bzw. Ihr Ressort noch wann durch wen setzen?".into(), nth_level: vec![Count::Number(12), Count::String("a".into())] }, Question { raw_question: "Sollten keine Maßnahmen gesetzt worden sein bzw. geplant sein: warum nicht?".into(), nth_level: vec![Count::Number(12), Count::String("b".into())] }, Question { raw_question: "Haben Sie bzw. Ihr Ministerium vor, Maßnahmen zu setzen, damit in Zukunft das Kindeswohl systematisch bei jeder Verbringung ins Ausland von unbegleiteten Minderjährigen - über die schriftliche Zustimmung des Obsorgeberechtigten bzw. gesetzlichen Vertreters in Österreich hinaus - geprüft wird?".into(), nth_level: vec![Count::Number(13)] }, Question { raw_question: "Sollen künftig für die freiwillige Rückkehr von unbegleiteten Minderjährigen besondere Bestimmungen gelten?".into(), nth_level: vec![Count::Number(13), Count::String("a".into())] }, Question { raw_question: "Wenn ja, welche?".into(), nth_level: vec![Count::Number(13), Count::String("a".into()), Count::String("i".into())] }, Question { raw_question: "Wenn nein, warum nicht?".into(), nth_level: vec![Count::Number(13), Count::String("a".into()), Count::String("ii".into())] }, Question { raw_question: "Haben Sie bzw. Ihr Ministerium inzwischen Informationen über den Verbleib der betroffenen Jugendlichen?".into(), nth_level: vec![Count::Number(14)] }, Question { raw_question: "Wenn ja, welche seit wann?".into(), nth_level: vec![Count::Number(14), Count::String("a".into())] }, Question { raw_question: "Welche Schritte haben Sie bzw. Ihr Ressort aktiv gesetzt, um zur Aufklärung dieses Sachverhalts beizutragen?".into(), nth_level: vec![Count::Number(15)] }, Question { raw_question: "Wann jeweils?".into(), nth_level: vec![Count::Number(15), Count::String("a".into())] }, Question { raw_question: "Die russische Ombudsfrau richtete in dem oben genannten Video Dank an die russische Botschaft und an die russischen Nachrichtendienste aus. Wie kann es sein, dass ukrainische Kinder in Österreich anscheinend von russischen Nachrichtendiensten bewacht bzw. lokalisiert werden?".into(), nth_level: vec![Count::Number(16)] }, Question { raw_question: "Nahm sich die DSN dieses Falles an?".into(), nth_level: vec![Count::Number(17)] }, Question { raw_question: "Wenn ja, wann inwiefern durch welchen durch wen gesetzte Maßnahme?".into(), nth_level: vec![Count::Number(17), Count::String("a".into())] }, Question { raw_question: "Nahm sich die DSN generell des Schutzes ukrainischer Kinder und Jugendliche in Österreich gegen Bedienstete Russlands und russische Nachrichtendienste an?".into(), nth_level: vec![Count::Number(18)] }, Question { raw_question: "Wenn ja, wann inwiefern durch welchen durch wen gesetzte Maßnahme?".into(), nth_level: vec![Count::Number(18), Count::String("a".into())] }, Question { raw_question: "Wurden Vorkehrungen seitens Ihres Ministeriums getroffen, um ukrainische Kinder in Österreich gegen Bedienstete Russlands und russische Nachrichtendienste zu schützen?".into(), nth_level: vec![Count::Number(19)] }, Question { raw_question: "Wenn ja, welche und aufgrund welcher wann durch wen gesetzte Maßnahme(n)?".into(), nth_level: vec![Count::Number(19), Count::String("a".into())] }, Question { raw_question: "Wenn nein, warum nicht?".into(), nth_level: vec![Count::Number(19), Count::String("b".into())] }, Question { raw_question: "Wenn nein, welche Maßnahme(n) planen Sie bzw. Ihr Ressort wann durch wen noch zu setzen?".into(), nth_level: vec![Count::Number(19), Count::String("c".into())] }, Question { raw_question: "Haben Sie bzw. Ihr Ministerium um das Einverständnis zur Übermittlung per Anfragebeantwortung der von uns erfragten personenbezogenen Daten bei den Betroffenen angefragt?".into(), nth_level: vec![Count::Number(20)] }, Question { raw_question: "Wenn ja, welche Antworten erhielten Sie?".into(), nth_level: vec![Count::Number(20), Count::String("a".into())] }, Question { raw_question: "Wenn nein, warum haben Sie dies nicht getan?".into(), nth_level: vec![Count::Number(20), Count::String("b".into())] }, Question { raw_question: "Welche gesetzlichen Regelungen würden durch eine Beantwortung Ihrerseits mit \"ja\" oder \"nein\" verletzt werden (bitte um genaue Anführung des Paragrafen und der entsprechenden nachvollziehbaren Subsumtion)?".into(), nth_level: vec![Count::Number(21)] }, Question { raw_question: "In der folgenden Frage wollten wir von Ihnen wissen, ob sich durch kritische Äußerungen von Teilnehmer:innen dadurch deren Arbeitssituation verschlechtert haben. Auch diese Fragen konnten Sie aufgrund von Datenschutz und Amtsverschwiegenheit nicht beantworten. Aufgrund der Tatsache, dass wir in unserer Frage keine personenbezogenen Daten erfragten, die in irgendeiner Weise auf eine Person rückführbar gewesen wären: Welche gesetzlichen Regelungen würden durch eine Beantwortung Ihrerseits mit \"ja\" oder \"nein\" verletzt werden (bitte um genaue Anführung des Paragrafen und der entsprechenden nachvollziehbaren Subsumtion)?".into(), nth_level: vec![Count::Number(21), Count::String("a".into())] }, Question { raw_question: "In der darauffolgenden Frage wollten wir von Ihnen wissen - falls es zu einer Verschlechterung der Arbeitssituation gekommen ist - was sich wodurch und inwiefern an der Arbeitssituation verschlechtert hat. Auch hier wurden keine personenbezogenen Daten erfragt, sondern lediglich anonym erfragt, wie sich die jeweilige Arbeitssituation (z.B. Versetzung, Überstunden, Mobbing am Arbeitsplatz, schlechtere Aussichten auf Beförderung etc.) verschlechtert hat. Welche gesetzlichen Regelungen würden durch eine Beantwortung Ihrerseits verletzt werden (bitte um genaue Anführung des Paragrafen und der entsprechenden nachvollziehbaren Subsumtion)?".into(), nth_level: vec![Count::Number(21), Count::String("b".into())] }, Question { raw_question: "Möchten Sie sich für eine anonymisierte Beantwortung meiner Fragen 2, 2a und 2b entscheiden?".into(), nth_level: vec![Count::Number(21), Count::String("c".into())] }];

        let result = extract_answer(&somai_client, &unique_topics, &docs, &questions).await;
        dbg!(&result);
        assert!(result.is_ok());
    }
}
