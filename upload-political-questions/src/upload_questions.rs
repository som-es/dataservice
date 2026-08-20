mod generated_questions;
pub use generated_questions::*;

mod user_questions;
pub use user_questions::*;

use sqlx::PgPool;

pub struct QuestionData {
    pub question: &'static str,
    pub is_left: Option<bool>,
    pub is_liberal: Option<bool>,
    pub topics: &'static [&'static str],
    pub detailed_topics: &'static [&'static str],
    pub topics_influenced: &'static [(&'static str, f64)],
    pub detailed_topics_influenced: &'static [(&'static str, f64)],
    pub is_part_of: Option<&'static [&'static str]>,
}

pub const QUESTIONS: &[QuestionData] = &[
    // --- LEFT / RIGHT QUESTIONS ---
    QuestionData {
        question: "Der Staat sollte weniger ins Wirtschaftsleben eingreifen.",
        is_left: Some(false),
        is_liberal: None,
        topics: &["Wirtschaft", "Budget und Finanzen"],
        detailed_topics: &[
            "Unternehmen und Wettbewerb",
            "Öffentliche Finanzen und Haushaltspolitik",
        ],
        topics_influenced: &[
            ("Wirtschaft", 0.9),
            ("Soziales", -0.7),
            ("Budget und Finanzen", 0.6),
            ("Arbeit", -0.5),
        ],
        detailed_topics_influenced: &[
            ("Unternehmen und Wettbewerb", 0.9),
            ("Öffentliche Finanzen und Haushaltspolitik", 0.6),
            ("Arbeitsrecht", -0.5),
            ("Soziale Sicherheit", -0.6),
        ],
        is_part_of: None,
    },
    QuestionData {
        question: "Der Sozialstaat sollte ausgebaut werden, auch wenn das höhere Steuern und Abgaben bedeutet.",
        is_left: Some(true),
        is_liberal: None,
        topics: &["Soziales", "Budget und Finanzen"],
        detailed_topics: &["Sozialpolitik", "Steuerwesen"],
        topics_influenced: &[
            ("Soziales", 0.9),
            ("Budget und Finanzen", -0.8),
            ("Wirtschaft", -0.6),
            ("Arbeit", 0.5),
        ],
        detailed_topics_influenced: &[
            ("Sozialpolitik", 0.9),
            ("Steuerwesen", -0.8),
            ("Arbeitslosenversicherung", 0.7),
            ("Soziale Sicherheit", 0.8),
        ],
        is_part_of: None,
    },
    QuestionData {
        question: "Die Politik sollte stärker in die Preise eingreifen, um die Teuerung abzumildern.",
        is_left: Some(true),
        is_liberal: None,
        topics: &["Wirtschaft", "Budget und Finanzen", "Soziales"],
        detailed_topics: &["Preis", "Öffentliche Finanzen und Haushaltspolitik"],
        topics_influenced: &[
            ("Soziales", 0.8),
            ("Wirtschaft", -0.7),
            ("Budget und Finanzen", -0.5),
            ("Arbeit", 0.4),
        ],
        detailed_topics_influenced: &[
            ("Preis", -0.8),
            ("Öffentliche Finanzen und Haushaltspolitik", -0.5),
            ("Sozialpolitik", 0.7),
            ("Beschäftigung und Arbeitsbedingungen", 0.4),
        ],
        is_part_of: None,
    },
    QuestionData {
        question: "Es muss mehr getan werden, um den Klimawandel zu stoppen, auch wenn das für den einzelnen teurer wird.",
        is_left: Some(true),
        is_liberal: None,
        topics: &["Klima, Umwelt und Energie", "Wirtschaft"],
        detailed_topics: &["Umwelt", "Energie"],
        topics_influenced: &[
            ("Klima, Umwelt und Energie", 0.9),
            ("Wirtschaft", -0.6),
            ("Soziales", -0.4),
            ("Verkehr und Infrastruktur", -0.3),
        ],
        detailed_topics_influenced: &[
            ("Umwelt", 0.9),
            ("Energie", 0.7),
            ("Straßenverkehr", -0.4),
            ("Industrie", -0.5),
        ],
        is_part_of: None,
    },
    QuestionData {
        question: "Die Infrastruktur für Autos sollte ausgebaut werden.",
        is_left: Some(false),
        is_liberal: None,
        topics: &["Verkehr und Infrastruktur", "Klima, Umwelt und Energie"],
        detailed_topics: &["Straßenverkehr", "Bauindustrie und öffentliches Bauwesen"],
        topics_influenced: &[
            ("Verkehr und Infrastruktur", 0.8),
            ("Klima, Umwelt und Energie", -0.7),
            ("Wirtschaft", 0.5),
            ("Soziales", -0.2),
        ],
        detailed_topics_influenced: &[
            ("Straßenverkehr", 0.8),
            ("Bauindustrie und öffentliches Bauwesen", 0.7),
            ("Umwelt", -0.7),
            ("Raumordnung", -0.4),
        ],
        is_part_of: None,
    },
    QuestionData {
        question: "Naturschutz darf nicht auf Kosten des Wirtschaftswachstums gehen.",
        is_left: Some(false),
        is_liberal: None,
        topics: &["Klima, Umwelt und Energie", "Wirtschaft"],
        detailed_topics: &["Umwelt", "Raumordnung"],
        topics_influenced: &[
            ("Wirtschaft", 0.8),
            ("Klima, Umwelt und Energie", -0.9),
            ("Arbeit", 0.4),
            ("Soziales", -0.3),
        ],
        detailed_topics_influenced: &[
            ("Umwelt", -0.9),
            ("Raumordnung", -0.6),
            ("Industrie", 0.7),
            ("Energie", -0.5),
        ],
        is_part_of: None,
    },
    QuestionData {
        question: "Eine gemeinsame Schule für alle bis 14 sollte eingeführt werden.",
        is_left: Some(true),
        is_liberal: None,
        topics: &["Bildung"],
        detailed_topics: &["Organisation des Unterrichtswesens", "Sekundarstufe"],
        topics_influenced: &[
            ("Bildung", 0.8),
            ("Soziales", 0.7),
            ("Familie und Generationen", 0.4),
            ("Wirtschaft", 0.0),
        ],
        detailed_topics_influenced: &[
            ("Organisation des Unterrichtswesens", 0.7),
            ("Sekundarstufe", 0.8),
            ("Sozialpolitik", 0.5),
            ("Familie", 0.4),
        ],
        is_part_of: None,
    },
    QuestionData {
        question: "Das Arbeitslosengeld sollte sinken, je länger man es bezieht.",
        is_left: Some(false),
        is_liberal: None,
        topics: &["Arbeit", "Soziales"],
        detailed_topics: &["Arbeitslosenversicherung", "Soziale Sicherheit"],
        topics_influenced: &[
            ("Wirtschaft", 0.6),
            ("Soziales", -0.8),
            ("Arbeit", -0.7),
            ("Budget und Finanzen", 0.5),
        ],
        detailed_topics_influenced: &[
            ("Arbeitslosenversicherung", -0.8),
            ("Soziale Sicherheit", -0.7),
            ("Beschäftigung und Arbeitsbedingungen", -0.6),
            ("Steuerwesen", 0.4),
        ],
        is_part_of: None,
    },
    QuestionData {
        question: "In Österreich sollte eine 32-Stunden-Woche eingeführt werden, bei gleichbleibendem Lohn.",
        is_left: Some(true),
        is_liberal: None,
        topics: &["Arbeit", "Wirtschaft"],
        detailed_topics: &["Beschäftigung und Arbeitsbedingungen", "Arbeitsrecht"],
        topics_influenced: &[
            ("Arbeit", 0.9),
            ("Wirtschaft", -0.8),
            ("Soziales", 0.6),
            ("Budget und Finanzen", -0.5),
        ],
        detailed_topics_influenced: &[
            ("Beschäftigung und Arbeitsbedingungen", 0.9),
            ("Arbeitsrecht", 0.8),
            ("Industrie", -0.7),
            ("Unternehmen und Wettbewerb", -0.6),
        ],
        is_part_of: None,
    },
    QuestionData {
        question: "Erbschaften sollten besteuert werden.",
        is_left: Some(true),
        is_liberal: None,
        topics: &["Budget und Finanzen", "Wirtschaft", "Soziales"],
        detailed_topics: &["Steuerwesen", "Öffentliche Finanzen und Haushaltspolitik"],
        topics_influenced: &[
            ("Budget und Finanzen", 0.8),
            ("Soziales", 0.7),
            ("Wirtschaft", -0.6),
            ("Familie und Generationen", -0.3),
        ],
        detailed_topics_influenced: &[
            ("Steuerwesen", 0.8),
            ("Öffentliche Finanzen und Haushaltspolitik", 0.7),
            ("Familie", -0.4),
            ("Bürgerliches Recht", -0.3),
        ],
        is_part_of: None,
    },
    QuestionData {
        question: "Wahlärzt:innen sollten verpflichtet werden, einen Teil ihrer Patient:innen nach dem Kassentarif zu behandeln.",
        is_left: Some(true),
        is_liberal: None,
        topics: &["Gesundheit und Ernährung", "Soziales"],
        detailed_topics: &["Gesundheit", "Sozialpolitik"],
        topics_influenced: &[
            ("Gesundheit und Ernährung", 0.8),
            ("Soziales", 0.7),
            ("Wirtschaft", -0.5),
            ("Budget und Finanzen", -0.3),
        ],
        detailed_topics_influenced: &[
            ("Gesundheit", 0.8),
            ("Sozialpolitik", 0.7),
            ("Krankenpflege", 0.6),
            ("Unternehmen und Wettbewerb", -0.4),
        ],
        is_part_of: None,
    },
    QuestionData {
        question: "Die Möglichkeit, Wohnungen befristet zu vermieten, sollte eingeschränkt werden.",
        is_left: Some(true),
        is_liberal: None,
        topics: &["Soziales", "Wirtschaft"],
        detailed_topics: &["Wohnungspolitik", "Bürgerliches Recht"],
        topics_influenced: &[
            ("Soziales", 0.8),
            ("Wirtschaft", -0.7),
            ("Familie und Generationen", 0.4),
            ("Budget und Finanzen", -0.3),
        ],
        detailed_topics_influenced: &[
            ("Wohnungspolitik", 0.8),
            ("Bürgerliches Recht", -0.6),
            ("Handel", -0.5),
            ("Familie", 0.4),
        ],
        is_part_of: None,
    },
    // --- LIBERAL / AUTHORITARIAN QUESTIONS ---
    QuestionData {
        question: "Wichtige Entscheidungen sollten stärker auf nationaler Ebene getroffen werden und nicht auf EU-Ebene.",
        is_left: None,
        is_liberal: Some(false),
        topics: &["Europäische Union", "Parlament und Demokratie"],
        detailed_topics: &["Verwaltungsreform", "Föderalismus"],
        topics_influenced: &[
            ("Europäische Union", -0.8),
            ("Außenpolitik", -0.6),
            ("Wirtschaft", -0.4),
            ("Parlament und Demokratie", 0.5),
        ],
        detailed_topics_influenced: &[
            ("Verwaltungsreform", 0.6),
            ("Föderalismus", 0.5),
            ("Internationale Beziehungen", -0.7),
            ("Handel", -0.4),
        ],
        is_part_of: None,
    },
    QuestionData {
        question: "Die Politik sollte das traditionelle Familienbild (Vater, Mutter, Kinder) stärken.",
        is_left: None,
        is_liberal: Some(false),
        topics: &["Familie und Generationen"],
        detailed_topics: &["Familie", "Sozialpolitik"],
        topics_influenced: &[
            ("Familie und Generationen", 0.8),
            ("Frauen und Gleichbehandlung", -0.7),
            ("Soziales", 0.5),
            ("Bildung", -0.3),
        ],
        detailed_topics_influenced: &[
            ("Familie", 0.8),
            ("Sozialpolitik", 0.5),
            ("Gleichbehandlung", -0.7),
            ("Organisation des Unterrichtswesens", -0.3),
        ],
        is_part_of: None,
    },
    QuestionData {
        question: "Österreich sollte seine Neutralität aufgeben.",
        is_left: None,
        is_liberal: Some(true),
        topics: &["Außenpolitik", "Landesverteidigung"],
        detailed_topics: &["Neutralität", "Verteidigung"],
        topics_influenced: &[
            ("Außenpolitik", 0.8),
            ("Landesverteidigung", 0.7),
            ("Europäische Union", 0.6),
            ("Inneres und Recht", -0.3),
        ],
        detailed_topics_influenced: &[
            ("Neutralität", -0.9),
            ("Verteidigung", 0.7),
            ("Internationale Beziehungen", 0.8),
            ("Öffentliche Sicherheit", 0.4),
        ],
        is_part_of: None,
    },
    QuestionData {
        question: "Zuwanderer:innen müssen sich an die österreichische Kultur und Gepflogenheiten anpassen.",
        is_left: None,
        is_liberal: Some(false),
        topics: &["Inneres und Recht", "Soziales"],
        detailed_topics: &["Integration", "Staatsangehöriger"],
        topics_influenced: &[
            ("Inneres und Recht", 0.7),
            ("Soziales", -0.6),
            ("Kultur", 0.5),
            ("Frauen und Gleichbehandlung", -0.4),
        ],
        detailed_topics_influenced: &[
            ("Integration", -0.7),
            ("Staatsangehöriger", 0.6),
            ("Menschenrechte", -0.5),
            ("Ethnische Gruppe", -0.5),
        ],
        is_part_of: None,
    },
    QuestionData {
        question: "Straftäter:innen sollten härter bestraft werden.",
        is_left: None,
        is_liberal: Some(false),
        topics: &["Inneres und Recht"],
        detailed_topics: &["Strafrecht", "Gerichtswesen"],
        topics_influenced: &[
            ("Inneres und Recht", 0.8),
            ("Soziales", -0.5),
            ("Parlament und Demokratie", -0.3),
        ],
        detailed_topics_influenced: &[
            ("Strafrecht", 0.8),
            ("Gerichtswesen", 0.6),
            ("Menschenrechte", -0.6),
            ("Opferhilfe", 0.4),
        ],
        is_part_of: None,
    },
    QuestionData {
        question: "Die Strafen für Klimakleber:innen sollten verschärft werden.",
        is_left: None,
        is_liberal: Some(false),
        topics: &["Inneres und Recht", "Klima, Umwelt und Energie"],
        detailed_topics: &["Strafrecht", "Öffentliche Sicherheit"],
        topics_influenced: &[
            ("Inneres und Recht", 0.7),
            ("Klima, Umwelt und Energie", -0.6),
            ("Parlament und Demokratie", -0.4),
        ],
        detailed_topics_influenced: &[
            ("Strafrecht", 0.7),
            ("Öffentliche Sicherheit", 0.6),
            ("Versammlungsfreiheit", -0.7),
            ("Umwelt", -0.5),
        ],
        is_part_of: None,
    },
    QuestionData {
        question: "Österreich sollte den Beitritt zur Nato erwägen.",
        is_left: None,
        is_liberal: Some(true),
        topics: &["Außenpolitik", "Landesverteidigung"],
        detailed_topics: &["Neutralität", "Internationale Beziehungen", "Verteidigung"],
        topics_influenced: &[
            ("Außenpolitik", 0.8),
            ("Landesverteidigung", 0.7),
            ("Europäische Union", 0.5),
            ("Inneres und Recht", -0.3),
        ],
        detailed_topics_influenced: &[
            ("Neutralität", -0.9),
            ("Internationale Beziehungen", 0.8),
            ("Verteidigung", 0.7),
            ("Völkerrecht", 0.5),
        ],
        is_part_of: None,
    },
    QuestionData {
        question: "Österreich sollte sich im Nahost-Konflikt klar auf die Seite Israels stellen.",
        is_left: None,
        is_liberal: Some(true),
        topics: &["Außenpolitik"],
        detailed_topics: &["Internationale Beziehungen", "Völkerrecht"],
        topics_influenced: &[
            ("Außenpolitik", 0.7),
            ("Inneres und Recht", -0.3),
            ("Soziales", -0.2),
        ],
        detailed_topics_influenced: &[
            ("Internationale Beziehungen", 0.7),
            ("Völkerrecht", 0.6),
            ("Diplomatische Beziehungen", 0.5),
            ("Religion", -0.4),
        ],
        is_part_of: None,
    },
    QuestionData {
        question: "Österreich sollte sich für die Erweiterung der EU einsetzen.",
        is_left: None,
        is_liberal: Some(true),
        topics: &["Europäische Union", "Außenpolitik"],
        detailed_topics: &["Europäische Union", "Internationale Beziehungen"],
        topics_influenced: &[
            ("Europäische Union", 0.8),
            ("Außenpolitik", 0.7),
            ("Wirtschaft", 0.5),
            ("Inneres und Recht", -0.3),
        ],
        detailed_topics_influenced: &[
            ("Europäische Union", 0.8),
            ("Internationale Beziehungen", 0.7),
            ("Handel", 0.5),
            ("Verwaltungsreform", -0.4),
        ],
        is_part_of: None,
    },
    QuestionData {
        question: "Die ORF-Haushaltsabgabe sollte (ersatzlos) abgeschafft werden.",
        is_left: None,
        is_liberal: Some(false),
        topics: &["Information und Medien"],
        detailed_topics: &["Fernsehen", "Hörfunk"],
        topics_influenced: &[
            ("Information und Medien", -0.8),
            ("Kultur", -0.6),
            ("Budget und Finanzen", 0.5),
        ],
        detailed_topics_influenced: &[
            ("Fernsehen", -0.8),
            ("Hörfunk", -0.7),
            ("Presse", -0.4),
            ("Öffentliche Finanzen und Haushaltspolitik", 0.5),
        ],
        is_part_of: None,
    },
    QuestionData {
        question: "Schwangerschaftsabbrüche innerhalb der ersten drei Monate sollten von der Krankenkasse bezahlt werden.",
        is_left: None,
        is_liberal: Some(true),
        topics: &["Gesundheit und Ernährung", "Frauen und Gleichbehandlung"],
        detailed_topics: &["Frau", "Gesundheit"],
        topics_influenced: &[
            ("Frauen und Gleichbehandlung", 0.9),
            ("Gesundheit und Ernährung", 0.7),
            ("Soziales", 0.5),
            ("Budget und Finanzen", -0.3),
        ],
        detailed_topics_influenced: &[
            ("Frau", 0.9),
            ("Gesundheit", 0.7),
            ("Sozialpolitik", 0.5),
            ("Krankenpflege", 0.4),
        ],
        is_part_of: None,
    },
    QuestionData {
        question: "Um Gewalt gegen Frauen zu bekämpfen, müssen traditionelle Männlichkeitsbilder überwunden werden.",
        is_left: None,
        is_liberal: Some(true),
        topics: &["Frauen und Gleichbehandlung", "Inneres und Recht"],
        detailed_topics: &["Opferhilfe", "Gleichbehandlung"],
        topics_influenced: &[
            ("Frauen und Gleichbehandlung", 0.9),
            ("Inneres und Recht", 0.6),
            ("Soziales", 0.5),
            ("Bildung", 0.4),
        ],
        detailed_topics_influenced: &[
            ("Opferhilfe", 0.8),
            ("Gleichbehandlung", 0.9),
            ("Strafrecht", 0.5),
            ("Organisation des Unterrichtswesens", 0.4),
        ],
        is_part_of: None,
    },
    QuestionData {
        question: "Cannabis sollte in Österreich legalisiert werden.",
        is_left: None,
        is_liberal: Some(true),
        topics: &["Gesundheit und Ernährung", "Inneres und Recht"],
        detailed_topics: &["Strafrecht", "Gesundheit"],
        topics_influenced: &[
            ("Inneres und Recht", 0.7),
            ("Gesundheit und Ernährung", 0.6),
            ("Soziales", 0.4),
            ("Wirtschaft", 0.3),
        ],
        detailed_topics_influenced: &[
            ("Strafrecht", 0.7),
            ("Gesundheit", 0.6),
            ("Öffentliche Sicherheit", -0.4),
            ("Handel", 0.3),
        ],
        is_part_of: None,
    },
    QuestionData {
        question: "Die Polizei sollte die Möglichkeit bekommen, die Messenger-Dienste (WhatsApp, Signal, ...) von Strafverdächtigen zu überwachen.",
        is_left: None,
        is_liberal: Some(false),
        topics: &["Inneres und Recht", "Information und Medien"],
        detailed_topics: &["Öffentliche Sicherheit", "Telekommunikation"],
        topics_influenced: &[
            ("Inneres und Recht", 0.7),
            ("Information und Medien", -0.8),
            ("Parlament und Demokratie", -0.6),
        ],
        detailed_topics_influenced: &[
            ("Öffentliche Sicherheit", 0.7),
            ("Telekommunikation", -0.8),
            ("Menschenrechte", -0.7),
            ("Internet", -0.6),
        ],
        is_part_of: None,
    },
    QuestionData {
        question: "Die Rechte von Beschuldigten im Strafverfahren müssen gestärkt werden.",
        is_left: None,
        is_liberal: Some(true),
        topics: &["Inneres und Recht"],
        detailed_topics: &["Menschenrechte", "Strafrecht"],
        topics_influenced: &[
            ("Inneres und Recht", 0.7),
            ("Parlament und Demokratie", 0.6),
            ("Soziales", 0.4),
        ],
        detailed_topics_influenced: &[
            ("Menschenrechte", 0.8),
            ("Strafrecht", 0.6),
            ("Gerichtswesen", 0.5),
            ("Opferhilfe", -0.3),
        ],
        is_part_of: None,
    },
    QuestionData {
        question: "Volksbegehren, die eine bestimmte Anzahl an Unterstützungen erreichen, sollten automatisch zu Volksabstimmungen führen.",
        is_left: None,
        is_liberal: Some(true),
        topics: &["Parlament und Demokratie"],
        detailed_topics: &["Direkt gewählte Kammer", "Partizipative Demokratie"],
        topics_influenced: &[
            ("Parlament und Demokratie", 0.8),
            ("Inneres und Recht", -0.4),
            ("Soziales", 0.3),
        ],
        detailed_topics_influenced: &[
            ("Direkt gewählte Kammer", 0.7),
            ("Partizipative Demokratie", 0.8),
            ("Gesetzgebungsverfahren", -0.4),
            ("Verfassung", 0.3),
        ],
        is_part_of: None,
    },
];

