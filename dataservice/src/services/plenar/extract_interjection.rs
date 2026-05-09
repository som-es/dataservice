use std::sync::Arc;

use chrono::{DateTime, Utc};
use combx::{
    match_delegate_name_with_delegate_id, with_data::active_delegates_at_date, DbDelegate,
    DelegateMatch, Interjection,
};
use somes_scraper::text_utils::{remove_paran_contents, ParanContent};
use sqlx::{Connection, Executor, Postgres};

const DASH: char = '–';
const DASH_STR: &str = "–";

pub async fn extract_interjections<'a>(
    paran_contents: Vec<ParanContent>,
    plenar_speech_id: i64,
    speaker_delegate_id: i64,
    plenar_date: DateTime<Utc>,
    con: impl Executor<'a, Database = Postgres>,
) -> Result<Vec<Interjection>, sqlx::Error> {
    let db_delegates: Arc<[DbDelegate]> =
        Arc::from(active_delegates_at_date(con, plenar_date.date_naive()).await?);
    let mut interjections = Vec::with_capacity(paran_contents.len());
    for paran_content in paran_contents {
        let db_delegates = db_delegates.clone();
        let Ok(interjection) = tokio::task::spawn_blocking(move || {
            extract_interjection(
                &paran_content,
                plenar_speech_id,
                speaker_delegate_id,
                &db_delegates,
                &plenar_date,
            )
        })
        .await
        else {
            continue;
        };
        interjections.extend(interjection);
    }
    Ok(interjections)
}

pub async fn extract_and_save_interjections<'a>(
    paran_contents: Vec<ParanContent>,
    plenar_speech_id: i64,
    speaker_delegate_id: i64,
    plenar_date: DateTime<Utc>,
    pool: &mut sqlx::Transaction<'a, Postgres>,
) -> Result<(), sqlx::Error> {
    let interjections = extract_interjections(
        paran_contents,
        plenar_speech_id,
        speaker_delegate_id,
        plenar_date,
        &mut **pool,
    )
    .await?;

    save_interjection(interjections, pool, plenar_speech_id).await
}

