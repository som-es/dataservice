use crate::{
    connect_pg, legislative_initiatives::absence_matching, services::scrape_untill_success,
    Shutdown,
};
use scraper::{
    call_to_order::CallToOrderInfo,
    plenar::{Debate, Plenar, PlenarInfo},
    AustriaScraper, PlenarSpeech, Scraper, TextExtractor,
};

async fn scrape_new_plenar_infos(scraper: &AustriaScraper) -> crate::Result<Vec<PlenarInfo>> {
    let raw_plenar = scraper.post_plenar_payload().await?;
    let plenar_paths = scraper.plenar_paths(raw_plenar)?;

    let mut new_plenar_infos = Vec::new();

    for plenar_path in &plenar_paths {
        let Ok(plenar_info) = scraper.get_plenar(&plenar_path).await else {
            log::debug!("Cannot process plenar info: '{plenar_path}'");
            continue;
        };

        new_plenar_infos.push(plenar_info);
    }

    Ok(new_plenar_infos)
}

async fn plenar_info_once(pg_pool: &PgPool) -> Vec<PlenarInfo> {
    log::info!("Try scraping plenar info");

    let mut scraper = AustriaScraper::default();

    let mut new_plenar_infos = vec![];

    for gp in ["XXVII", "XXVI", "XXV", "XXIV", "XXIII", "XXII", "XXI", "XX"] {
        log::info!("PLENAR info once GP: {gp}");
        scraper.scrape_info.gp = vec![gp.into()];

        new_plenar_infos.extend(
            scrape_untill_success::<Vec<PlenarInfo>, _>(|| async {
                scrape_new_plenar_infos(&scraper).await
            })
            .await,
        );
    }

    update_plenar_infos_in_db(&new_plenar_infos, pg_pool).await;
    new_plenar_infos
}

async fn update_plenar_infos_in_db(new_plenar_infos: &[PlenarInfo], pg_pool: &PgPool) {
    log::info!("Updating plenar infos in database...");

    // disable with setting
    let mut text_extractor = scraper::TextExtractor::new(AustriaScraper::BASE_URL);

    for plenar_info in new_plenar_infos {
        // let plenar_id = format!("{}/{}/{}", plenar_info.legislative_period, plenar_info.ityp, plenar_info.inr);

        let plenar_id = match insert_plenar_info(pg_pool, plenar_info).await {
            Ok(id) => id,
            Err(e) => {
                log::error!(
                    "Error inserting plenar info: '{e}'. Error occured for plenar info: {plenar_info:?}. Skipping adding call to orders and debates."
                );
                continue;
            }
        };

        for call_to_order_entry in &plenar_info.call_to_orders {
            if let Err(e) = insert_call_to_order(pg_pool, call_to_order_entry, plenar_id).await {
                log::warn!("Error inserting call to order: '{e}'. Error occured for call to order: {call_to_order_entry:?}");
            }
        }

        for debate in &plenar_info.debates {
            // get id from here, log here

            let debate_id = match insert_debate(pg_pool, debate, plenar_id).await {
                Ok(debate_id) => debate_id,
                Err(e) => {
                    log::warn!(
                        "Error inserting debate: '{e}'. Error occured for debate: {debate:?}"
                    );
                    continue;
                }
            };

            for plenar_speech in &debate.speeches {
                if let Err(e) =
                    insert_plenar_speech(pg_pool, plenar_speech, debate_id, &mut text_extractor)
                        .await
                {
                    log::warn!(
                        "Error inserting plenar speech: '{e}'. Error occured for plenar speech: {plenar_speech:?}"
                    );
                }
            }
        }

        if let Some(absences) = &plenar_info.absences {
            let matching = absence_matching(
                pg_pool,
                absences,
                plenar_info.date.date(),
                plenar_info.gp.clone(),
            )
            .await;
            for match_res in matching
                .correctly_matched
                .iter()
                .chain(matching.lev_or_deep_split_matched.iter())
            {
                if let Err(e) = sqlx::query!(
                    "
                    INSERT INTO absences (lev, similiarity_score, searched_with, matched_with, delegate_id, manually_matched, plenary_session_id)
                    VALUES ($1, $2, $3, $4, $5, $6, $7)
                    ON CONFLICT (delegate_id, plenary_session_id)
                    DO UPDATE SET
                        lev = EXCLUDED.lev,
                        similiarity_score = EXCLUDED.similiarity_score,
                        searched_with = EXCLUDED.searched_with,
                        matched_with = EXCLUDED.matched_with,
                        manually_matched = EXCLUDED.manually_matched
                    ", match_res.lev as i64, match_res.similiarity_score as i64, match_res.searched_with, match_res.matched_with, match_res.delegate_id as i32, match_res.manually_matched, plenar_id
                ).execute(pg_pool).await {
                    log::warn!("Error inserting absene {e:?} for absence {match_res:?}");
                }
            }
        }
    }
}

use log::{info, warn};
use sqlx::PgPool;

static mut NO_OPINION_COUNT: usize = 0;

