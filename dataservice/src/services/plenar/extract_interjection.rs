
use regex::Regex;

use sqlx::{Executor, Postgres};

pub async fn insert_interjections<'a>(
    pool: &mut sqlx::Transaction<'a, Postgres>,
    plenar_speech_id: i32,
    delegate_id: i32,
    interjection_texts: Vec<String>,
) -> Result<(), sqlx::Error> {
    for text in interjection_texts {
        // trim spaces
        let text = text.trim();
        if text.is_empty() {
            continue;
        }

        // insert into interjections table
        if let Err(e) = sqlx::query!(
            r#"
            INSERT INTO interjections (interjection_text, interjector_delegate_id, plenar_speech_id)
            VALUES ($1, $2, $3)
            ON CONFLICT (interjection_text, interjector_delegate_id, plenar_speech_id)
            DO NOTHING
            "#,
            text,
            delegate_id,
            plenar_speech_id
        )
        .execute(&mut **pool)
        .await
        {
            log::error!(
                "Could not insert interjection for speech {} and delegate {}: {:?}",
                plenar_speech_id,
                delegate_id,
                e
            );
        }
    }

    Ok(())
}





pub fn extract_interjection(param: &str) {
   let re = Regex::new(r"Abg\.\s*([A-Za-zÄÖÜäöüß\-\s]+?):\s*(.+)").unwrap();

    for part in param.split('-') {
        let trimmed = part.trim();

        if let Some(caps) = re.captures(trimmed) {
            let name = caps.get(1).unwrap().as_str();
            let text = caps.get(2).unwrap().as_str();

            println!("{}", param);
            println!("Name: {}", name);
            println!("Text: {}", text);
            println!("---");
            
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_interjection_extraction_1() {
        extract_interjection("Abg. Hallo: Das ist eine Frechheit!");
    }

    #[tokio::test]
    async fn test_interjection_extraction_2() {
        extract_interjection("Beifall bei der ÖVP. - Abg. Plessl: Was sagen Sie zum Vertrag 2002? Eine Nebelgranate! Das Problem Vertragsgestaltung 2002 sprechen Sie gar nicht an! - Abg. Steinacker - auf dem Weg zu ihrem Sitzplatz -: Aus gut mach schlecht!");
    }

    #[tokio::test]
    async fn test_interjection_extraction_3() {
        extract_interjection("Abg. Schwarz: Stimmt ja nicht! Erst heute hat er wieder ...! - Abg. Wöginger: Wo hat er das gesagt?");
    }

    #[tokio::test]
    async fn test_interjection_extraction_4() {
        extract_interjection("Abg. Stefan: Geh!");
    }

    #[tokio::test]
    async fn test_interjection_extraction_5() {
        extract_interjection("Abg. Keck: Sie sind auch kein Bürgermeister!");
    }

    #[tokio::test]
    async fn test_interjection_extraction_6() {
        extract_interjection("Abg. Stocker: ... kein Rechtsanspruch! - Weitere Zwischenrufe bei der ÖVP.");
    }

    #[tokio::test]
    async fn test_interjection_extraction_7() {
        extract_interjection("Abg. Meinl-Reisinger: Die Wirtshausprämie könnte man auch ...! Die braucht man auch nicht!");
    }

    // negative cases (sollten nichts extrahieren)

    #[tokio::test]
    async fn test_interjection_extraction_negative_1() {
        extract_interjection("Beifall bei Abgeordneten der SPÖ.");
    }

    #[tokio::test]
    async fn test_interjection_extraction_negative_2() {
        extract_interjection("Beifall bei der SPÖ.");
    }

    #[tokio::test]
    async fn test_interjection_extraction_negative_3() {
        extract_interjection("SPÖ");
    }

    #[tokio::test]
    async fn test_interjection_extraction_negative_4() {
        extract_interjection("Beifall und Bravoruf bei der SPÖ sowie Beifall bei den NEOS.");
    }
}