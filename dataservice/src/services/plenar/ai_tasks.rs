use combx::{
    AiSummary, Error::Custom, Keypoint, SpeechAbortReason, SpeechAiSummarizeOutput,
    SpeechAiSummary, SpeechKeypoint, SpeechRelations, SpeechRelationsToProposalOutput,
};
use somai_client::SomAiClient;
use somes_scraper::language::Language;
use sqlx::types::Json;

use crate::ai_task::TaskArgs;

pub async fn summarize_speech_and_relate(
    task_args: TaskArgs,
    spoken_text: String,
    language: Language,
) -> crate::Result<()> {
    let summary_entry = sqlx::query!(
        r#"
            select
                id,
                full_speech_summary as "full_speech_summary: Json<SpeechAiSummary>"
            from speech_ai_summaries
            where speech_id = $1
        "#,
        task_args.db_entry_id
    )
    .fetch_optional(&task_args.pg_pool)
    .await?;

    let (speech_ai_summary_id, summary) = match summary_entry {
        Some(entry) => (entry.id, entry.full_speech_summary.0),
        None => {
            if spoken_text.len() < 50 {
                log::error!(
                    "Cannot summarize speech {}, spoken text is too short / empty",
                    task_args.db_entry_id
                );
                Err(combx::Error::Custom(
                    "Spoken text too short for speech summarization".to_string(),
                ))?;
            }

            let output =
                speech_summary_prompt(&task_args.som_ai_client, spoken_text, language).await?;
            if output.abort_reason != SpeechAbortReason::None || output.summary.is_none() {
                log::error!(
                    "No speech? provided in context for speech {}, abort reason: {:?}, skipping",
                    task_args.db_entry_id,
                    output.abort_reason
                );
                Err(combx::Error::Custom(
                    "No speech provided in context, skipping".to_string(),
                ))?;
            }
            let summary = output.summary.unwrap();
            log::info!("summarized speech: {}", summary.short_title);

            let summary_id = insert_speech_ai_summary(
                &task_args.pg_pool,
                task_args.db_entry_id,
                &summary,
                &task_args.som_ai_client.get_model().await?.result,
                "1",
            )
            .await?;
            (summary_id, summary)
        }
    };

    let proposal_summaries = sqlx::query!(
        r#"
            select
                psli.legis_init_id as "legis_init_id!",
                s.full_summary as "full_summary: Json<AiSummary>"
            from plenar_speech_legis_inits psli
            join legislative_initiative_summaries lis on lis.legis_init_id = psli.legis_init_id
            join summaries s on s.id = lis.summary_id
            where psli.speech_id = $1
        "#,
        task_args.db_entry_id
    )
    .fetch_all(&task_args.pg_pool)
    .await?;

    for proposal in proposal_summaries {
        if sqlx::query_scalar!(
            "select id from speech_proposal_relations where speech_ai_summary_id = $1 and legis_init_id = $2",
            speech_ai_summary_id,
            proposal.legis_init_id
        )
        .fetch_optional(&task_args.pg_pool)
        .await?
        .is_some()
        {
            continue;
        }

        let output = speech_relations_prompt(
            &task_args.som_ai_client,
            &summary.key_points,
            &proposal.full_summary.key_points,
            &proposal.full_summary.short_summary,
            &proposal.full_summary.detailed_summary,
        )
        .await?;

        if output.abort_reason != SpeechAbortReason::None || output.speech_relations.is_none() {
            log::error!(
                "No speech relations generated for speech {} and legis init {}, abort reason: {:?}, skipping",
                task_args.db_entry_id,
                proposal.legis_init_id,
                output.abort_reason
            );
            continue;
        }

        insert_speech_relations(
            &task_args.pg_pool,
            speech_ai_summary_id,
            proposal.legis_init_id,
            &output.speech_relations.unwrap(),
            &task_args.som_ai_client.get_model().await?.result,
            "1",
        )
        .await?;
    }

    Ok(())
}

