use combx::document_text::{combine_texts, extract_text_from_documents_html_first};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use somai_client::SomAiClient;
use somes_common_lib::Document;

use crate::{ai_task::TaskArgs, OCR_ENGINE};

#[derive(JsonSchema, Serialize, Deserialize, Debug)]
pub struct EnforcementDates {
    pub enforcement_start_date: Option<String>,
    pub enforcement_end_date: Option<String>,
}

#[derive(JsonSchema, Serialize, Deserialize, Debug)]
pub struct Keypoint {
    pub point: String,
    pub enforcement_start_end: Option<EnforcementDates>,
    // TODO: references to document sections
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
pub struct Summary {
    pub short_title: String,
    pub short_summary: String,
    pub expressive_summary: String,
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
        combine_texts(
            &extract_text_from_documents_html_first(&inner_docs, OCR_ENGINE.clone()).await,
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

            let summary = summary_prompt(&task_args.som_ai_client, input, &unique_topics).await?;

            sqlx::query!(
        "UPDATE legislative_initiatives SET ai_emphasis = $1, is_emphasis_ai_generated = true WHERE id = $2",
        summary.short_summary,
        task_args.db_entry_id
    )
    .execute(&task_args.pg_pool)
    .await?;
        }
        Err(e) => log::error!("cannot extract text input from {:?} because of {e:?}", docs),
    }

    Ok(())
}

async fn summary_prompt(
    somai_client: &SomAiClient,
    input: String,
    topics: &[String],
) -> combx::Result<Summary> {
    Ok(somai_client
        .make_llm_request_with_schema::<Summary>(input, format!("
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

                    Überlege dir, falls passend, kritische und politische Fragen zu dem Antrag. => general_political_questions

                    Besonders brisante und gesellschaftlich spannende Anträge sollen für einen automatisierten politischen Kompass Test verwendet werden. Schreibe dazu Fragen auf, die durchschnittliche Bürger mit Ja/Nein beantworten können. => political_compass_questions

                    Klassifiziere die behandelnden Themen des Antrags anhand der folgenden Liste, wähle bis zu 10 Themen: {topics:?} => topics

                    Falls ein Glossar für schwierige Begriffe notwendig ist, achte darauf, dass die Definition direkt beginnt, ohne den eigentlichen zu definierenden Term anfangs zu wiederholen => glossary.difficult_terms.simple_definition 

                    Wähle den finanziellen Umfang des Antrags anhand der folgenden Kategorien, setze dabei nicht zu hoch an!: Hoch, Mittel, Niedrig, Neutral, Einnahmenerhöhend, Unklar => fiscal_analysis.economic_burden
                    Schätze die Kosten des Antrags in Millionen Euro pro Jahr oder pro Monat, falls möglich => fiscal_analysis.estimated_cost_per_year_in_million, fiscal_analysis.estimated_cost_per_month_in_million

                    Achte bei den produzierten Informationen auf Maschinenlesbarkeit.
                    Halte dich allgemein an das folgende Ausgabedatumsformat: YYYY-MM-DD
                    Die Informationen müssen von einer durchschnittlichen Person (aus Österreich) verstanden werden können und barrierfrei sein. 
                    "), None, false)
        .await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_summary_prompt() {
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
            combine_texts(&extract_text_from_documents_html_first(&docs, OCR_ENGINE.clone()).await);

        let result = summary_prompt(&somai_client, input, &unique_topics).await;
        assert!(result.is_ok());
        dbg!(result.unwrap());
    }
}
