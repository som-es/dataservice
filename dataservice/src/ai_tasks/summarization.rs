use chrono::Local;
use combx::{
    document_text::{combine_texts, extract_text_from_documents_html_first},
    AiSummary, SummarizeOutput,
};
use serde_json::json;
use somai_client::SomAiClient;
use somes_common_lib::Document;

use crate::{ai_task::TaskArgs, OCR_ENGINE};

pub async fn insert_ai_summary_sqlx(
    pool: &sqlx::PgPool,
    summary: &AiSummary,
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
        "GPT-OSS-120B",
        "1",
        Local::now()
    )
    .fetch_one(pool)
    .await?;
    Ok(id.id)
}

pub async fn summarize_documents(
    task_args: &TaskArgs,
    docs: &[Document],
    summarization_of: &str,
) -> combx::Result<SummarizeOutput> {
    let inner_docs = docs.to_vec();
    match tokio::spawn(async move {
        log::info!("starting doc text collection");
        let engine = OCR_ENGINE.clone();
        combine_texts(&extract_text_from_documents_html_first(&inner_docs, engine).await)
    })
    .await
    {
        Ok(input) => {
            log::info!("doc collection successful");
            if input.len() < 50 {
                log::error!(
            "Cannot summarize legis init {}, input is too short / empty for documents: {:?}",
            task_args.db_entry_id, docs
        );
                return Err(Box::new(combx::Error::Custom(
                    "Input too short for summarization".into(),
                )));
            }
            let unique_topics = sqlx::query_scalar!(
                "select topic_name as topic from unique_eurovoc_topics order by topic"
            )
            .fetch_all(&task_args.pg_pool)
            .await?;

            let summary = summary_prompt2(
                &task_args.som_ai_client,
                input,
                &unique_topics,
                summarization_of,
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

async fn summary_prompt(
    somai_client: &SomAiClient,
    input: String,
    topics: &[String],
) -> combx::Result<SummarizeOutput> {
    Ok(somai_client
        .make_llm_request_with_schema::<SummarizeOutput>(input, format!("
                    Du bist ein Experte für politische Zusammenfassungen und Analyse in Österreich für Laien. 
                    Bitte fasse die beigefügten Dokumente, die zu einem Antrag, der im österreichischen Nationalrat behandelt wird, gehören, zusammen. 
                    Diese Dokumente sollen dir helfen, den Antrag besser zu verstehen. 
                    Generiere einen kurzen und bündigen Titel für den Antrag. => short_title
                    Fasse einerseits den Antrag in 2, bei komplexeren Anträgen in maximal 3, Sätzen zusammen. => short_summary
                    Fasse andererseits den Antrag in 3 bis maximal 8 Stichpunkten zusammen. => key_points
                    Bei einfacheren Anträgen können es auch weniger Stichpunkte sein (auch weniger als 3).
                    Dabei sollen die ersten Stichpunkte oberflächlicher sein. Weitere Stichpunkte sollen graduell Details ergänzen. 
                    
                    Falls unterschiedliche Bestimmungen zu unterschiedlichen Zeitpunkten in Kraft treten (start_date) oder anschließend außer Kraft treten bzw. auslaufen (end_date), gib dies bei den jeweiligen Stichpunkten an. => key_points.enforcement_start_end.start_date, key_points.enforcement_start_end.end_date
                    Extrahiere das Datum des Inkrafttretens und des Auslaufens des Gesetzes, falls eindeutige, allgemein gültige Zeitpunkte vorhanden sind (Typischerweise nicht befüllen, sofern eine Spezifizierung nach Stichpunkten vorliegt). => general_enforcement_start_end
                    Zusätzliche Hinweise zu den Daten können in start_notes und end_notes angegeben werden. => general_enforcement_start_end.start_notes, general_enforcement_start_end.end_notes

                    Überlege dir, falls passend, kritische und politische Fragen zu dem Antrag. => general_political_questions

                    Besonders brisante und gesellschaftlich spannende Anträge sollen für einen automatisierten politischen Kompass Test verwendet werden. Schreibe dazu Fragen auf, die durchschnittliche Bürger mit Ja/Nein beantworten können. => political_compass_questions

                    Klassifiziere die behandelnden Themen des Antrags anhand der folgenden Liste, wähle bis zu 10 Themen: {topics:?} => topics

                    Falls ein Glossar für schwierige Begriffe notwendig ist, achte darauf, dass die Definition direkt beginnt, ohne den eigentlichen zu definierenden Term anfangs zu wiederholen => glossary.difficult_terms.simple_definition 

                    Wähle den finanziellen Umfang des Antrags anhand der folgenden Kategorien, setze dabei nicht zu hoch an!: HighCost, ModerateCost, LowCost, Neutral, RevenueGenerating, Unclear => fiscal_analysis.economic_burden
                    Schätze die Kosten des Antrags in Millionen Euro pro Jahr oder pro Monat, falls möglich => fiscal_analysis.estimated_cost_per_year_in_million, fiscal_analysis.estimated_cost_per_month_in_million

                    Falls z.B. keine Dokumente bereitgestellt wurden, die den Antrag beschreiben, brich den Vorgang ab und gib als Grund 'NoDocumentsProvidedInContext' an. => abort_reason

                    Achte bei den produzierten Informationen auf Maschinenlesbarkeit.
                    Halte dich allgemein an das folgende Datumsformat: YYYY-MM-DD -> Maschinenlesbar!
                    Die Informationen müssen von einer durchschnittlichen Person (aus Österreich) verstanden werden können und barrierfrei sein. Antworte ausschließlich in deutscher Sprache.
                    "), None, false, Some(|_: &mut serde_json::Value| {}))
        .await?)
}

pub fn patch_topics(schema: &mut serde_json::Value, allowed_topics: &[String]) {
    if let Some(topics_schema) = schema.pointer_mut("/properties/summary/properties/topics") {
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

pub(crate) async fn summary_prompt2(
    somai_client: &SomAiClient,
    input: String,
    topics: &[String],
    summarization_of: &str,
) -> combx::Result<SummarizeOutput> {
    Ok(somai_client
        .make_llm_request_with_schema::<SummarizeOutput>(
            input,
            format!("
            Du bist ein Experte für die Analyse österreichischer Parlamentsdaten (Anträge) und RIS-Einträge (z.B. Verordnungen). Deine Aufgabe ist es, komplexe politische Dokumente für Laien verständlich aufzubereiten und in ein präzises JSON-Format zu strukturieren.
            In diesem Fall analysierst du Dokumente, die zu folgendem Typ gehören: {summarization_of}.

### DEINE AUFGABEN:
1.  **Zusammenfassung:** 
    - Erstelle einen kurzen, sprechenden Titel, der das Thema des Antrags prägnant zusammenfasst (`short_title`).
    - Erstelle eine ultrakurze Zusammenfassung (2-3 Sätze) für `short_summary`.
    - Erstelle eine detailliertere Zusammenfassung für `detailed_summary`. (2x bis 3x so lang wie `short_summary`)
    - Erstelle eine noch detailliertere Zusammenfassung für `very_detailed_summary`. (3x bis 4x so lang wie `short_summary`)
    - Wenn Paragraphenangaben vorhanden wären, lass sie weg. Keine Paragraphenangaben in `short_summary` oder `detailed_summary`, da sie Laien verwirren würden.
    - Statt „Der Antrag fügte § 21 Abs. 23 hinzu ...“ formuliere lieber z.B. „Der Antrag fügt eine neue Regelung hinzu, die ...“.
2.  **Key Points:** Erstelle 3-8 Stichpunkte (`key_points`). Beginne oberflächlich und werde graduell detaillierter. 
    - Prüfe für jeden Punkt, ob spezifische Inkrafttretensdaten (`enforcement_start_date`) oder Ablaufdaten (`enforcement_end_date`) im Text stehen.
    - Paragrafen- oder Abschnittsangaben dürfen NICHT in `point` angegeben werden, ausschließlich in `paragraph_references`. Wenn Paragraphenangaben vorhanden wären, lass sie aus `point` weg => Keine Paragraphenangaben in den User-Facing-Elementen, da sie Laien verwirren würden.
    - Gib den rohen Verweis an, z.B. '§ 5 Abs. 2 Z 3' und zerlege ihn in `parsed_reference_parts`. (Artikel => Article, Paragraph => Paragraph, Abs. => Subsection, Z. => Point, also § 5 Abs. 2 Z 3: Paragraph(5), Subsection(2), Point(3)) Falls eine Referenz z.B. über mehrere Subsections geht, gib alle Teile an. (Abs. 1-3 => Subsection(1), Subsection(2), Subsection(3), oder Abs. 1,4 => Subsection(1), Subsection(4))
    - `paragraph_references` kann nur dann ausgefüllt werden, wenn ein Gesetzestext vorliegt. Diese Referenzen dienen der Nachvollziehbarkeit deiner Stichpunkte!
3.  **Metadaten & Einordnung:**
    - Klassifiziere das Thema anhand dieser Liste (max 10): {topics:?}
    - Bestimme die Komplexität (`complexity_scope_of_proposal`) unter Verwendung der Enum-Werte: Highest, High, Medium, Low, Lowest.
    - Extrahiere allgemeine Daten für `general_enforcement_start_end`, sofern sie für den gesamten Antrag gelten.
4.  **Kritische Analyse:**
    - Erstelle Pro- und Contra-Argumente (`arguments_for`, `arguments_against`).
    - Bestimme den Tonfall des Antrags (`tone`) aus den Werten: Neutral, Urgent, Aggressive, Optimistic, Bureaucratic.
5.  **Finanzielle Analyse (`fiscal_analysis`):**
    - Schätze die Kosten in Millionen Euro (pro Jahr/Monat), falls Daten vorhanden sind.
    - Wähle für `economic_burden` exakt einen dieser Werte: HighCost, ModerateCost, LowCost, Neutral, RevenueGenerating, Unclear.
6.  **Gesellschaftliche Relevanz:**
    - Formuliere kritische politische Fragen (`general_political_questions`).
    - Erstelle Ja/Nein-Fragen für einen Polit-Kompass-Test (`political_compass_questions`).
7.  **Glossar:** Erkläre schwierige Begriffe (`difficult_terms`). Erkläre lieber mehr Begriffe als zu wenige. Speichere den erklärten Begriff in `term` und die zugehörige Definition in `simple_definition` ab.

### FORMALE REGELN:
- Sprache: Ausschließlich Deutsch.
- Zielgruppe: Durchschnittliche Personen in Österreich (barrierefrei, einfach erklärt, keine Paragraphen, keine Paragraphen- und Absatzverweise in Texten).
- Datumsformat: YYYY-MM-DD.
- Jedes Datum muss maschinenlesbar sein und demnach im angegebenen Format sein. Zusätzliche Erläuterungen und Informationen können in `start_notes` und `end_notes` gegeben werden.
- Falls KEINE relevanten Dokumente, die den Antrag beschreiben, im Kontext gefunden wurden: Setze `summary` auf null und `abort_reason` auf 'NoDocumentsProvidedInContext'. Ansonsten, falls `summary` NICHT null ist, ist `abort_reason` 'None'.
- Halte dich strikt an die Feldnamen und Enum-Vorgaben des bereitgestellten JSON-Schemas.
            "),
            None,
            false,
       Some(|schema: &mut serde_json::Value| {
            // TODO: activate topic patching when #143 (https://github.com/som-es/somes/issues/143) is done
            // patch_topics(schema, &topics);
        })
        )
        .await?)
}
