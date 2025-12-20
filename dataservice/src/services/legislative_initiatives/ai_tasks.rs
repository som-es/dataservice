use combx::document_text::{combine_texts, extract_text_from_documents_html_first};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use somai_client::SomAiClient;
use somes_common_lib::Document;

use crate::{OCR_ENGINE, ai_task::TaskArgs};

#[derive(JsonSchema, Serialize, Deserialize, Debug)]
pub struct EnforcementDates {
    pub enforcement_start_date: String,
    pub start_notes: Option<String>,
    pub enforcement_end_date: Option<String>,
    pub end_notes: Option<String>,
}

#[derive(JsonSchema, Serialize, Deserialize, Debug)]
pub struct Keypoint {
    pub point: String,
    pub enforcement_start_end: Option<EnforcementDates>,
    pub paragraph_references: Vec<EuroLawReference>,
}

#[derive(JsonSchema, Serialize, Deserialize, Debug)]
pub enum ProposalComplexityScope {
    Highest,
    High,
    Medium,
    Low,
    Lowest,
}

#[derive(JsonSchema, Serialize, Deserialize, Debug)]
pub enum Tone {
    Neutral,
    Urgent,
    Aggressive,
    Optimistic,
    Bureaucratic,
}
#[derive(JsonSchema, Serialize, Deserialize, Debug)]
pub struct CriticalAnalysis {
    pub arguments_for: Vec<String>,
    pub arguments_against: Vec<String>,
    pub tone: Tone,
}

#[derive(JsonSchema, Serialize, Deserialize, Debug)]
pub struct TermDefinition {
    pub term: String,
    pub simple_definition: String,
}

#[derive(JsonSchema, Serialize, Deserialize, Debug)]
pub struct Glossary {
    pub difficult_terms: Vec<TermDefinition>,
}

#[derive(JsonSchema, Serialize, Deserialize, Debug)]
pub struct FiscalAnalysis {
    pub estimated_cost_per_year_in_million: Option<f64>,
    pub estimated_cost_per_month_in_million: Option<f64>,
    pub economic_burden: EconomicImpactLevel,
}

#[derive(JsonSchema, Serialize, Deserialize, Debug)]
pub enum EconomicImpactLevel {
    HighCost,
    ModerateCost,
    LowCost,
    Neutral,
    RevenueGenerating,
    Unclear,
}

#[derive(JsonSchema, Serialize, Deserialize, Debug)]
pub struct AiSummary {
    pub short_title: String,
    pub short_summary: String,
    pub detailed_summary: String,
    pub key_points: Vec<Keypoint>,
    pub general_enforcement_start_end: Option<EnforcementDates>,
    pub topics: Vec<String>,
    pub general_political_questions: Vec<String>,
    pub political_compass_questions: Vec<String>,
    pub complexity_scope_of_proposal: ProposalComplexityScope,
    pub critical_analysis: CriticalAnalysis,
    pub glossary: Glossary,
    pub fiscal_analysis: FiscalAnalysis,
}

#[derive(JsonSchema, Serialize, Deserialize, Debug)]
#[serde(tag = "kind", content = "value")]
pub enum LawPart {
    Article(u32),
    Paragraph(u32),
    Literal(char),
    Subsection(u32),
    Point(String),
}

#[derive(JsonSchema, Serialize, Deserialize, Debug)]
pub struct EuroLawReference {
    pub full_unparsed_reference: String,
    pub parsed_reference_parts: Vec<LawPart>,
}

#[derive(JsonSchema, Serialize, Deserialize, Debug)]
pub struct SummarizeOutput {
    pub summary: Option<AiSummary>,
    pub abort_reason: AbortReason,
}

#[derive(JsonSchema, Serialize, Deserialize, Debug)]
pub enum AbortReason {
    NoDocumentsProvidedInContext,
    None,
}

