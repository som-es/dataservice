use crate::{
    political_analysis::{
        delegate_context::{collect_delegate_profile, collect_spoken_texts},
        EmbeddingProvider,
    },
    LATEST_GP,
};
use qdrant_client::Qdrant;
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use somai_client::SomAiClient;
use somes_scraper::today;
use sqlx::prelude::{FromRow, Type};

pub fn create_stance_system_prompt(context: &str, profile: &str) -> String {
    let today = today();
    format!(
        r#"
    Du bist ein KI-Experte für politische Analyse und Stance Detection.
    Deine Aufgabe ist es, die inhaltliche Position (Stance) eines Politikers zu einer spezifischen Frage oder einem Thema präzise zu ermitteln – basierend ausschließlich auf dem bereitgestellten Kontext.

    WICHTIGE ANWEISUNG ZUM AUSGABEFORMAT:
    Du musst deine Antwort zwingend als valides JSON-Objekt formatieren.

    REGELN FÜR DIE INHALTLICHE GENERIERUNG:
    1. Direkter Einstieg: Beginne im Feld `political_stance` sofort mit der Position des Politikers. Lass unnötige Einleitungen (wie "Die Position ist...") weg.
    2. Textformat: Liste Argumente NICHT in Stichpunkten auf. Schreibe stattdessen einen einzigen, zusammenhängenden ausführlichen Absatz mit mehreren Sätzen zur Begründung.
    3. Zero Hallucination: Erfinde absolut nichts. Deine Analyse muss zu 100 % aus dem bereitgestellten Kontext ableitbar sein.
    4. Fehlende Informationen: Wenn der Kontext leer ist oder keine Äußerungen zur Frage enthält, schreibe in `political_stance` exakt dies (Beispiel: "Die Abgeordnete XY hat in ihren Äußerungen nie eine explizite Position zur Frage Z geäußert."). In diesem Fall muss `stance_detection` auf "Neutral" gesetzt werden.
    5. "stance_detection" ist immer im Bezug zur Frage: Falls die Haltung positiv gegenüber der Frage ist, trage "Positive" bei "stance_detection" ein, ansonsten "Negative".
    6. Zitationen: Halte dich strikt an die Quellen. Für jede Rede, die du zur Generierung der Haltung verwendet hast, musst du ein Objekt in das `citations`-Array einfügen (mit der jeweiligen `point_uuid` und `speech_id`).
    7. Historischer Kontext: Beachte bei der Analyse, dass sich Funktionen, Regierungsbeteiligungen und Parteizugehörigkeiten im Laufe der Zeit verändert haben können.
    8. Sprache: Alle generierten Textwerte im JSON müssen zwingend auf Deutsch verfasst sein.

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
    Ok(create_stance_system_prompt(&spoken_text, &delegate_profile))
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
    point_uuid: String,
    speech_id: i32,
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

    use crate::political_analysis::stance_system::{rag_pipeline, Stance};

    #[tokio::test]
    async fn test_rag_pipeline() {
        simple_logger::init_with_level(log::Level::Info).unwrap();
        let ai_client = SomAiClient::new("http://10.30.0.164:8000", "API_USER", "supersicher123!")
            .await
            .unwrap();

        let qdrant_client = Qdrant::from_url("http://10.30.0.164:6334").build().unwrap();
        let delegate_id = 8242;
        let question = "Der Staat sollte weniger ins Wirtschaftsleben eingreifen.";

        let stance: Stance =
            rag_pipeline::<Stance>(&ai_client, &qdrant_client, delegate_id, question)
                .await
                .unwrap();
        log::info!("stance: {stance:?}");
    }
}
