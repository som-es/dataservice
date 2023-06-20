use diesel::RunQueryDsl;
use scraper::{AustriaScraper, parties::Parties, Party};

use crate::{Shutdown, scrape_untill_success, schema::parties, models::DbParty};

pub fn update_parties_in_db(parties: &[Party]) {
    let conn = &mut crate::establish_connection();
    
    for party in parties {
        let db_party = DbParty {
            name: party.name.clone(),
            fraction: party.fraction as i32,
            color: party.color.clone(),
        };
        diesel::insert_into(parties::table)
            .values(&db_party)
            .on_conflict(parties::name)
            .do_nothing()
            .execute(conn)
            .expect("Error saving new party");
    }
}

pub async fn parties_once() {
    let mut scraper = AustriaScraper::default();
    for gp in ["XXVII", "XXVI", "XXV", "XXIV", "XXIII", "XXII"] {
        scraper.scrape_info.gp = vec![gp.into()];
        
        let parties = scrape_untill_success::<Vec<Party>, _>(|| async {
            scraper.get_parties().await.map_err(|e| e.into())
        })
        .await;

        update_parties_in_db(&parties);
    }
}

pub async fn run_shut(mut shutdown: Shutdown) {
    let party_sleep = std::env::var("PARTY_SLEEP")
        .unwrap_or_else(|_| "10000".to_string())
        .parse()
        .unwrap_or_else(|e| {
            log::warn!(
                "Could not parse PARTY_SLEEP env var: {e}. Using default value of 10000 seconds"
            );
            10000
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
                    log::info!("Starting party scraper");
                }).await {
                    log::error!("Error in party scraper: {:?}", e);
                }
                parties_once().await;
                log::info!("Party scraper service sleeps for {party_sleep} seconds...");
                tokio::time::sleep(tokio::time::Duration::from_secs(party_sleep)).await;
            } => {}
        }
    }
}
