use chrono::Local;
use combx::{
    AbortReason, AiSummary,
    Error::Custom,
    RetryState, SummarizeOutput,
    document_text::{combine_texts, extract_text_from_documents_html_first_stripped},
    with_data::unique_topics::unique_topics_for_lang,
};
use serde_json::json;
use somai_client::SomAiClient;
use somes_common_lib::Document;
use somes_scraper::language::Language;

use crate::{OCR_ENGINE, ai_task::TaskArgs};

pub async fn insert_ai_summary_sqlx(
    pool: &sqlx::PgPool,
    summary: &AiSummary,
    model_used: &str,
) -> combx::Result<i32> {
    let id = sqlx::query!(
        "INSERT INTO summaries (
            full_summary,
            short_title,
            short_summary,
            detailed_summary,
            complexity_scope_of_proposal,
            model_used,
            version,
            generated_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING id;
         ",
        serde_json::to_value(summary)?,
        summary.short_title,
        summary.short_summary,
        summary.detailed_summary,
        format!("{:?}", summary.complexity_scope_of_proposal),
        model_used,
        "1",
        Local::now()
    )
    .fetch_one(pool)
    .await?;
    Ok(id.id)
}

pub trait RisExtension {
    fn extend_docs(
        _pool: &sqlx::PgPool,
        _entry_id: i32,
    ) -> impl Future<Output = sqlx::Result<Vec<Document>>> {
        async { Ok(vec![]) }
    }
}

pub async fn summarize_documents<T: RisExtension>(
    task_args: &TaskArgs,
    docs: &[Document],
    summarization_of: &str,
    language: Language,
) -> combx::Result<SummarizeOutput> {
    let summary = summarize_documents_inner(task_args, docs, summarization_of, language).await?;

    if summary.retry_state == RetryState::ExtendDocsViaRis {
        let mut extend_ris_docs = T::extend_docs(&task_args.pg_pool, task_args.db_entry_id).await?;
        if extend_ris_docs.is_empty() {
            return Ok(summary);
        }
        extend_ris_docs.extend_from_slice(docs);
        return summarize_documents_inner(task_args, &extend_ris_docs, summarization_of, language)
            .await;
    }
    Ok(summary)
}

async fn summarize_documents_inner(
    task_args: &TaskArgs,
    docs: &[Document],
    summarization_of: &str,
    language: Language,
) -> combx::Result<SummarizeOutput> {
    let inner_docs = docs.to_vec();
    match tokio::spawn(async move {
        log::info!("starting doc text collection");
        let engine = OCR_ENGINE.clone();
        combine_texts(&extract_text_from_documents_html_first_stripped(&inner_docs, engine).await)
    })
    .await
    {
        Ok(input) => {
            log::info!("doc collection successful");
            if input.len() < 50 {
                log::error!(
                    "Cannot summarize legis init {}, input is too short / empty for documents: {:?}",
                    task_args.db_entry_id,
                    docs
                );
                return Err(Box::new(combx::Error::Custom(
                    "Input too short for summarization".into(),
                )));
            }
            let unique_topics = unique_topics_for_lang(&task_args.pg_pool, language).await?;

            let summary = summary_prompt(
                &task_args.som_ai_client,
                input,
                &unique_topics,
                summarization_of,
                language,
            )
            .await?;

            return Ok(summary);
        }
        Err(e) => log::error!("cannot extract text input from {:?} because of {e:?}", docs),
    }

    Err(Box::new(combx::Error::Custom(
        "Cannot extract text input for summarization".into(),
    )))
}

pub fn patch_topics(
    attribute_pointer: &str,
    schema: &mut serde_json::Value,
    allowed_topics: &[String],
) {
    if let Some(topics_schema) = schema.pointer_mut(attribute_pointer) {
        *topics_schema = json!({
            "type": "array",
            "items": {
                "type": "string",
                "enum": allowed_topics
            },
            "uniqueItems": true,
            "minItems": 1,
            "maxItems": 10
        });
    }
}

