use crate::{
    models::{DbCallToOrder, DbDebate, DbPlenar, DbPlenarSpeech, DbPlenarSpeechLink},
    schema::{call_to_order, debates, plenar_infos, plenar_speech_links, plenar_speeches},
    services::scrape_untill_success,
    Shutdown,
};
use diesel::{ExpressionMethods, PgConnection, QueryDsl, QueryResult, RunQueryDsl};
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

async fn plenar_info_once() -> Vec<PlenarInfo> {
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

    update_plenar_infos_in_db(&new_plenar_infos).await;
    new_plenar_infos
}

async fn update_plenar_infos_in_db(new_plenar_infos: &[PlenarInfo]) {
    log::info!("Updating plenar infos in database...");

    let mut conn = crate::establish_connection();

    // disable with setting
    let mut text_extractor = scraper::TextExtractor::new(AustriaScraper::BASE_URL);

    for plenar_info in new_plenar_infos {
        // let plenar_id = format!("{}/{}/{}", plenar_info.legislative_period, plenar_info.ityp, plenar_info.inr);

        let plenar_id = match insert_plenar_info(plenar_info, &mut conn) {
            Ok(id) => id,
            Err(e) => {
                log::error!(
                    "Error inserting plenar info: '{e}'. Error occured for plenar info: {plenar_info:?}. Skipping adding call to orders and debates."
                );
                continue;
            }
        };

        for call_to_order_entry in &plenar_info.call_to_orders {
            if let Err(e) = insert_call_to_order(call_to_order_entry, plenar_id, &mut conn) {
                log::warn!("Error inserting call to order: '{e}'. Error occured for call to order: {call_to_order_entry:?}");
            }
        }

        for debate in &plenar_info.debates {
            // get id from here, log here

            let debate_id = match insert_debate(debate, plenar_id, &mut conn) {
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
                    insert_plenar_speech(plenar_speech, debate_id, &mut conn, &mut text_extractor)
                        .await
                {
                    log::warn!(
                        "Error inserting plenar speech: '{e}'. Error occured for plenar speech: {plenar_speech:?}"
                    );
                }
            }
        }
    }
}

static mut NO_OPINION_COUNT: usize = 0;