pub async fn save_interjection<'a>(
    interjections: Vec<Interjection>,
    tx: &mut sqlx::Transaction<'a, Postgres>,
    plenar_speech_id: i64,
) -> Result<(), sqlx::Error> {
    let mut tx = tx.begin().await?;

    let delegate_matching_ids = sqlx::query_scalar!(
        "DELETE FROM interjections WHERE plenar_speech_id = $1 RETURNING delegate_matching_id",
        plenar_speech_id as i32
    )
    .fetch_all(&mut *tx)
    .await?;

    for to_delete_delegate_matching_id in delegate_matching_ids {
        sqlx::query!(
            "delete from delegate_matching where id = $1",
            to_delete_delegate_matching_id
        )
        .execute(&mut *tx)
        .await?;
    }

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
            interjection.delegate_match.similiarity_score,
            interjection.delegate_match.searched_with,
            interjection.delegate_match.matched_with,
            interjection.delegate_match.delegate_id,
            interjection.delegate_match.manually_matched,
        )
        .fetch_one(&mut *tx)
        .await?
        .id;

        sqlx::query!(
            r#"
            INSERT INTO interjections (
                interjection_text,
                interjector_delegate_id,
                plenar_speech_id,
                delegate_matching_id,
                rel_start_idx,
                rel_end_idx
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            interjection.interjection_text,
            interjection.interjector_delegate_id,
            plenar_speech_id as i32,
            delegate_matching_id,
            interjection.rel_start_idx as i32,
            interjection.rel_end_idx as i32,
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    Ok(())
}

fn extract_interjection(
    paran: &ParanContent,
    plenar_speech_id: i64,
    speaker_delegate_id: i64,
    db_delegate: &[DbDelegate],
    plenar_date: &DateTime<Utc>,
) -> Vec<Interjection> {
    let parts = extract_interjections_parts(&paran.content);

    let mut interjections = Vec::new();

    for (name, text) in parts {
        if name.is_empty() || text.is_empty() {
            continue;
        }
        let delegate_results = match_delegate_name_with_delegate_id(
            db_delegate.iter(),
            plenar_date.date_naive(),
            &name,
            true,
        );

        if let Some(best) = delegate_results.get(0) {
            let delegate_match = DelegateMatch {
                similiarity_score: best.similiarity_score as i64,
                searched_with: best.searched_with.clone(),
                matched_with: best.matched_with.clone(),
                delegate_id: best.delegate_id as i32,
                manually_matched: best.manually_matched,
            };

            interjections.push(Interjection {
                interjection_text: Some(text),
                interjector_delegate_id: best.delegate_id as i32,
                plenar_speech_id: plenar_speech_id as i32,
                delegate_match,
                date: *plenar_date,
                rel_start_idx: paran.start_idx as i32,
                rel_end_idx: paran.end_idx as i32,
                speaker_delegate_id: speaker_delegate_id as i32,
            });
        }
    }

    interjections
}

fn extract_interjections_parts(param: &str) -> Vec<(String, String)> {
    let mut text = param.to_string();
    let mut interjections = vec![];
    while let Some(name_text_split) = text.find(':') {
        let name = &text[..name_text_split];

        let text_part_start = text[name_text_split..]
            .char_indices()
            .nth(1)
            .map(|(i, _)| name_text_split + i)
            .unwrap_or(text.len());

        let text_part = &text[text_part_start..];

        let name = name.trim();
        let mut name = if name.contains("Abg.") {
            name.split_once("Abg.").unwrap_or(("", "")).1.to_string()
        } else {
            "".to_string()
        };

        // Spezialfall: "Name – irgendwas –"
        if name.matches(DASH).count() >= 2 {
            name = name.split(DASH).next().unwrap_or("").trim().to_string();
        }
        let mut end_idx = text_part.len();
        let mut pre_skipped = false;
        let mut prev_non_whitespace_char = None;
        for (idx, char) in text_part.char_indices() {
            if char == DASH {
                if let Some(check_char) = text_part[idx..].chars().nth(2) {
                    if check_char.is_uppercase()
                        || (prev_non_whitespace_char.map(|c: char| c.is_ascii_punctuation()))
                            .unwrap_or_default()
                    {
                        end_idx = idx;
                        pre_skipped = true;
                        break;
                    }
                }
            }
            if !char.is_whitespace() {
                prev_non_whitespace_char = Some(char);
            }
        }

        let res_text = text_part[..end_idx].trim().to_string();
        let skip = if pre_skipped { 3 } else { 0 };
        let start = text[text_part_start + end_idx..]
            .char_indices()
            .nth(skip)
            .map(|(i, _)| text_part_start + end_idx + i)
            .unwrap_or(text.len());

        text = text[start..].to_string();

        (_, name) = remove_paran_contents(&name, '[', ']');

        interjections.push((name.trim().to_string(), res_text));
    }
    interjections
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use crate::connect_pg2;

    use super::*;

    fn assert_extraction(input: &str, expected: Vec<(&str, &str)>) {
        let result = extract_interjections_parts(input);

        let expected: Vec<(String, String)> = expected
            .into_iter()
            .map(|(n, t)| (n.to_string(), t.to_string()))
            .collect();

        assert_eq!(result, expected);
    }

    // (Abg. Martin Graf: Das war ein Schallmeiner-Schurkenstück! – Abg. Schallmeiner: ... kennt sich der Herr Graf aus, mit den Schurkenstücken! – Unruhe im Saal. – Der Präsident gibt das Glockenzeichen.)

    #[test]
    fn test_interjection_extraction_x() {
        assert_extraction(
            "Abg. Martin Graf: Das war ein Schallmeiner-Schurkenstück! – Abg. Schallmeiner: ... kennt sich der Herr Graf aus, mit den Schurkenstücken! – Unruhe im Saal. – Der Präsident gibt das Glockenzeichen. – Abg. Hirsch: Super.. – Rufe ber der ÖVP: AJA!",
            vec![
                ("Martin Graf", "Das war ein Schallmeiner-Schurkenstück!"),
                ("Schallmeiner", "... kennt sich der Herr Graf aus, mit den Schurkenstücken!"),
                    ("Hirsch", "Super.."),
                    ("", "AJA!")
            ],
        );
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
    fn test_interjection_extraction_qouted_1() {
        //
        assert_extraction(
            "Abg. Bures: „Wie es sich gehört!“",
            vec![("Bures", "„Wie es sich gehört!“")],
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
    fn test_interjection_extraction_7() {
        assert_extraction(
            "Abg. Dr. Schüssel:
                    Benita – und nicht Benito!",
            vec![("Dr. Schüssel", "Benita – und nicht Benito!")],
        );
    }
    #[test]
    fn test_interjection_extraction_8() {
        assert_extraction(
            "Abg. Gewessler [Grüne]: Mit Ihrer Politik sicher nicht!",
            vec![("Gewessler", "Mit Ihrer Politik sicher nicht!")],
        );
    }

    #[test]
    fn test_interjection_extraction_9() {
        // Rufe bei der ÖVP: Oh!
        assert_extraction(
            "Rufe bei der ÖVP: Oh! – Abg. Ha: Uh!",
            vec![("", "Oh!"), ("Ha", "Uh!")],
        );
    }

    // Sie nicht! – weitere Zwischenrufe bei der ÖVP
    #[test]
    fn test_interjection_extraction_10() {
        // Rufe bei der ÖVP: Oh!
        assert_extraction(
            "Abg. Sieber: Sie nicht! – weitere Zwischenrufe bei der ÖVP",
            vec![("Sieber", "Sie nicht!")],
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

    #[tokio::test]
    pub async fn test_interjection_delegate_id_matching_multiple() {
        let paran_content =
            "Beifall bei der ÖVP. – Abg. Dr. Plessl: Was sagen Sie zum Vertrag 2002? Eine Nebelgranate! Das Problem Vertragsgestaltung 2002 sprechen Sie gar nicht an! – Abg. Steinacker – auf dem Weg zu ihrem Sitzplatz –: Aus gut mach schlecht!";
        let pg = connect_pg2().await;
        let plenar_date = DateTime::<Utc>::from_naive_utc_and_offset(
            NaiveDate::from_ymd_opt(2019, 9, 25)
                .unwrap()
                .and_hms_nano_opt(0, 0, 0, 0)
                .unwrap(),
            Utc,
        );

        let interjections = extract_interjections(
            vec![ParanContent {
                start_idx: 0,
                end_idx: 0,
                content: paran_content.into(),
            }],
            0,
            0,
            plenar_date,
            &pg,
        )
        .await
        .unwrap();

        let match1 = DelegateMatch {
            similiarity_score: 4,
            searched_with: "Dr. Plessl".into(),
            matched_with: "Plessl Rudolf".into(),
            delegate_id: 51549,
            manually_matched: false,
        };

        let match2 = DelegateMatch {
            similiarity_score: 0,
            searched_with: "Steinacker".into(),
            matched_with: "Steinacker Michaela, Mag.".into(),
            delegate_id: 83059,
            manually_matched: false,
        };
        assert_eq!(interjections[0].delegate_match.clone(), match1);
        assert_eq!(interjections[1].delegate_match.clone(), match2);
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