pub async fn speech_summary_prompt(
    somai_client: &SomAiClient,
    input: String,
    language: Language,
) -> crate::Result<SpeechAiSummarizeOutput> {
    let system_prompt = match language {
        Language::De => "
                        Du bist ein Experte für die Analyse von Parlamentsdaten. Deine Aufgabe ist es, eine Rede aus dem jeweiligen Gremium (EU-Parlament, Nationalrat, ..) für Laien verständlich aufzubereiten und in ein präzises JSON-Format zu strukturieren.

            ### DEINE AUFGABEN:
            1.  **Zusammenfassung:**
                - Erstelle einen kurzen, sprechenden Titel, der das Thema und die Position der Rede prägnant zusammenfasst (`short_title`).
                - Erstelle Zusammenfassungen der Rede in aufsteigender Ausführlichkeit:
                    - `one_sentence_short_summary`: genau ein Satz.
                    - `very_short_summary`: maximal 2 Sätze.
                    - `short_summary`: maximal 2-3 Sätze.
                    - `summary`: 1,5x bis 2x so lang wie `short_summary`.
                    - `detailed_summary`: 2x bis 3x so lang wie `short_summary`.
                    - `very_detailed_summary`: 3x bis 4x so lang wie `short_summary`.
                - Gib die Kernaussagen und die Position der redenden Person wieder, nicht den Wortlaut.
            2.  **Key Points:** Erstelle 3-8 Stichpunkte (`key_points`). Beginne oberflächlich und werde graduell detaillierter.
                - `summarized_point`: der zusammengefasste Stichpunkt.
                - `unmodified_reference_point`: die unveränderte, wörtliche Textstelle der Rede, auf der der Stichpunkt basiert. Übernehme sie GENAU VOM VORLIEGENDEN TEXT. Kürze nichts! Diese Referenzen dienen der Nachvollziehbarkeit deiner Stichpunkte!
            3.  **Kritische Analyse (`critical_analysis`):**
                - Erstelle Argumente, die für bzw. gegen die Position der redenden Person sprechen (`arguments_for`, `arguments_against`).
                - Bestimme den Tonfall der Rede (`tone`) aus den Werten: Neutral, Urgent, Aggressive, Optimistic, Bureaucratic.
            4.  **Glossar:** Erkläre schwierige Begriffe (`difficult_terms`). Erkläre lieber mehr Begriffe als zu wenige. Speichere den erklärten Begriff in `term` und die zugehörige Definition in `simple_definition` ab. Achte darauf, dass die Definition direkt beginnt, ohne den eigentlichen zu definierenden Term anfangs zu wiederholen.

            ### FORMALE REGELN:
            - Sprache: Von dem Input, also der gegebenen Rede, übernehmen.
            - Zielgruppe: Durchschnittliche Personen in Europa (barrierefrei, einfach erklärt, keine Paragraphenverweise in Texten).
            - Zwischenrufe und Regieanmerkungen (z.B. 'Beifall bei ...') gehören nicht zum Inhalt der Rede.
            - Falls KEINE Rede im Kontext gefunden wurde: Setze `summary` auf null und `abort_reason` auf 'NoSpeechProvidedInContext'. Ansonsten, falls `summary` NICHT null ist - das ist der Normalfall - , ist `abort_reason` 'None'.
            - Halte dich strikt an die Feldnamen und Enum-Vorgaben des bereitgestellten JSON-Schemas.
        ",
        Language::En => "
        You are an expert in analyzing parliamentary data. Your task is to process a speech from the respective body (EU Parliament, National Council, ...) so that it is understandable for laypeople and to structure it into a precise JSON format.

        ### YOUR TASKS:

        1. **Summary:**
            - Create a short, catchy title that concisely summarizes the topic and position of the speech (`short_title`).
            - Create summaries of the speech in increasing levels of detail:
                - `one_sentence_short_summary`: exactly one sentence.
                - `very_short_summary`: maximum 2 sentences.
                - `short_summary`: maximum 2-3 sentences.
                - `summary`: 1.5x to 2x as long as `short_summary`.
                - `detailed_summary`: 2x to 3x as long as `short_summary`.
                - `very_detailed_summary`: 3x to 4x as long as `short_summary`.
            - Convey the core statements and the position of the speaker, not the literal wording.

        2. **Key Points:** Create 3-8 bullet points (`key_points`). Start superficially and become gradually more detailed.
            - `summarized_point`: the summarized bullet point.
            - `unmodified_reference_point`: the unchanged, verbatim passage from the speech on which the bullet point is based. Take it EXACTLY FROM THE SOURCE TEXT. Do not shorten anything! These references serve to make your bullet points traceable!

        3. **Critical Analysis (`critical_analysis`):**
            - Create arguments that speak for or against the speaker's position (`arguments_for`, `arguments_against`).
            - Determine the tone of the speech (`tone`) from the following values: Neutral, Urgent, Aggressive, Optimistic, Bureaucratic.

        4. **Glossary:** Explain difficult terms (`difficult_terms`). Prefer explaining more terms rather than too few. Store the explained term in `term` and the corresponding definition in `simple_definition`. Make sure the definition begins directly, without repeating the actual term being defined at the start.

        ### FORMAL RULES:
        - Language: Adopt from the input, i.e., the given speech.
        - Target audience: Average people in Europe (accessible, simply explained, no references to legal paragraphs in the texts).
        - Interjections and stage directions (e.g., 'Applause from ...') are not part of the content of the speech.
        - If NO speech was found in the context: Set `summary` to null and `abort_reason` to 'NoSpeechProvidedInContext'. Otherwise, if `summary` is NOT null — which is the normal case — `abort_reason` is 'None'.
        - Strictly adhere to the field names and enum specifications of the provided JSON schema.
        ",
        _ => return Err(Custom("language not supported".into()).into()),
    };
    somai_client
        .make_llm_request_with_schema::<SpeechAiSummarizeOutput>(
            input,
            system_prompt.to_string(),
            None,
            false,
            Some(|_: &mut serde_json::Value| {}),
        )
        .await
}

