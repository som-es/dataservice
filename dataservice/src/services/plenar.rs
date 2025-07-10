use std::thread::JoinHandle;

use crate::{
    legislative_initiatives::absence_matching, services::scrape_untill_success,
    should_update::StoredUpdatedAt, Shutdown, GPS,
};
use somes_scraper::{
    call_to_order::CallToOrderInfo,
    plenar::{Debate, DebateInfo, Plenar, PlenarInfo},
    text_utils, AustriaScraper, PlenarSpeech, Reference, Scraper, TextExtractor,
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

async fn plenar_info_once(pg_pool: &PgPool, only_latest: bool) {
    log::info!("Try scraping plenar info");

    let tok_handle = Handle::current();

    let mut handles = vec![];

    for gp in GPS {
        let pg_pool = pg_pool.clone();
        let tok_handle = tok_handle.clone();

        // tokio::task::spawn_local(future)

        handles.push(std::thread::spawn(move || {
            tok_handle.block_on(async move {
                let mut scraper = AustriaScraper::default();
                log::info!("PLENAR info once GP: {gp}");
                scraper.scrape_info.gp = vec![gp.into()];

                let new_plenar_infos = scrape_untill_success::<Vec<PlenarInfo>, _>(|| async {
                    scrape_new_plenar_infos(&scraper).await
                })
                .await;

                let mut handles: Vec<JoinHandle<()>> = vec![];

                for plenar_info in new_plenar_infos.into_iter() {
                    if handles.len() > 32 {
                        log::info!("awaiting 32 plenar handles");
                        for handle in handles.drain(..) {
                            if let Err(e) = handle.join() {
                                log::error!("could not await plenar info handle: {e:?}");
                            }
                        }
                    }
                    let pg_pool = pg_pool.clone();

                    let tok_handle = Handle::current();
                    handles.push(std::thread::spawn(move || {
                        tok_handle.block_on(async move {
                            update_plenar_infos_in_db(&[plenar_info], &pg_pool).await;
                        })
                    }));
                }
                for handle in handles.drain(..) {
                    if let Err(e) = handle.join() {
                        log::error!("could not await plenar info handle: {e:?}");
                    }
                }
            })
        }));
        if only_latest {
            break;
        }
    }
    for handle in handles.drain(..) {
        if let Err(e) = handle.join() {
            log::error!("could not await plenar info handle: {e:?}");
        }
    }
}

async fn update_plenar_infos_in_db(new_plenar_infos: &[PlenarInfo], pg_pool: &PgPool) {
    log::info!("Updating plenar infos in database...");

    // disable with setting
    let mut text_extractor = somes_scraper::TextExtractor::new(AustriaScraper::BASE_URL);

    for plenar_info in new_plenar_infos {
        if !crate::should_update(plenar_info, pg_pool).await {
            continue;
        }
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
                if let Err(e) = insert_plenar_speech(
                    pg_pool,
                    plenar_speech,
                    &debate.info,
                    debate_id,
                    &mut text_extractor,
                )
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
                log::info!("----------------------------------------------------- INSERTING MATCH RES {} -------- !!!!!!!!!!!!!!", plenar_info.gp);
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
use sqlx::{postgres::PgQueryResult, PgPool};
use tokio::runtime::{Handle, Runtime};

static mut NO_OPINION_COUNT: usize = 0;

async fn insert_plenar_speech_legis_init(
    pool: &PgPool,
    legis_init_id: Option<i32>,
    plenar_speech_id: i32,
    reference: &Reference,
) -> sqlx::Result<PgQueryResult> {
    sqlx::query!("
            insert into 
                plenar_speech_legis_inits (gp, ityp, inr, speech_id, legis_init_id) 
            values ($1, $2, $3, $4, $5) 
            on conflict (gp, ityp, inr, speech_id) do update set legis_init_id = EXCLUDED.legis_init_id", reference.gp, reference.ityp, reference.inr as i32, plenar_speech_id, legis_init_id).execute(pool).await
}

pub struct DbReference {
    origin_legis_init_id: i32,
    ref_gp: String,
    ref_ityp: String,
    ref_inr: i32,
}

async fn insert_plenar_speech(
    pool: &PgPool,
    plenar_speech: &PlenarSpeech,
    corresponding_debate: &DebateInfo,
    val_debate_id: i32,
    text_extractor: &mut TextExtractor,
) -> Result<(), sqlx::Error> {
    let disable_speech_text =
        std::env::var("DISABLE_SPEECH_TEXT").unwrap_or_else(|_| "true".into()) == "true";

    let opinion = match &plenar_speech.speech.opinion {
        somes_scraper::Opinion::Infavor => Some("Pro".to_string()),
        somes_scraper::Opinion::Against => Some("Contra".to_string()),
        somes_scraper::Opinion::Other(text) => {
            unsafe { NO_OPINION_COUNT += 1 };
            info!(
                "No opinion for speech: {:?}. 'Opinion': {text}, no opinion count: {}",
                plenar_speech.speech.spoken_text_html_urls,
                unsafe { NO_OPINION_COUNT }
            );
            Some(text.clone())
        }
        somes_scraper::Opinion::None => {
            unsafe { NO_OPINION_COUNT += 1 };
            info!(
                "No opinion for speech: {:?}, no opinion_count: {}",
                plenar_speech.speech.spoken_text_html_urls,
                unsafe { NO_OPINION_COUNT }
            );
            None
        }
    };

    let plenar_speech_id = sqlx::query!(
        r#"
        INSERT INTO plenar_speeches (delegate_id, debate_id, opinion, spoken_text, duration_in_seconds, ty, delegate_display_name)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        ON CONFLICT (debate_id, delegate_id, duration_in_seconds, ty)
        DO UPDATE SET
            opinion = EXCLUDED.opinion,
            -- spoken_text = EXCLUDED.spoken_text, -- setting this to None on conflict results in fluctuating none counts etc
            duration_in_seconds = EXCLUDED.duration_in_seconds,
            ty = EXCLUDED.ty
        RETURNING id
        "#,
        plenar_speech.speech.delegate_id as i32,
        val_debate_id,
        opinion,
        None::<String>, // spoken_text is initially None
        plenar_speech.duration_in_seconds as i32,
        plenar_speech.ty,
        plenar_speech.speech.delegate_display_name,
    )
    .fetch_one(pool)
    .await?
    .id;

    for reference in &corresponding_debate.references {
        let legis_init_id = if let Ok(x) = sqlx::query!(
            "select id from legislative_initiatives where gp = $1 and ityp = $2 and inr = $3",
            reference.gp,
            reference.ityp,
            reference.inr as i32
        )
        .fetch_one(pool)
        .await
        {
            Some(x.id)
        } else {
            None
        };

        // let own_references = i
        if let Err(e) =
            insert_plenar_speech_legis_init(pool, legis_init_id, plenar_speech_id, reference).await
        {
            log::info!("could not insert plenar speech legis init for legis int {legis_init_id:?} and speech {plenar_speech_id}: {e:?}")
        }
        if let Some(legis_init_id) = legis_init_id {
            let own_references = sqlx::query_as!(DbReference, "select origin_legis_init_id, ref_gp, ref_ityp, ref_inr from legis_inits_refs where origin_legis_init_id = $1", legis_init_id).fetch_all(pool).await.unwrap_or_default();

            for reference in own_references {
                let legis_init_id = if let Ok(x) = sqlx::query!(
                    "select id from legislative_initiatives where gp = $1 and ityp = $2 and inr = $3",
                    reference.ref_gp,
                    reference.ref_ityp,
                    reference.ref_inr
                )
                .fetch_one(pool)
                .await
                {
                    Some(x.id)
                } else {
                    None
                };

                if let Err(e) = insert_plenar_speech_legis_init(
                    pool,
                    legis_init_id,
                    plenar_speech_id,
                    &Reference {
                        gp: reference.ref_gp,
                        ityp: reference.ref_ityp,
                        inr: reference.ref_inr as u32,
                    },
                )
                .await
                {
                    log::info!("could not insert plenar speech legis init for legis int {legis_init_id:?} and speech {plenar_speech_id}: {e:?}")
                }
            }
        }
    }

    for url in &plenar_speech.speech.spoken_text_html_urls {
        sqlx::query!(
            r#"
            INSERT INTO plenar_speech_links (plenar_speech_id, document_url, site_start, site_end)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (plenar_speech_id, document_url) DO NOTHING
            "#,
            plenar_speech_id,
            url.document_url.to_string(),
            url.site_start.map(|i| i as i32),
            url.site_end.map(|i| i as i32)
        )
        .execute(pool)
        .await?;
    }

    // let existing_spoken_text = sqlx::query!(
    //     r#"
    //     SELECT spoken_text FROM plenar_speeches
    //     WHERE debate_id = $1 AND delegate_id = $2
    //     "#,
    //     val_debate_id,
    //     plenar_speech.speech.delegate_id as i32
    // )
    // .fetch_optional(pool)
    // .await?;

    // if let Some(record) = existing_spoken_text {
    // if record.spoken_text.is_none() {
    if let Some(text_html_url) = plenar_speech.speech.spoken_text_html_urls.first() {
        let updated_spoken_text = if disable_speech_text {
            None
        } else {
            text_extractor
                .speech_extraction_with_fit(
                    &text_html_url,
                    &plenar_speech.speech.delegate_display_name,
                )
                .await
            // Some(
            //     text_extractor
            //         .extract_text_from_url(&text_html_url.document_url, None, true)
            //         .await,
            // )
        };
        if updated_spoken_text.is_none() {
            warn!(
                        "DEBATE_ID: {}, Missing spoken text for plenar speech id: {plenar_speech_id}, and plenar info: {:?}",
                        val_debate_id, plenar_speech
                    );
        }

        if let Some(updated_spoken_text) = &updated_spoken_text {
            let (removed_parans, _) =
                text_utils::remove_paran_contents(updated_spoken_text, '(', ')');
            for removed_paran in removed_parans {
                // "asdfhasdjklf ahsdfhjklasd (Abg. Hallo: Das ist eine Frechheit!)"
                // TODO: Issue #25
            }
        }

        sqlx::query!(
            r#"
                    UPDATE plenar_speeches
                    SET spoken_text = $1
                    WHERE id = $2 
                    "#,
            updated_spoken_text,
            plenar_speech_id,
        )
        .execute(pool)
        .await?;
    } else {
        warn!(
            "DEBATE_ID: {}, No spoken text html urls for plenar speech: {:?}",
            val_debate_id, plenar_speech
        );
    }
    // }
    // }

    Ok(())
}

impl StoredUpdatedAt for PlenarInfo {
    async fn stored_updated_at(&self, pg: &PgPool) -> Option<chrono::NaiveDateTime> {
        sqlx::query!(
            "select updated_at from plenar_infos where inr = $1",
            self.inr as i32
        )
        .fetch_optional(pg)
        .await
        .ok()
        .flatten()
        .map(|x| x.updated_at)
        .flatten()
    }
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
        debate.info.about
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
        INSERT INTO plenar_infos (inr, title, description, add_date, legislative_period, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (inr, legislative_period)
        DO UPDATE SET
            title = EXCLUDED.title,
            description = EXCLUDED.description,
            updated_at = EXCLUDED.updated_at
        RETURNING id
        "#,
        plenar_info.inr as i32,
        plenar_info.title,
        plenar_info.description,
        plenar_info.date,
        plenar_info.gp,
        plenar_info.updated_at
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

pub async fn run_shut_latest(mut shutdown: Shutdown, pg_pool: PgPool) {
    // insert_all_serially().await;
    // return;
    let plenar_sleep = std::env::var("PLENAR_SLEEP_LATEST")
        .unwrap_or_else(|_| "800".to_string())
        .parse()
        .unwrap_or_else(|e| {
            log::warn!(
                "Could not parse PLENAR_SLEEP_LATEST env var: {e}. Using default value of 800 seconds"
            );
            800
        });

    #[cfg(feature = "plenar_latest")]
    while !shutdown.is_shutdown() {
        tokio::select! {
            _ = shutdown.recv() => {
                println!("shutting down");
                // return ok?
                break;
            }
            _ = async {
                let pg_pool = pg_pool.clone();
                if let Err(e) = tokio::task::spawn_blocking(move || Handle::current().block_on(async move {
                    log::info!("Starting latest plenar info scraper");
                    plenar_info_once(&pg_pool, true).await;
                })).await {
                    log::error!("Error in plenar info scraper: {:?}", e);
                }
                // legis_init_once().await;
                log::info!("Latest Plenar info scraper service sleeps for {plenar_sleep} seconds...");
                tokio::time::sleep(tokio::time::Duration::from_secs(plenar_sleep)).await;
            } => {}
        }
    }
}

pub async fn run_shut(mut shutdown: Shutdown, pg_pool: PgPool) {
    // insert_all_serially().await;
    // return;
    let plenar_sleep = std::env::var("PLENAR_SLEEP")
        .unwrap_or_else(|_| "9000".to_string())
        .parse()
        .unwrap_or_else(|e| {
            log::warn!(
                "Could not parse PLENAR_SLEEP env var: {e}. Using default value of 9000 seconds"
            );
            9000
        });

    #[cfg(feature = "plenar")]
    while !shutdown.is_shutdown() {
        tokio::select! {
            _ = shutdown.recv() => {
                println!("shutting down");
                // return ok?
                break;
            }
            _ = async {
                let pg_pool = pg_pool.clone();
                if let Err(e) = tokio::task::spawn_blocking(move || Handle::current().block_on(async move {
                    log::info!("Starting plenar info scraper");
                    plenar_info_once(&pg_pool, false).await;
                })).await {
                    log::error!("Error in plenar info scraper: {:?}", e);
                }
                // legis_init_once().await;
                log::info!("Plenar info scraper service sleeps for {plenar_sleep} seconds...");
                tokio::time::sleep(tokio::time::Duration::from_secs(plenar_sleep)).await;
            } => {}
        }
    }
}

async fn insert_all_serially(pg_pool: PgPool) {
    let mut scraper = AustriaScraper::default();

    let mut text_extractor = TextExtractor::new(AustriaScraper::BASE_URL);

    let mut yes = 0;
    let mut no = 0;

    std::env::set_var("SCRAPER_IGNORE_CACHE_XXVII", "false");
    let mut all_speeches_count = 0;
    'outer: for gp in ["XXVII", "XXVI", "XXV", "XXIV", "XXIII", "XXII", "XXI", "XX"] {
        // 'outer: for gp in ["XXI", "XX"] {
        scraper.scrape_info.gp = vec![gp.to_string()];

        let plenar_infos = scraper.post_plenar_payload().await.unwrap();

        let paths = scraper
            .plenar_paths(plenar_infos)
            .expect("Failed to get paths");

        for path in paths.iter().rev() {
            println!("path: {path}");
            // path with no content at 1: /gegenstand/XXVII/NRSITZ/216

            // newer ones do not provide a document

            let plenar_info = scraper.get_plenar(&path).await;
            let Ok(plenar_info) = plenar_info else {
                continue;
            };

            let plenar_id = match insert_plenar_info(&pg_pool, &plenar_info).await {
                Ok(id) => id,
                Err(e) => {
                    log::error!(
                                "Error inserting plenar info: '{e}'. Error occured for plenar info: {plenar_info:?}. Skipping adding call to orders and debates."
                            );
                    continue;
                }
            };

            for debate in &plenar_info.debates {
                // get id from here, log here

                let debate_id = match insert_debate(&pg_pool, debate, plenar_id).await {
                    Ok(debate_id) => debate_id,
                    Err(e) => {
                        log::warn!(
                            "Error inserting debate: '{e}'. Error occured for debate: {debate:?}"
                        );
                        continue;
                    }
                };

                for plenar_speech in &debate.speeches {
                    if let Err(e) = insert_plenar_speech(
                        &pg_pool,
                        plenar_speech,
                        &debate.info,
                        debate_id,
                        &mut text_extractor,
                    )
                    .await
                    {
                        no += 1;
                        log::warn!(
                        "Error inserting plenar speech: '{e}'. Error occured for plenar speech: {plenar_speech:?}"
                    );
                    } else {
                        yes += 1;
                    }
                }
            }

            // update_plenar_infos_in_db(&[plenar_info], &pg_pool).await;
            /*for debate in plenar_info.debates {

                for speech in debate.speeches {

                    if let Some(spoken_text_url) = speech.speech.spoken_text_html_urls.first() {
                        let text = extractor
                            .speech_extraction_with_fit(
                                spoken_text_url,
                                &speech.speech.delegate_display_name,
                            )
                            .await
                            .unwrap();

                        // if text.len() <= 100 {
                        //     println!("small text: {text}");
                        // }
                        all_speeches_count += 1;
                    }
                }
            }
            */
            // if all_speeches_count % 100 == 0 {
            //     dbg!("all s c: {all_speeches_count}");
            //     break 'outer;
            // }
            // let speeches_count: usize = plenar_info.debates.iter().map(|d| d.speeches.len()).sum();
            // all_speeches_count += speeches_count;
            // println!("plenar_info: {:?}", plenar_info.absences);
        }
    }

    // 20805
    println!("speeches: {all_speeches_count}");
    println!("yes: {yes}, no: {no}");
}

#[cfg(test)]
mod tests {
    use std::{collections::HashSet, io::Read};

    use chrono::NaiveDateTime;
    use rand::Rng;
    use somes_scraper::{AustriaScraper, TextExtractor};

    use crate::connect_pg;

    #[test]
    fn test_date_time_parse() {
        let x = NaiveDateTime::parse_from_str("2014-5-17T12:34:56", "%Y-%m-%dT%H:%M:%S").unwrap();
        println!("x: {x:?}");
    }

    #[tokio::test]
    async fn test_duplicate_speeches() {
        let pool = connect_pg().await;

        println!("stating selecting");
        let speeches = sqlx::query!("select * from plenar_speeches")
            .fetch_all(&pool)
            .await
            .unwrap();
        println!("finished selecting speeches");

        let mut unique_speeches = HashSet::new();
        for speech in &speeches {
            if let Some(spoken_text) = &speech.spoken_text {
                if !unique_speeches.insert(spoken_text) {
                    // println!("speech: {speech:?}");
                }
            }
        }

        let mut rng = rand::thread_rng();
        loop {
            let idx = rng.gen_range(0..speeches.len());
            let speech = &speeches[idx];
            println!();
            println!(
                "speech: {}",
                speech.spoken_text.clone().unwrap_or("".to_string())
            );
            println!("{:?}", speech);

            println!();

            let mut input = String::new();
            std::io::stdin().read_to_string(&mut input).unwrap();
            println!("input: {input}")
        }
    }
}
