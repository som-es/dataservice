use chrono::{DateTime, Utc};
use combx::{
    match_delegate_name_with_delegate_id, with_data::active_delegates_at_date, DbDelegate,
    DelegateMatching, Interjection,
};
use somes_scraper::PlenarSpeech;
use sqlx::Postgres;

const DASH: char = '–';
const DASH_STR: &str = "–";

pub async fn extract_interjections<'a>(
    params: Vec<String>,
    plenar_speech: PlenarSpeech,
    plenar_date: DateTime<Utc>,
    pool: &mut sqlx::Transaction<'a, Postgres>,
    plenar_speech_id: i32,
) -> Result<(), sqlx::Error> {
    let db_delegates = active_delegates_at_date(&mut **pool, plenar_date.date_naive()).await?;

    let mut interjections: Vec<Interjection> = Vec::new();

    for param in params {
        let extracted =
            extract_interjection(param.as_str(), &plenar_speech, &db_delegates, &plenar_date);

        interjections.extend(extracted);
    }

    save_interjection(interjections, pool, plenar_speech_id).await
}

pub async fn save_interjection<'a>(
    interjections: Vec<Interjection>,
    pool: &mut sqlx::Transaction<'a, Postgres>,
    plenar_speech_id: i32,
) -> Result<(), sqlx::Error> {
    for interjection in interjections {
        let delegate_matching_id: i32 = sqlx::query!(
            r#"
            INSERT INTO delegate_matching (
                similiarity_score,
                searched_with,
                matched_with,
                delegate_id, 
                manually_matched
            )
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id
            "#,
            interjection.delegate_matching.similiarity_score,
            interjection.delegate_matching.searched_with,
            interjection.delegate_matching.matched_with,
            interjection.delegate_matching.delegate_id,
            interjection.delegate_matching.manually_matched,
        )
        .fetch_one(&mut **pool)
        .await?
        .id;

        sqlx::query!(
            r#"
            INSERT INTO interjections (
                interjection_text,
                interjector_delegate_id,
                plenar_speech_id,
                delegate_matching_id
            )
            VALUES ($1, $2, $3, $4)
            "#,
            interjection.interjection_text,
            interjection.interjector_delegate_id,
            plenar_speech_id,
            delegate_matching_id,
        )
        .execute(&mut **pool)
        .await?;
    }

    Ok(())
}

fn extract_interjection(
    param: &str,
    plenar_speech: &PlenarSpeech,
    db_delegate: &Vec<DbDelegate>,
    plenar_date: &DateTime<Utc>,
) -> Vec<Interjection> {
    let parts = extract_interjections_parts(param);

    let mut result = Vec::new();

    for (name, text) in parts {
        let delegate_results = match_delegate_name_with_delegate_id(
            db_delegate.iter(),
            plenar_date.date_naive(),
            &name,
            true,
        );

        if let Some(best) = delegate_results.get(0) {
            let delegate_matching = DelegateMatching {
                similiarity_score: best.similiarity_score as i64,
                searched_with: best.searched_with.clone(),
                matched_with: best.matched_with.clone(),
                delegate_id: best.delegate_id as i32,
                manually_matched: best.manually_matched,
            };

            result.push(Interjection {
                interjection_text: text,
                interjector_delegate_id: best.delegate_id as i32,
                plenar_speech: plenar_speech.clone(),
                delegate_matching,
            });
        }
    }

    result
}

fn extract_interjections_parts(param: &str) -> Vec<(String, String)> {
    let text = format!("{DASH_STR} {}", param);

    text.split(&format!("{DASH_STR} Abg."))
        .filter_map(|s| {
            let s = s.trim();
            if s.is_empty() {
                return None;
            }

            let (name, text) = s.split_once(':')?;

            let mut name = name.trim().to_string();

            // 👉 Spezialfall: "Name – irgendwas –"
            if name.matches(DASH).count() >= 2 {
                name = name
                    .split(DASH)
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string();
            }

            Some((name, text.trim().to_string()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_extraction(input: &str, expected: Vec<(&str, &str)>) {
        let result = extract_interjections_parts(input);

        let expected: Vec<(String, String)> = expected
            .into_iter()
            .map(|(n, t)| (n.to_string(), t.to_string()))
            .collect();

        assert_eq!(result, expected);
    }

    #[test]
    fn test_interjection_extraction_1() {
        assert_extraction(
            "Abg. Hallo: Das ist eine Frechheit!",
            vec![("Hallo", "Das ist eine Frechheit!")],
        );
    }

    #[test]
    fn test_interjection_extraction_2() {
        assert_extraction(
            "Abg. Schwarz: Stimmt ja nicht! Erst heute hat er wieder ...!",
            vec![("Schwarz", "Stimmt ja nicht! Erst heute hat er wieder ...!")],
        );
    }

    #[test]
    fn test_interjection_extraction_3() {
        assert_extraction("Abg. Stefan: Geh!", vec![("Stefan", "Geh!")]);
    }

    #[test]
    fn test_interjection_extraction_4() {
        assert_extraction(
            "Abg. Keck: Sie sind auch kein Bürgermeister!",
            vec![("Keck", "Sie sind auch kein Bürgermeister!")],
        );
    }

    #[test]
    fn test_interjection_extraction_5() {
        assert_extraction(
            "Abg. Meinl-Reisinger: Die Wirtshausprämie könnte man auch ...! Die braucht man auch nicht!",
            vec![(
                "Meinl-Reisinger",
                "Die Wirtshausprämie könnte man auch ...! Die braucht man auch nicht!",
            )],
        );
    }

    #[test]
    fn test_interjection_extraction_6_multiple() {
        assert_extraction(
            "Beifall bei der ÖVP. – Abg. Plessl: Was sagen Sie zum Vertrag 2002? Eine Nebelgranate! Das Problem Vertragsgestaltung 2002 sprechen Sie gar nicht an! – Abg. Steinacker – auf dem Weg zu ihrem Sitzplatz –: Aus gut mach schlecht!",
            vec![
                (
                    "Plessl",
                    "Was sagen Sie zum Vertrag 2002? Eine Nebelgranate! Das Problem Vertragsgestaltung 2002 sprechen Sie gar nicht an!",
                ),
                ("Steinacker", "Aus gut mach schlecht!"),
            ],
        );
    }

    #[test]
    fn test_interjection_extraction_with_inserted_text() {
        assert_extraction(
            "Abg. Steinacker – auf dem Weg zu ihrem Sitzplatz –: Aus gut mach schlecht!",
            vec![("Steinacker", "Aus gut mach schlecht!")],
        );
    }

    #[test]
    fn test_interjection_extraction_negative_1() {
        assert_extraction("Beifall bei Abgeordneten der SPÖ.", vec![]);
    }

    #[test]
    fn test_interjection_extraction_negative_2() {
        assert_extraction("SPÖ", vec![]);
    }

    #[test]
    fn test_interjection_extraction_negative_3() {
        assert_extraction(
            "Beifall und Bravoruf bei der SPÖ sowie Beifall bei den NEOS.",
            vec![],
        );
    }
}