pub async fn speech_relations_prompt(
    somai_client: &SomAiClient,
    speech_key_points: &[SpeechKeypoint],
    proposal_key_points: &[Keypoint],
    proposal_summary: &str,
    detailed_proposal_summary: &str,
) -> crate::Result<SpeechRelationsToProposalOutput> {
    let speech_points = speech_key_points
        .iter()
        .enumerate()
        .map(|(id, key_point)| format!("{id}: {}", key_point.summarized_point))
        .collect::<Vec<_>>()
        .join("\n");
    let proposal_points = proposal_key_points
        .iter()
        .enumerate()
        .map(|(id, key_point)| format!("{id}: {}", key_point.point))
        .collect::<Vec<_>>()
        .join("\n");
    let input = format!(
        "### STICHPUNKTE DER REDE:\n{speech_points}\n\n### STICHPUNKTE DES ANTRAGS:\n{proposal_points}\n\n### ZUSAMMENFASSUNG DES ANTRAGS:\n{proposal_summary}\n\n### DETAILLIERTE ZUSAMMENFASSUNG DES ANTRAGS:\n{detailed_proposal_summary}"
    );

    somai_client
        .make_llm_request_with_schema::<SpeechRelationsToProposalOutput>(
            input,
            "
            Du bist ein Experte für die Analyse österreichischer Parlamentsdaten. Dir werden nummerierte Stichpunkte einer Rede aus dem österreichischen Nationalrat sowie nummerierte Stichpunkte und Zusammenfassungen des zugehörigen Antrags gegeben. Deine Aufgabe ist es, die Rede-Stichpunkte den Antrags-Stichpunkten zuzuordnen und in ein präzises JSON-Format zu strukturieren.

### DEINE AUFGABEN:
1.  **Stichpunkt-Relationen (`propsal_keypoint_relations`):** Erstelle für jeden Rede-Stichpunkt, der sich inhaltlich auf Stichpunkte des Antrags bezieht, eine Relation.
    - `speech_key_point`: die Nummer des Rede-Stichpunkts.
    - `referenced_proposal_key_point_ids`: die Nummern ALLER Antrags-Stichpunkte, auf die sich der Rede-Stichpunkt inhaltlich bezieht.
    - Erstelle KEINE Relation für Rede-Stichpunkte ohne inhaltlichen Bezug zum Antrag. Erfinde KEINE Relationen!
2.  **Zusammenfassungs-Relationen:**
    - `speech_related_to_proposal_summary`: Bezieht sich die Rede inhaltlich auf die Zusammenfassung des Antrags?
    - `speech_related_to_detailed_proposal_summary`: Bezieht sich die Rede inhaltlich auf die detaillierte Zusammenfassung des Antrags?

### FORMALE REGELN:
- Verwende ausschließlich die angegebenen Nummern der Stichpunkte.
- Falls KEINE Rede-Stichpunkte im Kontext gefunden wurden: Setze `speech_relations` auf null und `abort_reason` auf 'NoSpeechProvidedInContext'. Ansonsten, falls `speech_relations` NICHT null ist - das ist der Normalfall - , ist `abort_reason` 'None'.
- Halte dich strikt an die Feldnamen des bereitgestellten JSON-Schemas.
            "
            .to_string(),
            None,
            false,
            Some(|_: &mut serde_json::Value| {}),
        )
        .await
}

