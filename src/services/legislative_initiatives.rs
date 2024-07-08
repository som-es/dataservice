use diesel::RunQueryDsl;
use scraper::{legis_init::LegislativeInitiatives, AustriaScraper, LegislativeInitiative, Opinion};

use crate::{
    models::{
        DbEurovocTopicLegisInit, DbLegisDocument, DbLegislativeInitiative,
        DbOtherKeywordTopicLegisInit, DbSpeech, DbTopicLegisInit, DbVote,
    },
    schema::{
        eurovoc_topics_legis_init, legislative_documents, legislative_initiatives,
        other_keyword_topics_legis_init, speeches, topics_legis_init, votes,
    },
    services::scrape_untill_success,
    Shutdown,
};

pub async fn scrape_new_legis_inits(
    scraper: &AustriaScraper,
) -> crate::Result<Vec<LegislativeInitiative>> {
    let legis_inits = scraper.post_legis_payload().await?;
    let legis_init_paths = scraper.legis_paths(legis_inits)?;

    let mut new_legis_inits = Vec::new();

    for legis_init_path in &legis_init_paths {
        // skipping legis inits that can't be processed right now
        // mainly many I and A
        // https://www.parlament.gv.at/gegenstand/XXVII/I/1494
        // https://www.parlament.gv.at/gegenstand/XXVII/A/2686
        let Ok(legis_init) = scraper.get_legis_init(&legis_init_path).await else {
            log::debug!("Cannot process legislative initiative: '{legis_init_path}'");
            continue;
        };

        new_legis_inits.push(legis_init);
    }

    Ok(new_legis_inits)
}

async fn legis_init_once(only_latest: bool) -> Vec<LegislativeInitiative> {
    log::info!("Try scraping legislative initiatives");
    let mut scraper = AustriaScraper::default();

    for gp in ["XXVII", "XXVI", "XXV", "XXIV", "XXIII", "XXII", "XXI", "XX"] {
        log::info!("LEGIS INIT once GP: {gp}");
        scraper.scrape_info.gp = vec![gp.into()];

        let new_legis_inits = scrape_untill_success::<Vec<LegislativeInitiative>, _>(|| async {
            scrape_new_legis_inits(&scraper).await
        })
        .await;

        update_legis_inits_in_db(&new_legis_inits);
        if only_latest {
            break;
        }
    }

    // new_legis_inits
    vec![]
}

fn update_legis_inits_in_db(new_legis_inits: &[LegislativeInitiative]) {
    log::info!("Updating legislative initiative in database...");

    let conn = &mut crate::establish_connection();
    for legis_init in new_legis_inits {
        let db_legis_init = DbLegislativeInitiative {
            title: legis_init.title.clone(),
            inr: legis_init.inr as i32,
            emphasis: legis_init.emphasis.clone(),
            description: legis_init.description.clone(),
            ityp: legis_init.ityp.clone(),
            gp: legis_init.gp.clone(),
            accepted: legis_init.accepted,
            created_at: legis_init.date,
            appeared_at: Some(legis_init.created_at),
            updated_at: legis_init.updated_at,
        };

        let legis_init_id = match diesel::insert_into(legislative_initiatives::table)
            .values(&db_legis_init)
            .on_conflict((
                legislative_initiatives::inr,
                legislative_initiatives::ityp,
                legislative_initiatives::gp,
            ))
            .do_update()
            // .set((legislative_initiatives::created_at.eq(db_legis_init.created_at),))
            .set(&db_legis_init)
            .returning(legislative_initiatives::id)
            .get_result(conn)
        {
            Ok(id) => id,
            Err(e) => {
                log::error!("Error inserting legislative initiative: '{e}'. Error occured for legislative initiative: {legis_init:?}");
                continue;
            }
        };

        for vote in &legis_init.votes {
            let db_vote = DbVote {
                party: vote.party.name.clone(),
                fraction: vote.party.fraction as i32,
                infavor: vote.infavor,
                legislative_initiatives_id: legis_init_id,
            };

            if let Err(e) = diesel::insert_into(votes::table)
                .values(&db_vote)
                .on_conflict((votes::legislative_initiatives_id, votes::party))
                .do_update()
                .set(&db_vote)
                .execute(conn)
            {
                log::warn!("Error inserting vote: '{e}'. The error occured for the following vote: {vote:?}")
            }
        }

        for speech in &legis_init.speeches {
            let db_speech = DbSpeech {
                delegate_id: speech.delegate_id as i32,

                // TODO: get opinion even if none is given
                infavor: match speech.opinion {
                    Opinion::Infavor => Some(true),
                    Opinion::Against => Some(false),
                    _ => None,
                },
                legislative_initiatives_id: legis_init_id,
            };

            if let Err(e) = diesel::insert_into(speeches::table)
                .values(&db_speech)
                .on_conflict((speeches::legislative_initiatives_id, speeches::delegate_id))
                .do_update()
                .set(&db_speech)
                .execute(conn)
            {
                log::warn!("Error inserting speech, probably delegate does not exist in database and has got inactive recently: {e}, for speech: {speech:?}")
            }
        }

        for topic in legis_init.topics.clone() {
            let topic = DbTopicLegisInit {
                topic,
                legislative_initiatives_id: legis_init_id,
            };

            if let Err(e) = diesel::insert_into(topics_legis_init::table)
                .values(&topic)
                .on_conflict((
                    topics_legis_init::legislative_initiatives_id,
                    topics_legis_init::topic,
                ))
                .do_update()
                .set(&topic)
                .execute(conn)
            {
                log::error!("Error inserting topic: '{e}'. The error occured for the following topic: {topic:?}")
            }
        }

        for topic in legis_init.eurovoc_topics.clone() {
            let topic = DbEurovocTopicLegisInit {
                topic,
                legislative_initiatives_id: legis_init_id,
            };

            if let Err(e) = diesel::insert_into(eurovoc_topics_legis_init::table)
                .values(&topic)
                .on_conflict((
                    eurovoc_topics_legis_init::legislative_initiatives_id,
                    eurovoc_topics_legis_init::topic,
                ))
                .do_update()
                .set(&topic)
                .execute(conn)
            {
                log::error!("Error inserting topic: '{e}'. The error occured for the following eurovoc topic: {topic:?}")
            }
        }

        for topic in legis_init.other_keyword_topics.clone() {
            let topic = DbOtherKeywordTopicLegisInit {
                topic,
                legislative_initiatives_id: legis_init_id,
            };

            if let Err(e) = diesel::insert_into(other_keyword_topics_legis_init::table)
                .values(&topic)
                .on_conflict((
                    other_keyword_topics_legis_init::legislative_initiatives_id,
                    other_keyword_topics_legis_init::topic,
                ))
                .do_update()
                .set(&topic)
                .execute(conn)
            {
                log::error!("Error inserting topic: '{e}'. The error occured for the following other keyword topic: {topic:?}")
            }
        }

        for document in &legis_init.documents {
            let legis_doc = DbLegisDocument {
                legislative_initiatives_id: legis_init_id,
                title: document.title.clone(),
                document_type: document.ftype.to_string(),
                document_url: document.link.clone(),
            };

            if let Err(e) = diesel::insert_into(legislative_documents::table)
                .values(&legis_doc)
                .on_conflict((
                    legislative_documents::document_url,
                    legislative_documents::legislative_initiatives_id,
                ))
                .do_update()
                .set(&legis_doc)
                .execute(conn)
            {
                log::error!("Error inserting document: '{e}'. The error occured for the following document: {document:?}")
            }
        }
    }
}

