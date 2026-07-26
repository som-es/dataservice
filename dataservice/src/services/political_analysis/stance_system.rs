use crate::{
    LATEST_GP,
    political_analysis::{
        EmbeddingProvider,
        delegate_context::{collect_delegate_profile, collect_spoken_texts},
    },
};
use qdrant_client::Qdrant;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use somai_client::SomAiClient;
use somes_scraper::today;
use sqlx::prelude::{FromRow, Type};

pub fn create_stance_system_prompt(question: &str, context: &str, profile: &str) -> String {
    let today = today();
    format!(
        r#"
    Du bist ein KI-Experte für politische Analyse und Stance Detection.
    Deine Aufgabe ist es, die inhaltliche Position (Stance) eines Politikers zu einer spezifischen Aussage oder Frage präzise zu ermitteln – basierend ausschließlich auf dem bereitgestellten Kontext.

    ZU BEWERTENDE FRAGE / AUSSAGE:
    "{question}"

    REGELN FÜR DIE INHALTLICHE GENERIERUNG:
    1. Direkter Einstieg (`political_stance`): Beginne sofort mit der Position des Politikers zur gestellten Frage. Lass unnötige Einleitungen weg. Schreibe einen einzigen, zusammenhängenden Absatz (keine Stichpunkte).
    2. Dritte Person: Für Außenstehende muss beim Lesen klar sein, dass die Haltung aus einer externen Analyse stammt (z. B. "Der Abgeordnete argumentiert, dass...").
    3. Zero Hallucination: Erfinde absolut nichts. Deine Analyse muss zu 100 % aus dem bereitgestellten Kontext ableitbar sein.
    4. Fehlende Informationen: Wenn der Kontext leer ist oder keine Äußerungen zur Frage enthält, schreibe in `political_stance`: "Die/Der Abgeordnete hat sich in den vorliegenden Äußerungen nicht explizit zu dieser Frage positioniert." In diesem Fall muss `stance_detection` zwingend auf "Neutral" gesetzt werden.

    REGELN FÜR DIE KLASSIFIZIERUNG (`stance_detection`):
    Werte das in `political_stance` erarbeitete Verhältnis zur gestellten Frage/Aussage aus. Erlaubte Werte sind NUR: "Positive", "Negative" oder "Neutral".
    - Trage "Positive" ein, wenn der Politiker DAFÜR ist, der Aussage ZUSTIMMT oder die Maßnahme UNTERSTÜTZT.
    - Trage "Negative" ein, wenn der Politiker DAGEGEN ist, der Aussage WIDERSPRICHT oder die Maßnahme ABLEHNT.
    - Trage "Neutral" ein, wenn er sich nicht eindeutig positioniert, abwägt oder keine Daten vorliegen.

    REGELN FÜR ZITATIONEN (`citations`):
    - Halte dich strikt an die Quellen. Für jede Rede, die du zur Generierung der Haltung herangezogen hast, musst du ein Objekt in das Array einfügen.

    ALLGEMEINE REGELN:
    - Historischer Kontext: Beachte, dass sich Funktionen, Regierungsbeteiligungen und Parteizugehörigkeiten verändert haben können.
    - Sprache: Alle Textwerte im JSON müssen zwingend auf Deutsch verfasst sein (außer die Keys und die vorgegebenen englischen Labels wie "Positive").

    DATENGRUNDLAGE:
    Kontext (Redesnippets des Politikers):
    {context}

    JSON Steckbrief (Informationen zum Politiker):
    {profile}

    Allgemeine System-Informationen:
    Heute ist der {today}. Aktuelle Legislaturperiode: {LATEST_GP}. Aktuelle Regierung: ÖVP-SPÖ-NEOS.
    "#
    )
}

pub async fn populate_system_prompt(
    qdrant_client: &Qdrant,
    embedder: &impl EmbeddingProvider,
    delegate_id: i32,
    question: &str,
) -> crate::Result<String> {
    let spoken_text = collect_spoken_texts(qdrant_client, embedder, delegate_id, question).await?;
    let http_client = reqwest::Client::builder().build()?;
    let delegate_profile = collect_delegate_profile(&http_client, delegate_id).await?;
    Ok(create_stance_system_prompt(
        question,
        &spoken_text,
        &delegate_profile,
    ))
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(JsonSchema, Hash, Debug, Deserialize, PartialEq, Serialize, Clone, Type)]
pub enum StanceDetection {
    Postive,
    Neutral,
    Negative,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(JsonSchema, Hash, Debug, Deserialize, PartialEq, Serialize, Clone, FromRow, Type)]
pub struct Citation {
    pub point_uuid: String,
    pub speech_id: i32,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(JsonSchema, Hash, Debug, Deserialize, PartialEq, Serialize, Clone, FromRow, Type)]
pub struct Stance {
    pub political_stance: String,
    pub stance_detection: StanceDetection,
    pub citations: Vec<Citation>,
}

pub async fn rag_pipeline<T: JsonSchema + DeserializeOwned>(
    ai_client: &SomAiClient,
    qdrant_client: &Qdrant,
    delegate_id: i32,
    question: &str,
) -> crate::Result<T> {
    let prompt = populate_system_prompt(qdrant_client, ai_client, delegate_id, question).await?;
    log::info!("stance prompt length: {}", prompt.len());
    Ok(ai_client
        .make_llm_request_with_schema::<T>(
            question.into(),
            prompt,
            None,
            false,
            Some(|_schema: &mut serde_json::Value| {}),
        )
        .await?)
}

#[cfg(test)]
mod tests {
    use qdrant_client::Qdrant;
    use somai_client::SomAiClient;

    use crate::political_analysis::stance_system::{Stance, rag_pipeline};

    #[tokio::test]
    async fn test_rag_pipeline() {
        simple_logger::init_with_level(log::Level::Info).unwrap();
        let ai_client = SomAiClient::new("http://10.30.0.164:8000", "API_USER", "supersicher123!")
            .await
            .unwrap();

        let model = ai_client.get_model().await.unwrap();
        log::info!("model: {model:?}");
        return;

        let qdrant_client = Qdrant::from_url("http://10.30.0.164:6334").build().unwrap();
        let delegate_id = 8242;
        // let question = "Straftäter:innen sollten härter bestraft werden.";
        let question = "Der Staat sollte weniger ins Wirtschaftsleben eingreifen.";
        let question = "Erbschaften sollten besteuert werden.";
        let question = "Wichtige Entscheidungen sollten stärker auf nationaler Ebene getroffen werden und nicht auf EU-Ebene. ";
        let question = "Österreich sollte seine Neutralität aufgeben.";

        let question = "Österreich sollte sich für die Erweiterung der EU einsetzen.";
        let question = "Um Gewalt gegen Frauen zu bekämpfen, müssen traditionelle Männlichkeitsbilder überwunden werden.";
        let question = "Die Polizei sollte die Möglichkeit bekommen, die Messenger-Dienste (WhatsApp, Signal, ...) von Strafverdächtigen zu überwachen. ";
        let question = "Die Rechte von Beschuldigten im Strafverfahren müssen gestärkt werden.";
        let question = "Volksbegehren, die eine bestimmte Anzahl an Unterstützungen erreichen, sollten automatisch zu Volksabstimmungen führen";

        let stance: Stance =
            rag_pipeline::<Stance>(&ai_client, &qdrant_client, delegate_id, question)
                .await
                .unwrap();
        log::info!("stance: {stance:?}");
    }
}