pub async fn insert_speech_ai_summary<'c, E>(
    executor: E,
    speech_id: i32,
    summary: &SpeechAiSummary,
    model_used: &str,
    version: &str,
) -> sqlx::Result<i32>
where
    E: sqlx::PgExecutor<'c>,
{
    let rec = sqlx::query!(
        r#"
        insert into speech_ai_summaries (
            speech_id,
            short_title,
            one_sentence_short_summary,
            very_short_summary,
            short_summary,
            summary,
            detailed_summary,
            very_detailed_summary,
            full_speech_summary,
            model_used,
            version
        )
        values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        returning id
        "#,
        speech_id,
        summary.short_title,
        summary.one_sentence_short_summary,
        summary.very_short_summary,
        summary.short_summary,
        summary.summary,
        summary.detailed_summary,
        summary.very_detailed_summary,
        Json(summary) as Json<&SpeechAiSummary>,
        model_used,
        version,
    )
    .fetch_one(executor)
    .await?;

    Ok(rec.id)
}

pub async fn insert_speech_relations<'c, E>(
    executor: E,
    speech_ai_summary_id: i32,
    legis_init_id: i32,
    speech_relations: &SpeechRelations,
    model_used: &str,
    version: &str,
) -> sqlx::Result<i32>
where
    E: sqlx::PgExecutor<'c>,
{
    let rec = sqlx::query!(
        r#"
        insert into speech_proposal_relations (
            speech_ai_summary_id,
            legis_init_id,
            full_speech_relations,
            model_used,
            version
        )
        values ($1, $2, $3, $4, $5)
        returning id
        "#,
        speech_ai_summary_id,
        legis_init_id,
        Json(speech_relations) as Json<&SpeechRelations>,
        model_used,
        version,
    )
    .fetch_one(executor)
    .await?;

    Ok(rec.id)
}

#[cfg(test)]
mod tests {
    use combx::{AiSummary, DbAiSummary, Keypoint};
    use somai_client::SomAiClient;
    use somes_scraper::language::Language;

    use crate::plenar::{speech_relations_prompt, speech_summary_prompt};

    const SPOKEN_TEXT: &str = r#"Herr Präsident! Sehr geehrte Damen und Herren! Hohes Haus! Wir beschließen heute eine Novelle des Erneuerbaren-Ausbau-Gesetzes, und ich sage Ihnen ganz offen: Dieses Gesetz ist überfällig. (Beifall bei den Grünen und bei der ÖVP.) Die Energiekrise der letzten Jahre hat uns allen gezeigt, wie gefährlich die Abhängigkeit von russischem Gas für unsere Haushalte und unsere Betriebe ist. Mit dieser Novelle beschleunigen wir die Genehmigungsverfahren für Windkraft- und Photovoltaikanlagen deutlich. Was bisher sieben Jahre gedauert hat, soll künftig in maximal zwei Jahren möglich sein. Wir schaffen dafür eigene Beschleunigungszonen, in denen die Bundesländer Vorrangflächen für erneuerbare Energie ausweisen müssen. (Zwischenruf des Abg. Müller.) Ja, Herr Kollege, auch die Länder müssen liefern, genau das ist der Punkt! Zusätzlich verankern wir eine verpflichtende Bürgerbeteiligung: Gemeinden und Anrainer sollen künftig direkt von den Erträgen der Anlagen in ihrer Region profitieren. Das schafft Akzeptanz vor Ort, und das ist mir besonders wichtig. Was mir Sorgen bereitet, meine Damen und Herren, ist die Blockadehaltung mancher hier im Haus. Wer heute gegen dieses Gesetz stimmt, stimmt für weiter steigende Strompreise und gegen die Unabhängigkeit unseres Landes. Die Klimaziele 2030 - hundert Prozent Strom aus erneuerbaren Quellen - erreichen wir nur, wenn wir jetzt handeln. Ich ersuche Sie daher um breite Zustimmung zu dieser Novelle. Vielen Dank. (Beifall bei den Grünen und bei der ÖVP.)"#;