pub async fn summarize_laws(task_args: TaskArgs, docs: &[Document]) -> combx::Result<()> {
    // is emphasis already generated/set? -> skip
    let info = sqlx::query!(
        "select ai_emphasis, is_emphasis_ai_generated from legislative_initiatives where id = $1",
        task_args.db_entry_id
    )
    .fetch_one(&task_args.pg_pool)
    .await?;
    if info.ai_emphasis.is_some() {
        log::info!(
            "Skipping legis init emphasis summarization for id {}, already set.",
            task_args.db_entry_id
        );
        return Ok(());
    }

    let inner_docs = docs.to_vec();
    match tokio::spawn(async move {
        let engine = OCR_ENGINE.clone();
        combine_texts(
            &extract_text_from_documents_html_first(&inner_docs, engine).await,
        )
    })
    .await
    {
        Ok(input) => {
            if input.len() < 50 {
                log::error!(
            "Cannot summarize legis init {}, input is too short / empty for documents: {:?}",
            task_args.db_entry_id, docs
        );
                return Ok(());
            }
            let unique_topics = sqlx::query_scalar!(
                "select topic_name as topic from unique_eurovoc_topics order by topic"
            )
            .fetch_all(&task_args.pg_pool)
            .await?;

            let summary = summary_prompt2(&task_args.som_ai_client, input, &unique_topics).await?;

    //         sqlx::query!(
    //     "UPDATE legislative_initiatives SET ai_emphasis = $1, is_emphasis_ai_generated = true WHERE id = $2",
    //     summary.short_summary,
    //     task_args.db_entry_id
    // )
    // .execute(&task_args.pg_pool)
    // .await?;
        }
        Err(e) => log::error!("cannot extract text input from {:?} because of {e:?}", docs),
    }

    Ok(())
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
                    "), None, false)
        .await?)
}