async fn insert_plenar_speech(
    pool: &PgPool,
    plenar_speech: &PlenarSpeech,
    val_debate_id: i32,
    text_extractor: &mut TextExtractor,
) -> Result<(), sqlx::Error> {
    let disable_speech_text =
        std::env::var("DISABLE_SPEECH_TEXT").unwrap_or_else(|_| "true".into()) == "true";

    let opinion = match &plenar_speech.opinion {
        scraper::Opinion::Infavor => Some("Pro".to_string()),
        scraper::Opinion::Against => Some("Contra".to_string()),
        scraper::Opinion::Other(text) => {
            unsafe { NO_OPINION_COUNT += 1 };
            info!(
                "No opinion for speech: {:?}. 'Opinion': {text}, no opinion count: {}",
                plenar_speech.spoken_text_html_urls,
                unsafe { NO_OPINION_COUNT }
            );
            Some(text.clone())
        }
        scraper::Opinion::None => {
            unsafe { NO_OPINION_COUNT += 1 };
            info!(
                "No opinion for speech: {:?}, no opinion_count: {}",
                plenar_speech.spoken_text_html_urls,
                unsafe { NO_OPINION_COUNT }
            );
            None
        }
    };

    let plenar_speech_id = sqlx::query!(
        r#"
        INSERT INTO plenar_speeches (delegate_id, debate_id, opinion, spoken_text, duration_in_seconds, ty)
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (debate_id, delegate_id)
        DO UPDATE SET
            opinion = EXCLUDED.opinion,
            spoken_text = EXCLUDED.spoken_text,
            duration_in_seconds = EXCLUDED.duration_in_seconds,
            ty = EXCLUDED.ty
        RETURNING id
        "#,
        plenar_speech.delegate_id as i32,
        val_debate_id,
        opinion,
        None::<String>, // spoken_text is initially None
        plenar_speech.duration_in_seconds as i32,
        plenar_speech.ty
    )
    .fetch_one(pool)
    .await?
    .id;

    for url in &plenar_speech.spoken_text_html_urls {
        sqlx::query!(
            r#"
            INSERT INTO plenar_speech_links (plenar_speech_id, document_url)
            VALUES ($1, $2)
            ON CONFLICT (plenar_speech_id, document_url) DO NOTHING
            "#,
            plenar_speech_id,
            url.document_url.to_string()
        )
        .execute(pool)
        .await?;
    }

    let existing_spoken_text = sqlx::query!(
        r#"
        SELECT spoken_text FROM plenar_speeches
        WHERE debate_id = $1 AND delegate_id = $2
        "#,
        val_debate_id,
        plenar_speech.delegate_id as i32
    )
    .fetch_optional(pool)
    .await?;

    if let Some(record) = existing_spoken_text {
        if record.spoken_text.is_none() {
            if let Some(text_html_url) = plenar_speech.spoken_text_html_urls.first() {
                let updated_spoken_text = if disable_speech_text {
                    None
                } else {
                    Some(
                        text_extractor
                            .extract_text_from_url(&text_html_url.document_url, None, true)
                            .await,
                    )
                }
                .flatten();

                sqlx::query!(
                    r#"
                    UPDATE plenar_speeches
                    SET spoken_text = $1
                    WHERE debate_id = $2 AND delegate_id = $3
                    "#,
                    updated_spoken_text,
                    val_debate_id,
                    plenar_speech.delegate_id as i32
                )
                .execute(pool)
                .await?;
            } else {
                warn!(
                    "DEBATE_ID: {}, No spoken text html urls for plenar speech: {:?}",
                    val_debate_id, plenar_speech
                );
            }
        }
    }

    Ok(())
}

async fn insert_debate(pool: &PgPool, debate: &Debate, plenar_id: i32) -> Result<i32, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        INSERT INTO debates (plenar_id, about)
        VALUES ($1, $2)
        ON CONFLICT (plenar_id, about)
        DO UPDATE SET about = EXCLUDED.about
        RETURNING id
        "#,
        plenar_id,
        debate.about
    )
    .fetch_one(pool)
    .await?;

    Ok(result.id)
}

pub async fn insert_plenar_info(
    pool: &PgPool,
    plenar_info: &PlenarInfo,
) -> Result<i32, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        INSERT INTO plenar_infos (inr, title, description, add_date, legislative_period)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (inr, add_date, legislative_period)
        DO UPDATE SET
            title = EXCLUDED.title,
            description = EXCLUDED.description
        RETURNING id
        "#,
        plenar_info.inr as i32,
        plenar_info.title,
        plenar_info.description,
        plenar_info.date,
        plenar_info.gp
    )
    .fetch_one(pool)
    .await?;

    Ok(result.id)
}

pub async fn insert_call_to_order(
    pool: &PgPool,
    call_to_order: &CallToOrderInfo,
    plenar_id: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO call_to_order (id, plenar_id, inr, receiver_id)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (id) DO NOTHING
        "#,
        format!("{}/{}", call_to_order.ityp, call_to_order.inr),
        plenar_id,
        call_to_order.inr as i32,
        call_to_order.receiver_id as i32
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn run_shut(mut shutdown: Shutdown) {
    let plenar_sleep = std::env::var("PLENAR_SLEEP")
        .unwrap_or_else(|_| "400".to_string())
        .parse()
        .unwrap_or_else(|e| {
            log::warn!(
                "Could not parse PLENAR_SLEEP env var: {e}. Using default value of 400 seconds"
            );
            400
        });

    while !shutdown.is_shutdown() {
        tokio::select! {
            _ = shutdown.recv() => {
                println!("shutting down");
                // return ok?
                break;
            }
            _ = async {
                if let Err(e) = tokio::task::spawn(async move {
                    log::info!("Starting plenar info scraper");
                    let pg_pool = connect_pg().await;
                    plenar_info_once(&pg_pool).await;
                }).await {
                    log::error!("Error in plenar info scraper: {:?}", e);
                }
                // legis_init_once().await;
                log::info!("Plenar info scraper service sleeps for {plenar_sleep} seconds...");
                tokio::time::sleep(tokio::time::Duration::from_secs(plenar_sleep)).await;
            } => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDateTime;

    #[test]
    fn test_date_time_parse() {
        let x = NaiveDateTime::parse_from_str("2014-5-17T12:34:56", "%Y-%m-%dT%H:%M:%S").unwrap();
        println!("x: {x:?}");
    }
}