pub async fn run_shut_latest_gp(mut shutdown: Shutdown) {
    let legis_init_sleep = std::env::var("LEGISLATIVE_INITIATIVE_SLEEP_LATEST")
        .unwrap_or_else(|_| "50".to_string())
        .parse()
        .unwrap_or_else(|e| {
            log::warn!(
                "Could not parse LEGISLATIVE_INITIATIVE_SLEEP_LATEST env var: {e}. Using default value of 50 seconds"
            );
            50
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
                    log::info!("Starting legislative initiative (latest) scraper");
                    legis_init_once(true).await;
                }).await {
                    log::error!("Error in legislative initiative (latest) scraper: {:?}", e);
                }
                // legis_init_once().await;
                log::info!("Legislative initiative (latest) scraper service sleeps for {legis_init_sleep} seconds...");
                tokio::time::sleep(tokio::time::Duration::from_secs(legis_init_sleep)).await;
            } => {}
        }
    }
}

pub async fn run_shut_every_gp(mut shutdown: Shutdown) {
    let legis_init_sleep = std::env::var("LEGISLATIVE_INITIATIVE_SLEEP")
        .unwrap_or_else(|_| "1000".to_string())
        .parse()
        .unwrap_or_else(|e| {
            log::warn!(
                "Could not parse LEGISLATIVE_INITIATIVE_SLEEP env var: {e}. Using default value of 1000 seconds"
            );
            1000
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
                    log::info!("Starting legislative initiative scraper");
                    legis_init_once(false).await;
                }).await {
                    log::error!("Error in legislative initiative scraper: {:?}", e);
                }
                // legis_init_once().await;
                log::info!("Legislative initiative scraper service sleeps for {legis_init_sleep} seconds...");
                tokio::time::sleep(tokio::time::Duration::from_secs(legis_init_sleep)).await;
            } => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDateTime;
    use scraper::AustriaScraper;

    #[test]
    fn test_date_time_parse() {
        let x = NaiveDateTime::parse_from_str("2014-5-17T12:34:56", "%Y-%m-%dT%H:%M:%S").unwrap();
        println!("x: {x:?}");
    }

    #[tokio::test]
    async fn test_legis_init_empty_gp_scrape() {
        let mut scraper: AustriaScraper = AustriaScraper::default();
        // println!("s: {:?}", scraper.scrape_info.gp);

        // let out = crate::legislative_initiatives::scrape_new_legis_inits(&scraper).await.unwrap();
        // println!("out: {out:?}");
    }
}