async fn summary_prompt2(
    somai_client: &SomAiClient,
    input: String,
    topics: &[String],
) -> combx::Result<SummarizeOutput> {
    Ok(somai_client
        .make_llm_request_with_schema::<SummarizeOutput>(
            input,
            format!("
            Du bist ein Experte für die Analyse österreichischer Parlamentsanträge. Deine Aufgabe ist es, komplexe politische Dokumente für Laien verständlich aufzubereiten und in ein präzises JSON-Format zu strukturieren.

### DEINE AUFGABEN:
1.  **Zusammenfassung:** 
    - Erstelle einen kurzen, sprechenden Titel, der das Thema des Antrags prägnant zusammenfasst (`short_title`).
    - Erstelle eine ultrakurze Zusammenfassung (2-3 Sätze) für `short_summary`.
    - Erstelle eine detailliertere Zusammenfassung für `detailed_summary`.
2.  **Key Points:** Erstelle 3-8 Stichpunkte (`key_points`). Beginne oberflächlich und werde graduell detaillierter. 
    - Prüfe für jeden Punkt, ob spezifische Inkrafttretensdaten (`enforcement_start_date`) oder Ablaufdaten (`enforcement_end_date`) im Text stehen.
    - Paragrafen- oder Abschnittsverweise dürfen NICHT in `point` angegeben werden, ausschließlich in `paragraph_references`. 
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
7.  **Glossar:** Erkläre schwierige Begriffe (`difficult_terms`). Die Definition muss direkt ohne Wiederholung des Begriffs starten.

### FORMALE REGELN:
- Sprache: Ausschließlich Deutsch.
- Zielgruppe: Durchschnittliche Personen in Österreich (barrierefrei, einfach erklärt, keine Paragraphen).
- Datumsformat: YYYY-MM-DD.
- Jedes Datum muss maschinenlesbar sein und demnach im angegebenen Format sein. Zusätzliche Erläuterungen und Informationen können in `start_notes` und `end_notes` gegeben werden.
- Falls KEINE relevanten Dokumente, die den Antrag beschreiben, im Kontext gefunden wurden: Setze `summary` auf null und `abort_reason` auf 'NoDocumentsProvidedInContext'. Ansonsten, falls `summary` NICHT null ist, ist `abort_reason` 'None'.
- Halte dich strikt an die Feldnamen und Enum-Vorgaben des bereitgestellten JSON-Schemas.
            "),
            None,
            false,
        )
        .await?)
}

#[cfg(test)]
mod tests {
    use crate::OCR_ENGINE;

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
                document_url: Some("/dokument/XXVIII/I/298/fname_1722547.pdf".into()),
                document_type: Some("DigitalPDF".into()),
            },
            Document {
                title: Some("Erläuterungen".into()),
                document_url: Some("/dokument/XXVIII/I/298/fname_1722549.pdf".into()),
                document_type: Some("DigitalPDF".into()),
            },
            Document {
                title: Some("Gesetzestext".into()),
                document_url: Some("/dokument/XXVIII/I/298/fnameorig_1722547.html".into()),
                document_type: Some("HTML".into()),
            },
            Document {
                title: Some("Erläuterungen".into()),
                document_url: Some("/dokument/XXVIII/I/298/fnameorig_1722549.html".into()),
                document_type: Some("HTML".into()),
            },
            Document {
                title: Some("Textgegenüberstellung".into()),
                document_url: Some("/dokument/XXVIII/I/298/imfname_1722550.pdf".into()),
                document_type: Some("DigitalPDF".into()),
            },
            Document {
                title: Some("Vorblatt und WFA".into()),
                document_url: Some("/dokument/XXVIII/I/298/imfname_1722551.pdf".into()),
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
            combine_texts(&extract_text_from_documents_html_first(&docs, None).await);

        let result = summary_prompt2(&somai_client, input, &unique_topics).await;
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
                document_url: Some("/dokument/XXVIII/I/308/fname_1723244.pdf".into()),
                document_type: Some("DigitalPDF".into()),
            },
            Document {
                title: Some("Erläuterungen".into()),
                document_url: Some("/dokument/XXVIII/I/308/fname_1723246.pdf".into()),
                document_type: Some("DigitalPDF".into()),
            },
            Document {
                title: Some("Gesetzestext".into()),
                document_url: Some("/dokument/XXVIII/I/308/fnameorig_1723244.html".into()),
                document_type: Some("HTML".into()),
            },
            Document {
                title: Some("Erläuterungen".into()),
                document_url: Some("/dokument/XXVIII/I/308/fnameorig_1723246.html".into()),
                document_type: Some("HTML".into()),
            },
            Document {
                title: Some("Textgegenüberstellung".into()),
                document_url: Some("/dokument/XXVIII/I/308/imfname_1723247.pdf".into()),
                document_type: Some("DigitalPDF".into()),
            },
            Document {
                title: Some("Vorblatt und WFA".into()),
                document_url: Some("/dokument/XXVIII/I/308/imfname_1723248.pdf".into()),
                document_type: Some("DigitalPDF".into()),
            },
            Document {
                title: Some("Anlage 1 zum NISG 2026".into()),
                document_url: Some("/dokument/XXVIII/I/308/imfname_1723249.pdf".into()),
                document_type: Some("DigitalPDF".into()),
            },
            Document { 
                title: Some("Anlage 2 zum NISG 2026".into()),
                document_url: Some("/dokument/XXVIII/I/308/imfname_1723250.pdf".into()),
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

        let result = summary_prompt2(&somai_client, input, &unique_topics).await;
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
                document_url: Some("/dokument/XXVIII/I/269/fname_1718387.pdf".into()),
                document_type: Some("DigitalPDF".into()),
            },
            Document {
                title: Some("Erläuterungen".into()),
                document_url: Some("/dokument/XXVIII/I/269/fname_1718389.pdf".into()),
                document_type: Some("DigitalPDF".into()),
            },
            Document {
                title: Some("Textgegenüberstellung".into()),
                document_url: Some("/dokument/XXVIII/I/269/fname_1718390.pdf".into()),
                document_type: Some("DigitalPDF".into()),
            },
            Document {
                title: Some("Gesetzestext".into()),
                document_url: Some("/dokument/XXVIII/I/269/fnameorig_1718387.html".into()),
                document_type: Some("HTML".into()),
            },
            Document {
                title: Some("Erläuterungen".into()),
                document_url: Some("/dokument/XXVIII/I/269/fnameorig_1718389.html".into()),
                document_type: Some("HTML".into()),
            },
            Document {
                title: Some("Textgegenüberstellung".into()),
                document_url: Some("/dokument/XXVIII/I/269/fnameorig_1718390.html".into()),
                document_type: Some("HTML".into()),
            },
            Document {
                title: Some("Vorblatt und WFA".into()),
                document_url: Some("/dokument/XXVIII/I/269/imfname_1718391.pdf".into()),
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
            combine_texts(&extract_text_from_documents_html_first(&docs, None).await);
        let result = summary_prompt2(&somai_client, input, &unique_topics).await;
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

        let docs = [
            Document {
                title: Some("Selbständiger Entschließungsantrag (gescanntes Original)".into()),
                document_url: Some("/dokument/XXVIII/A/568/imfname_1722828.pdf".into()),
                document_type: Some("OriginalPDF".into()),
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
        let result = summary_prompt2(&somai_client, input, &unique_topics).await;
        assert!(result.is_ok());
        dbg!(result.unwrap());
    }
}