pub(crate) async fn summary_prompt(
    somai_client: &SomAiClient,
    input: String,
    topics: &[String],
    summarization_of: &str,
    language: Language,
) -> combx::Result<SummarizeOutput> {
    let prompt = match language {
        Language::En => {
            format!(
                r#"# Role

                You are an expert in analyzing parliamentary data (motions/applications) and RIS entries (e.g., regulations). You structure complex political documents into a precise JSON format that is understandable for laypeople.

                You analyze documents of the type: {summarization_of}.

                ---

                ## STEP 0 — MANDATORY CHECK BEFORE FILLING IN ANY OTHER FIELD

                Answer these two questions for yourself in this order **before** you begin Task 1:

                **Question A — Are there any relevant documents present at all?** No → `summary` = `null`, `abort_reason` = `NoDocumentsProvidedInContext`. All other fields = `null`/empty. STOP, no further processing.

                **Question B — Is the text provided sufficient to explain WHAT is actually changing — not just THAT something is changing?**

                Warning signs where the answer is NO:

                - The text only says "§ X is being amended/repealed" or "Annex/Attachment Y is omitted/replaced," **without naming** the content of § X or Attachment Y, or making it inferable.
                - The text consists predominantly of references to other places whose content is not provided.
                - A correct summary would only be possible if you had to **guess or invent** the referenced content.

                If a warning sign applies → generate the `summary` anyway, but `retry_state` = `ExtendDocsViaRis`.

                **Rule of thumb: When in doubt, generate anyway and guess.** A summary like "Attachment 5 was deleted" is not entirely worthless for laypeople. Nevertheless, prefer to choose `ExtendDocsViaRis` **and** guess. In the next attempt we will try to refine the summary.

                Only if A is answered with "yes, sufficient" do you proceed with Tasks 1–7.

                ### Example — follow this pattern exactly

                **Input (complete document):**

                > "1. After § 12 para. 10, the following para. 11 is added: '(11) Attachment 5 expires at the end of the day of promulgation.' 2. Attachment 5 is omitted."

                **Expected output:**

                \```json
                  "summary": generate summary anyway...!!,
                  "abort_reason": "ExtendDocsViaRis"
                \```

                *(all remaining fields likewise null/empty — do not fantasize about what might have been in Attachment 5)*

                **Justification (not part of the output):** The content of "Attachment 5" is not stated anywhere. A summary could only say "an attachment is being deleted," without it being recognizable to citizens what that means (e.g., which ticket, which prices, which regions are affected).

                ---

                ## TASKS 1–7 (only if Step 0 was passed)

                1. **Summary**
                   - `short_title`: short, descriptive title.
                   - `short_summary`: max. 2–3 sentences.
                   - `detailed_summary`: 2–3× as long as `short_summary`.
                   - `very_detailed_summary`: 3–4× as long as `short_summary`.
                   - No paragraph references in `short_summary`/`detailed_summary` — they confuse laypeople. Instead of "The motion adds § 21 para. 23" → "The motion adds a new provision that …".
                2. **Key Points** (`key_points`, 3–8 bullet points, starting superficially, gradually more detailed)
                   - Check each point for `enforcement_start_date`/`enforcement_end_date` in the text.
                   - No paragraph references in `point` — only in `paragraph_references`.
                   - State the raw reference (e.g., "§ 5 para. 2 no. 3") and break it down in `parsed_reference_parts`: Article, Paragraph, Subsection, Point. For ranges/lists, list all parts individually (para. 1–3 → Subsection(1), Subsection(2), Subsection(3); para. 1,4 → Subsection(1), Subsection(4)).
                   - Only fill `paragraph_references` if a legal text is present.
                3. **Metadata & Classification**
                   - Classify topic (max. 10) based on: {topics:?}
                   - `complexity_scope_of_proposal`: Highest, High, Medium, Low, Lowest.
                   - `general_enforcement_start_end`, if applicable to the entire motion.
                4. **Critical Analysis**
                   - `arguments_for`, `arguments_against`.
                   - `tone`: Neutral, Urgent, Aggressive, Optimistic, Bureaucratic.
                5. **Financial Analysis** (`fiscal_analysis`)
                   - Costs in million € (year/month), if data available.
                   - `economic_burden`: exactly one of HighCost, ModerateCost, LowCost, Neutral, RevenueGenerating, Unclear.
                6. **Societal Relevance**
                   - `general_political_questions`: critical political questions.
                   - `political_compass_questions`: yes/no questions for a political compass test.
                7. **Glossary** (`difficult_terms`)
                   - Better to explain more terms than too few. `term` + `simple_definition`.

                ---

                ## FORMAL RULES

                - Language of the documents = response language (German stays German, English stays English).
                - Target audience: average people in Austria/EU — accessible, no paragraph/subsection references in body text.
                - Date format: YYYY-MM-DD, machine-readable. Additional info in `start_notes`/`end_notes`.
                - Field names and enum values exactly as in the JSON schema.

                ## SELF-CHECK BEFORE OUTPUT

                Before answering, check:

                1. Did I really perform Step 0 first — not skip it?
                2. If `summary` = `null`: Are really **all** other fields also `null`/empty, or did I accidentally generate content anyway?
                3. If `summary` ≠ `null`: Is `abort_reason` = `"None"`?"#
            )
        }
        Language::De => format!(
            r#"
            # Rolle

            Du bist ein Experte für die Analyse von Parlamentsdaten (Anträge) und RIS-Einträgen (z. B. Verordnungen). Du strukturierst komplexe politische Dokumente in ein präzises JSON-Format, das für Laien verständlich ist.

            Du analysierst Dokumente vom Typ: {summarization_of}.

            ---

            ## SCHRITT 0 — PFLICHT-PRÜFUNG, BEVOR DU IRGENDEIN ANDERES FELD BEFÜLLST

            Beantworte für dich diese zwei Fragen in dieser Reihenfolge, **bevor** du mit Aufgabe 1 beginnst:

            **Frage A — Liegen überhaupt relevante Dokumente vor?** Nein → `summary` = `null`, `abort_reason` = `NoDocumentsProvidedInContext`. Alle anderen Felder = `null`/leer. STOPP, keine weitere Verarbeitung.

            **Frage B — Reicht der vorliegende Text aus, um zu erklären, WAS sich WIRKLICH ändert — nicht nur DASS sich etwas ändert?**

            Warnsignale, bei denen die Antwort NEIN ist:

            - Der Text sagt nur "§ X wird geändert/aufgehoben" oder "Beilage/Anhang Y entfällt/wird ersetzt", **ohne den Inhalt** von § X bzw. Beilage Y zu nennen oder erschließbar zu machen.
            - Der Text besteht überwiegend aus Verweisen auf andere Stellen, deren Inhalt nicht mitgeliefert wird.
            - Eine korrekte Zusammenfassung wäre nur möglich, wenn du den referenzierten Inhalt **raten oder erfinden** müsstest.

            Trifft ein Warnsignal zu → `summary` trotzdem generieren aber `retry_state` = `ExtendDocsViaRis`.

            **Faustregel: Im Zweifel trotzdem generieren und raten.** Eine Zusammenfassung wie "Beilage 5 wurde gestrichen" ist für Laien nicht ganz wertlos. Trotzdem lieber `ExtendDocsViaRis` wählen **und** raten. Im nächsten Versuch versuchen wir die Summary zu präzisieren.

            Nur wenn A mit "ja, ausreichend" beantwortet sind, fährst du mit Aufgabe 1–7 fort.

            ### Beispiel — folge exakt diesem Muster

            **Eingabe (vollständiges Dokument):**

            > "1. Nach § 12 Abs. 10 wird folgender Abs. 11 angefügt: „(11) Die Beilage 5 tritt mit Ablauf des Tages der Kundmachung außer Kraft.“ 2. Die Beilage 5 entfällt."

            **Erwartete Ausgabe:**

            ```json
              "summary": trotzdem Summary generieren...!!,
              "abort_reason": "ExtendDocsViaRis"
            ```

            *(alle übrigen Felder ebenfalls null/leer — nicht fantasieren, was in Beilage 5 gestanden haben könnte)*

            **Begründung (nicht Teil der Ausgabe):** Der Inhalt von "Beilage 5" wird nirgends genannt. Eine Zusammenfassung könnte nur sagen "eine Beilage wird gestrichen", ohne dass für Bürger:innen erkennbar ist, was das bedeutet (z. B. welches Ticket, welche Preise, welche Regionen betroffen sind).

            ---

            ## AUFGABEN 1–7 (nur wenn Schritt 0 bestanden wurde)

            1. **Zusammenfassung**
               - `short_title`: kurzer, sprechender Titel.
               - `short_summary`: max. 2–3 Sätze.
               - `detailed_summary`: 2–3× so lang wie `short_summary`.
               - `very_detailed_summary`: 3–4× so lang wie `short_summary`.
               - Keine Paragraphenangaben in `short_summary`/`detailed_summary` — Laien verwirren sie. Statt „Der Antrag fügt § 21 Abs. 23 hinzu“ → „Der Antrag fügt eine neue Regelung hinzu, die …“.
            2. **Key Points** (`key_points`, 3–8 Stichpunkte, oberflächlich beginnend, graduell detaillierter)
               - Prüfe je Punkt auf `enforcement_start_date`/`enforcement_end_date` im Text.
               - Keine Paragraphenangaben in `point` — nur in `paragraph_references`.
               - Rohverweis angeben (z. B. „§ 5 Abs. 2 Z 3“) und in `parsed_reference_parts` zerlegen: Article, Paragraph, Subsection, Point. Bei Bereichen/Listen alle Teile einzeln angeben (Abs. 1–3 → Subsection(1), Subsection(2), Subsection(3); Abs. 1,4 → Subsection(1), Subsection(4)).
               - `paragraph_references` nur befüllen, wenn ein Gesetzestext vorliegt.
            3. **Metadaten & Einordnung**
               - Thema klassifizieren (max. 10) anhand: {topics:?}
               - `complexity_scope_of_proposal`: Highest, High, Medium, Low, Lowest.
               - `general_enforcement_start_end`, sofern für den gesamten Antrag gültig.
            4. **Kritische Analyse**
               - `arguments_for`, `arguments_against`.
               - `tone`: Neutral, Urgent, Aggressive, Optimistic, Bureaucratic.
            5. **Finanzielle Analyse** (`fiscal_analysis`)
               - Kosten in Mio. € (Jahr/Monat), falls Daten vorhanden.
               - `economic_burden`: exakt einer von HighCost, ModerateCost, LowCost, Neutral, RevenueGenerating, Unclear.
            6. **Gesellschaftliche Relevanz**
               - `general_political_questions`: kritische politische Fragen.
               - `political_compass_questions`: Ja/Nein-Fragen für Polit-Kompass-Test.
            7. **Glossar** (`difficult_terms`)
               - Lieber mehr Begriffe erklären als zu wenige. `term` + `simple_definition`.

            ---

            ## FORMALE REGELN

            - Sprache der Dokumente = Antwortsprache (Deutsch bleibt Deutsch, Englisch bleibt Englisch).
            - Zielgruppe: Durchschnittspersonen in Österreich/EU — barrierefrei, keine Paragraphen-/Absatzverweise in Fließtexten.
            - Datumsformat: YYYY-MM-DD, maschinenlesbar. Zusatzinfos in `start_notes`/`end_notes`.
            - Feldnamen und Enum-Werte exakt wie im JSON-Schema.

            ## SELBSTCHECK VOR DER AUSGABE

            Bevor du antwortest, prüfe:

            1. Habe ich Schritt 0 wirklich zuerst durchgeführt — nicht übersprungen?
            2. Falls `summary` = `null`: Sind wirklich **alle** anderen Felder ebenfalls `null`/leer, oder habe ich versehentlich doch Inhalte generiert?
            3. Falls `summary` ≠ `null`: Ist `abort_reason` = `"None"`?                        "#
        ),
        _ => return Err(Custom("language not supported".into()).into()),
    };
    somai_client
        .make_llm_request_with_schema::<SummarizeOutput>(
            input,
            prompt,
            None,
            false,
            Some(|schema: &mut serde_json::Value| {
                // TODO: activate topic patching when #143 (https://github.com/som-es/somes/issues/143) is done
                patch_topics("/properties/summary/properties/topics", schema, topics);
            }),
        )
        .await
}
