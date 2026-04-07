use chrono::{DateTime, Utc};
use chrono_tz::America::Paramaribo;
use combx::{
    DbDelegate, DelegateMatching, Interjection, match_delegate_name_with_delegate_id, with_data::active_delegates_at_date
};
use regex::Regex;
use somes_scraper::PlenarSpeech;
use sqlx::Postgres;

pub async fn extract_interjections<'a>(
    params: Vec<String>,
    plenar_speech: PlenarSpeech,
    plenar_date: DateTime<Utc>,
    pool: &mut sqlx::Transaction<'a, Postgres>,
    plenar_speech_id: i32,
) -> Result<(), sqlx::Error> {
    let db_delegates = active_delegates_at_date(&mut **pool, plenar_date.date_naive())
        .await
        .unwrap();

    let mut interjections: Vec<Interjection> = Vec::new();

    for param in params {
        interjections.push(
            extract_interjection(param.as_str(), &plenar_speech, &db_delegates, &plenar_date)
                .unwrap(),
        )
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

pub fn extract_interjection(
    param: &str,
    plenar_speech: &PlenarSpeech,
    db_delegate: &Vec<DbDelegate>,
    plenar_date: &DateTime<Utc>,
) -> Option<Interjection> {
    let (name, text) = extract_interjection_parts(param)?;
    let delegate_results = match_delegate_name_with_delegate_id(
        db_delegate.iter(),
        plenar_date.date_naive(),
        &name,
        true,
    );

    let best = delegate_results.get(0)?;

    let delegate_matching = DelegateMatching {
        similiarity_score: best.similiarity_score as i64,
        searched_with: best.searched_with.clone(),
        matched_with: best.matched_with.clone(),
        delegate_id: best.delegate_id as i32,
        manually_matched: best.manually_matched,
    };

    Some(Interjection {
        interjection_text: text,
        interjector_delegate_id: best.delegate_id as i32,
        plenar_speech: plenar_speech.clone(),
        delegate_matching,
    })
}

fn extract_interjection_parts(param: &str) -> Option<(String, String)> {
    let re = Regex::new(r"Abg\.\s*([A-Za-zÄÖÜäöüß\-\s]+?):\s*(.+)").ok()?;

    let caps = re.captures(param)?;

    let name = caps.get(1)?.as_str().trim().to_string();
    let text = caps.get(2)?.as_str().trim().to_string();

    Some((name, text))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_interjection_extraction_1() {
        assert_extraction(
            "Abg. Hallo: Das ist eine Frechheit!",
            Some(("Hallo", "Das ist eine Frechheit!")),
        );
    }
    #[test]
    fn test_interjection_extraction_2() {
        assert_extraction(
            "Abg. Schwarz: Stimmt ja nicht! Erst heute hat er wieder ...!",
            Some(("Schwarz", "Stimmt ja nicht! Erst heute hat er wieder ...!")),
        );
    }

    #[test]
    fn test_interjection_extraction_3() {
        assert_extraction("Abg. Stefan: Geh!", Some(("Stefan", "Geh!")));
    }

    #[test]
    fn test_interjection_extraction_4() {
        assert_extraction(
            "Abg. Keck: Sie sind auch kein Bürgermeister!",
            Some(("Keck", "Sie sind auch kein Bürgermeister!")),
        );
    }

    #[test]
    fn test_interjection_extraction_5() {
        assert_extraction(
        "Abg. Meinl-Reisinger: Die Wirtshausprämie könnte man auch ...! Die braucht man auch nicht!",
        Some(("Meinl-Reisinger", "Die Wirtshausprämie könnte man auch ...! Die braucht man auch nicht!")),
    );
    }

    #[test]
    fn test_interjection_extraction_6() {
        assert_extraction(
            "Beifall bei der ÖVP. – Abg. Plessl: Was sagen
Sie zum Vertrag 2002? Eine Nebelgranate! Das Problem Vertragsgestaltung 2002
sprechen Sie gar nicht an! – Abg. Steinacker – auf dem Weg zu ihrem Sitzplatz –: Aus
gut mach schlecht!",
            Some((
                "Meinl-Reisinger",
                "Die Wirtshausprämie könnte man auch ...! Die braucht man auch nicht!",
            )),
        );
    }

    #[test]
    fn test_interjection_extraction_negative_1() {
        assert_extraction("Beifall bei Abgeordneten der SPÖ.", None);
    }

    #[test]
    fn test_interjection_extraction_negative_2() {
        assert_extraction("SPÖ", None);
    }

    #[test]
    fn test_interjection_extraction_negative_3() {
        assert_extraction(
            "Beifall und Bravoruf bei der SPÖ sowie Beifall bei den NEOS.",
            None,
        );
    }

    fn assert_extraction(input: &str, expected: Option<(&str, &str)>) {
        let result = extract_interjection_parts(input);

        match (result, expected) {
            (Some((name, text)), Some((exp_name, exp_text))) => {
                assert_eq!(name, exp_name);
                assert_eq!(text, exp_text);
            }
            (None, None) => {}
            (other, expected) => {
                panic!("Mismatch:\nGot: {:?}\nExpected: {:?}", other, expected);
            }
        }
    }
}