pub async fn upload_questions(
    pool: &PgPool,
    questions: &[QuestionData],
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    for q in questions {
        let is_part_of = q
            .is_part_of
            .map(|is_part_of| is_part_of.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        let result = sqlx::query!(
            r#"
            INSERT INTO political_questions (question, is_left, is_liberal, is_part_of)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (question) DO UPDATE SET is_left = EXCLUDED.is_left, is_liberal = EXCLUDED.is_liberal, is_part_of = EXCLUDED.is_part_of
            RETURNING id
            "#,
            q.question,
            q.is_left,
            q.is_liberal,
            is_part_of.as_deref()
        )
        .fetch_optional(&mut *tx)
        .await?;

        if let Some(record) = result {
            let question_id = record.id;
            println!("{:?}", question_id);

            sqlx::query!(
                r#"DELETE FROM political_questions_topics WHERE question_id = $1"#,
                question_id
            )
            .execute(&mut *tx)
            .await?;

            sqlx::query!(
                r#"DELETE FROM political_questions_detailed_topics WHERE question_id = $1"#,
                question_id
            )
            .execute(&mut *tx)
            .await?;

            sqlx::query!(
                r#"DELETE FROM political_questions_topics_influence WHERE question_id = $1"#,
                question_id
            )
            .execute(&mut *tx)
            .await?;

            sqlx::query!(
                r#"DELETE FROM political_questions_detailed_topics_influence WHERE question_id = $1"#,
                question_id
            )
            .execute(&mut *tx)
            .await?;

            for &topic in q.topics {
                println!("{}", topic);
                sqlx::query!(
                    r#"
                    INSERT INTO political_questions_topics (question_id, topic)
                    VALUES ($1, $2)
                    ON CONFLICT (question_id, topic) DO NOTHING
                    "#,
                    question_id,
                    topic
                )
                .execute(&mut *tx)
                .await?;
            }

            for &topic in q.detailed_topics {
                sqlx::query!(
                    r#"
                    INSERT INTO political_questions_detailed_topics (question_id, topic)
                    VALUES ($1, $2)
                    ON CONFLICT (question_id, topic) DO NOTHING
                    "#,
                    question_id,
                    topic
                )
                .execute(&mut *tx)
                .await?;
            }

            for &(topic, influence) in q.topics_influenced {
                sqlx::query!(
                    r#"
                    INSERT INTO political_questions_topics_influence (question_id, topic, influence)
                    VALUES ($1, $2, $3)
                    ON CONFLICT (question_id, topic) DO UPDATE SET influence = EXCLUDED.influence
                    "#,
                    question_id,
                    topic,
                    influence
                )
                .execute(&mut *tx)
                .await?;
            }

            for &(topic, influence) in q.detailed_topics_influenced {
                sqlx::query!(
                    r#"
                    INSERT INTO political_questions_detailed_topics_influence (question_id, topic, influence)
                    VALUES ($1, $2, $3)
                    ON CONFLICT (question_id, topic) DO UPDATE SET influence = EXCLUDED.influence
                    "#,
                    question_id,
                    topic,
                    influence
                )
                .execute(&mut *tx)
                .await?;
            }
        }
    }

    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    #[tokio::test]
    async fn extract_vote_results() {
        let vr = reqwest::get(
            "https://somes.at/api/at/v1/vote_results/search?is_finished=true&search=&page=1&entries_per_page=500&legislative_initiative[is_law][eq]=false&sort=Desc",
        ).await.unwrap().json::<Value>().await.unwrap();
        let vote_results = vr["vote_results"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|vote_result| {
                Some(
                    vote_result
                        .get("ai_summary")?
                        .get("full_summary")?
                        .get("political_compass_questions")?
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|value| value.as_str().unwrap())
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>();

        for vote_result in &vote_results[..100] {
            dbg!(vote_result);
        }
    }
}
