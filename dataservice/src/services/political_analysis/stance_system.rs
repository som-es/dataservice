use crate::{
    political_analysis::{
        delegate_context::{collect_delegate_profile, collect_spoken_texts},
        EmbeddingProvider,
    },
    LATEST_GP,
};
use qdrant_client::Qdrant;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use somai_client::SomAiClient;
use somes_scraper::today;
use sqlx::prelude::{FromRow, Type};

pub fn create_stance_system_prompt(context: &str, profile: &str) -> String {
    let today = today();
    format!(
        r#"
        Du bist ein Experte für politische Analyse und Stance Detection.
        Deine Aufgabe ist es, basierend auf einer bereitgestellten Frage oder Themen und einem gegebenen Kontext, die Position des angegebenen Politikers zu analysieren.
        Achte darauf, die Haltung klar erkenntlich für ein Stance Detection System in der Erklärung der Position einzubauen. Dabei muss jede Antwort auf Deutsch erfolgen

        WICHTIG - FOLGE DIESER ANTWORT STRUKTUR:
        1. Fang direkt mit der Position des Politikers an, ohne unnötige Einleitung
        2. Erwähne direkt die Position ohne drum-herum!
        3. Liste mögliche Punkte NICHT auf, sondern schreib einen einzigen Absatz über die Position
        4. Falls keine Äußerungen zum Untermauern der Position vorhanden sind, dann breche ab. Beispiel: "Die SPÖ-Abgeordnete Michaela Stefans hat in ihren Äußerungen nie eine explizite Position zur Aufgabe der Neutralität Österreichs geäußert"
        5. Erfinde nichts. Beziehe dich rein auf den folgenden Kontext! Wenn der Kontext demnach leer ist, oder nicht genügend Informationen enthält, dann kannst du auch nicht korrekt antworten!
        6. Beachte, dass manche Informationen, wie z.B. welche Regierungsparteien gerade im Amt ist, sich im Laufe der Zeit verändert haben.
        7. Schreibe die schriftliche Position, die aus den Reden generiert wurde, in "political_stance". Ob diese Position bzw. Haltung positiv, neutral oder negativ ist, ist in "stance_detection" einzutragen.
        8. Halte dich dringend an diese Zitationsregel: Speichere die Identifikationdaten für die generierte Haltung verwendeten Reden in "citations" ab. Point-ID => "point_uuid", Rede-ID => "speech_id"

        Kontext (in Form von Redesnippets des Politkers): '{context}'
        JSON Steckbrief (beinhaltet Informationen zum angegebenen Politker): '{profile}'
        Allgemeine Informationen: Heute ist der {today}, aktuelle Legislaturperiode: {LATEST_GP}, Regierung aus ÖVP-SPÖ-NEOS

        Antworte vollständig in deutscher Sprache!
        Wie erwähnt: Erfinde nichts! Beziehe dich rein auf den folgenden Kontext! Wenn der Kontext demnach leer ist, oder nicht genügend Informationen für die Beantwortung der Frage enthält, dann kannst du auch nicht auf die Position eingehen!
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
    Pro,
    Neutral,
    Contra,
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
    political_stance: String,
    stance_dection: StanceDetection,
    citations: Vec<Citation>,
}

pub async fn rag_pipeline(
    ai_client: &SomAiClient,
    qdrant_client: &Qdrant,
    delegate_id: i32,
    question: &str,
) -> crate::Result<()> {
    let prompt = populate_system_prompt(qdrant_client, ai_client, delegate_id, question).await?;
    log::info!("stance prompt length: {}", prompt.len());
    let res = ai_client
        .make_llm_request_with_schema::<Stance>(
            question.into(),
            prompt,
            None,
            false,
            Some(|_schema: &mut serde_json::Value| {}),
        )
        .await?;
    dbg!(&res);
    Ok(())
}

#[cfg(test)]
mod tests {
    use qdrant_client::Qdrant;
    use somai_client::SomAiClient;

    use crate::political_analysis::stance_system::rag_pipeline;

    #[tokio::test]
    async fn test_rag_pipeline() {
        simple_logger::init_with_level(log::Level::Info).unwrap();
        let ai_client = SomAiClient::new("http://10.30.0.164:8000", "API_USER", "supersicher123!")
            .await
            .unwrap();

        let qdrant_client = Qdrant::from_url("http://10.30.0.164:6334").build().unwrap();
        let delegate_id = 8242;
        let question = "Der Staat sollte weniger ins Wirtschaftsleben eingreifen.";

        rag_pipeline(&ai_client, &qdrant_client, delegate_id, question)
            .await
            .unwrap();
    }
}
