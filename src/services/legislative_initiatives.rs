use chrono::NaiveDateTime;
use diesel::RunQueryDsl;
use scraper::{legis_init::LegislativeInitiatives, AustriaScraper, LegislativeInitiative, Opinion};

use crate::{
    models::{DbLegislativeInitiative, DbSpeech, DbVote},
    schema::{legislative_initiatives, speeches, votes},
    services::scrape_untill_success,
    today_and_time, Shutdown,
};

async fn scrape_new_legis_inits() -> crate::Result<Vec<LegislativeInitiative>> {
    let scraper = AustriaScraper::default();

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

async fn legis_init_once() -> Vec<LegislativeInitiative> {
    log::info!("Try scraping legislative initiatives");

    let new_legis_inits = scrape_untill_success::<Vec<LegislativeInitiative>, _>(|| async {
        scrape_new_legis_inits().await
    })
    .await;
    update_legis_inits_in_db(&new_legis_inits);

    new_legis_inits
}

fn update_legis_inits_in_db(new_legis_inits: &[LegislativeInitiative]) {
    log::info!("Updating legislative initiative in database...");

    let conn = &mut crate::establish_connection();
    for legis_init in new_legis_inits {
        let db_legis_init = DbLegislativeInitiative {
            id: legis_init.id.clone(),
            title: legis_init.title.clone(),
            emphasis: legis_init.emphasis.clone(),
            description: legis_init.description.clone(),
            ityp: legis_init.ityp.clone(),
            gp: legis_init.gp.clone(),
            accepted: legis_init.accepted,
            created_at: NaiveDateTime::parse_from_str(&legis_init.date, "%Y-%m-%dT%H:%M:%S")
                .unwrap_or(today_and_time()),
        };

        if let Err(e) = diesel::insert_into(legislative_initiatives::table)
            .values(&db_legis_init)
            .on_conflict(legislative_initiatives::id)
            .do_nothing()
            .execute(conn)
        {
            log::error!("Error inserting legislative initiative: '{e}'. Error occured for legislative initiative: {legis_init:?}");
            continue;
        }

        for vote in &legis_init.votes {
            let db_vote = DbVote {
                party: vote.party.clone(),
                fraction: vote.fraction as i32,
                infavor: vote.infavor,
                legislative_initiatives_id: db_legis_init.id.clone(),
            };

            if let Err(e) = diesel::insert_into(votes::table)
                .values(&db_vote)
                .on_conflict((votes::legislative_initiatives_id, votes::party))
                .do_nothing()
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
                    _ => {
                        log::info!("No opinion given for speech: {speech:?}");
                        None
                    }
                },
                legislative_initiatives_id: db_legis_init.id.clone(),
            };

            if let Err(e) = diesel::insert_into(speeches::table)
                .values(&db_speech)
                .on_conflict((speeches::legislative_initiatives_id, speeches::delegate_id))
                .do_nothing()
                .execute(conn)
            {
                log::warn!("Error inserting speech, probably delegate does not exist in database and has got inactive recently: {e}, for speech: {speech:?}")
            }
        }
    }
}

pub async fn run_shut(mut shutdown: Shutdown) {
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
                    legis_init_once().await;
                }).await {
                    log::error!("Error in legislative initiative scraper: {:?}", e);
                }
                // legis_init_once().await;
                tokio::time::sleep(tokio::time::Duration::from_secs(1000)).await;
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