async fn insert_plenar_speech(
    plenar_speech: &PlenarSpeech,
    val_debate_id: i32,
    conn: &mut PgConnection,
    text_extractor: &mut TextExtractor,
) -> QueryResult<()> {
    let disable_speech_text = std::env::var("DISABLE_SPEECH_TEXT").unwrap_or("true".into());
    let disable_speech_text = disable_speech_text == "true";

    let db_plenar_speech = DbPlenarSpeech {
        delegate_id: plenar_speech.delegate_id as i32,
        debate_id: val_debate_id,
        opinion: match &plenar_speech.opinion {
            scraper::Opinion::Infavor => Some("Pro".into()),
            scraper::Opinion::Against => Some("Contra".into()),
            scraper::Opinion::Other(text) => {
                unsafe { NO_OPINION_COUNT += 1 };
                log::info!(
                    "No opinion for speech: {:?}. 'Opinion': {text}, no opinion count: {}",
                    plenar_speech.spoken_text_html_urls,
                    unsafe { NO_OPINION_COUNT }
                );
                Some(text.clone())
            }
            scraper::Opinion::None => {
                unsafe { NO_OPINION_COUNT += 1 };
                log::info!(
                    "No opinion for speech: {:?}, no opinion_count: {}",
                    plenar_speech.spoken_text_html_urls,
                    unsafe { NO_OPINION_COUNT }
                );
                None
            }
        },
        spoken_text: None,
        duration_in_seconds: plenar_speech.duration_in_seconds as i32,
        ty: plenar_speech.ty.clone(),
    };
    // Some(text_extractor.extract_text_from_url(&plenar_speech).await)

    let plenar_speech_id = diesel::insert_into(plenar_speeches::table)
        .values(&db_plenar_speech)
        .on_conflict((plenar_speeches::debate_id, plenar_speeches::delegate_id))
        .do_update()
        .set(&db_plenar_speech)
        .returning(plenar_speeches::id)
        .get_result(conn)?;

    for url in &plenar_speech.spoken_text_html_urls {
        let psl = DbPlenarSpeechLink {
            plenar_speech_id,
            document_url: url.to_string()
        };
        diesel::insert_into(plenar_speech_links::table)
            .values(&psl)
            .on_conflict((plenar_speech_links::plenar_speech_id, plenar_speech_links::document_url))
            .do_nothing()
            .execute(conn)?;
    }
    {
        use crate::db::schema::plenar_speeches::dsl::plenar_speeches;
        use crate::db::schema::plenar_speeches::{debate_id, delegate_id, spoken_text};

        let has_spoken_text: Vec<Option<String>> = plenar_speeches
            .select(spoken_text)
            .filter(debate_id.eq(db_plenar_speech.debate_id))
            .filter(delegate_id.eq(db_plenar_speech.delegate_id))
            .load::<Option<String>>(conn)?;
        if has_spoken_text.len() == 1 && has_spoken_text[0].is_none() {
            let Some(text_html_url) = plenar_speech.spoken_text_html_urls.first() else {
                log::warn!(
                    "DEBATE_ID: {val_debate_id}, No spoken text html urls for plenar speech: {:?}",
                    plenar_speech
                );
                return Ok(());
            };

            let updated_spoken_text = if disable_speech_text {
                None
            } else {
                Some(text_extractor.extract_text_from_url(text_html_url).await)
            };

            diesel::update(plenar_speeches)
                .filter(debate_id.eq(db_plenar_speech.debate_id))
                .filter(delegate_id.eq(db_plenar_speech.delegate_id))
                .set(spoken_text.eq(updated_spoken_text))
                .execute(conn)?;
        }
    }
    Ok(())
}

fn insert_debate(debate: &Debate, plenar_id: i32, conn: &mut PgConnection) -> QueryResult<i32> {
    let db_debate = DbDebate {
        plenar_id,
        about: debate.about.clone(),
    };
    diesel::insert_into(debates::table)
        .values(&db_debate)
        .on_conflict((debates::plenar_id, debates::about))
        .do_update()
        .set(&db_debate)
        .returning(debates::id)
        .get_result(conn)
}

pub fn insert_plenar_info(plenar_info: &PlenarInfo, conn: &mut PgConnection) -> QueryResult<i32> {
    let db_plenar_info = DbPlenar {
        inr: plenar_info.inr as i32,
        title: plenar_info.title.clone(),
        description: plenar_info.description.clone(),
        add_date: plenar_info.date.clone(),
        legislative_period: plenar_info.legislative_period.clone(),
    };
    diesel::insert_into(plenar_infos::table)
        .values(&db_plenar_info)
        .on_conflict((
            plenar_infos::inr,
            plenar_infos::add_date,
            plenar_infos::legislative_period,
        ))
        .do_update()
        .set(&db_plenar_info)
        .returning(plenar_infos::id)
        .get_result::<i32>(conn)
}

pub fn insert_call_to_order(
    call_to_order: &CallToOrderInfo,
    plenar_id: i32,
    conn: &mut PgConnection,
) -> QueryResult<usize> {
    let db_call_to_order = DbCallToOrder {
        id: format!("{}/{}", call_to_order.ityp, call_to_order.inr),
        plenar_id,
        inr: call_to_order.inr as i32,
        receiver_id: call_to_order.receiver_id.clone() as i32,
    };
    diesel::insert_into(call_to_order::table)
        .values(&db_call_to_order)
        .on_conflict(call_to_order::id)
        .do_nothing()
        .execute(conn)
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
                    plenar_info_once().await;
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