    #[tokio::test]
    async fn test_speech_summarization() {
        let _ = simple_logger::init_with_level(log::Level::Info);
        let somai_client =
            SomAiClient::new("http://10.30.0.164:8000", "API_USER", "supersicher123!")
                .await
                .unwrap();

        let result =
            speech_summary_prompt(&somai_client, SPOKEN_TEXT.to_string(), Language::De).await;
        dbg!(&result);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_speech_summarization_from_db() {
        let _ = simple_logger::init_with_level(log::Level::Info);
        let somai_client =
            SomAiClient::new("http://10.30.0.164:8000", "API_USER", "supersicher123!")
                .await
                .unwrap();

        let pg_pool = combx::connect_pg().await;
        let spoken_text = sqlx::query_scalar!(
            "select spoken_text from plenar_speeches where spoken_text is not null and length(spoken_text) > 1000 order by id desc limit 1"
        )
        .fetch_one(&pg_pool)
        .await
        .unwrap()
        .unwrap();

        let result = speech_summary_prompt(&somai_client, spoken_text, Language::De).await;
        dbg!(&result);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_speech_proposal_relations_with_existing_vote_result() {
        let _ = simple_logger::init_with_level(log::Level::Info);
        let somai_client =
            SomAiClient::new("http://10.30.0.164:8000", "API_USER", "supersicher123!")
                .await
                .unwrap();

        use sqlx::types::Json;
        let pg_pool = combx::connect_pg().await;

        let db_ai_summary = sqlx::query_as!(
            DbAiSummary,
            r#"select
                        id,
                full_summary as "full_summary: Json<AiSummary>",
                short_title,
                short_summary,
                detailed_summary,
                complexity_scope_of_proposal,
                model_used,
                version,
                generated_at from summaries where id = $1"#,
            3396
        )
        .fetch_one(&pg_pool)
        .await
        .unwrap();

        let spoken_text = sqlx::query_scalar!("select spoken_text from plenar_speeches where id = 615902168 and spoken_text is not null").fetch_one(&pg_pool).await.unwrap().unwrap();
        dbg!(&db_ai_summary);
        dbg!(&spoken_text);

        let speech_ai_summary = speech_summary_prompt(&somai_client, spoken_text, Language::De)
            .await
            .unwrap()
            .summary
            .unwrap();
        dbg!(&speech_ai_summary);

        let result = speech_relations_prompt(
            &somai_client,
            &speech_ai_summary.key_points,
            &db_ai_summary.full_summary.key_points,
            &db_ai_summary.full_summary.short_summary,
            &db_ai_summary.full_summary.detailed_summary,
        )
        .await;
        dbg!(&result);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_speech_proposal_relations() {
        let _ = simple_logger::init_with_level(log::Level::Info);
        let somai_client =
            SomAiClient::new("http://10.30.0.164:8000", "API_USER", "supersicher123!")
                .await
                .unwrap();

        let summary = speech_summary_prompt(&somai_client, SPOKEN_TEXT.to_string(), Language::De)
            .await
            .unwrap()
            .summary
            .unwrap();

        #[rustfmt::skip]
        let proposal_key_points = vec![
            Keypoint { point: "Genehmigungsverfahren für Windkraft- und Photovoltaikanlagen werden auf maximal zwei Jahre verkürzt.".into(), enforcement_start_end: None, paragraph_references: vec![] },
            Keypoint { point: "Die Bundesländer müssen Beschleunigungszonen mit Vorrangflächen für erneuerbare Energie ausweisen.".into(), enforcement_start_end: None, paragraph_references: vec![] },
            Keypoint { point: "Gemeinden und Anrainer werden verpflichtend an den Erträgen neuer Anlagen beteiligt.".into(), enforcement_start_end: None, paragraph_references: vec![] },
            Keypoint { point: "Netzbetreiber müssen jährlich einen Netzausbauplan veröffentlichen.".into(), enforcement_start_end: None, paragraph_references: vec![] },
            Keypoint { point: "Für Wasserstoff-Pilotprojekte wird ein eigenes Förderkontingent von 40 Millionen Euro pro Jahr eingerichtet.".into(), enforcement_start_end: None, paragraph_references: vec![] },
        ];

        let result = speech_relations_prompt(
            &somai_client,
            &summary.key_points,
            &proposal_key_points,
            "Die Novelle des Erneuerbaren-Ausbau-Gesetzes beschleunigt Genehmigungsverfahren für erneuerbare Energieanlagen und führt Beschleunigungszonen ein. Gemeinden und Anrainer sollen an den Erträgen beteiligt werden.",
            "Die Novelle des Erneuerbaren-Ausbau-Gesetzes verkürzt Genehmigungsverfahren für Windkraft- und Photovoltaikanlagen auf maximal zwei Jahre und verpflichtet die Bundesländer, Beschleunigungszonen mit Vorrangflächen auszuweisen. Gemeinden und Anrainer werden verpflichtend an den Erträgen neuer Anlagen beteiligt, um die Akzeptanz vor Ort zu erhöhen. Netzbetreiber müssen jährlich einen Netzausbauplan veröffentlichen, und für Wasserstoff-Pilotprojekte wird ein eigenes Förderkontingent eingerichtet.",
        )
        .await;
        dbg!(&result);
        assert!(result.is_ok());
    }
}
