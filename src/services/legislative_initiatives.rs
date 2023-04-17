use diesel::RunQueryDsl;
use scraper::{legis_init::LegislativeInitiatives, AustriaScraper, LegislativeInitiative};

use crate::{
    models::{DbLegislativeInitiative, DbVote},
    schema::{legislative_initiatives, votes},
    services::scrape_untill_success,
    Shutdown,
};

async fn scrape_new_legis_inits() -> crate::Result<Vec<LegislativeInitiative>> {
    let scraper = AustriaScraper::default();

    let legis_inits = scraper.post_legis_payload().await?;
    let legis_init_paths = scraper.legis_paths(legis_inits)?;

    let mut new_legis_inits = Vec::new();

    for legis_init_path in &legis_init_paths {
        // skipping proposals that can't be processed right now
        let Ok(legis_init) = scraper.get_legis_init(&legis_init_path).await else {
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
    for legis_init in new_legis_inits {
        let db_legis_init = DbLegislativeInitiative {
            id: legis_init.id.clone(),
            title: legis_init.title.clone(),
            description: legis_init.description.clone(),
            ityp: legis_init.ityp.clone(),
            gp: legis_init.gp.clone(),
            accepted: legis_init.accepted,
        };

        diesel::insert_into(legislative_initiatives::table)
            .values(&db_legis_init)
            .on_conflict(legislative_initiatives::id)
            .do_nothing()
            .execute(&mut crate::establish_connection())
            .expect("msg");

        for vote in &legis_init.votes {
            let db_vote = DbVote {
                party: vote.party.clone(),
                fraction: vote.fraction as i32,
                infavor: vote.infavor,
                legislative_initiatives_id: db_legis_init.id.clone(),
            };

            diesel::insert_into(votes::table)
                .values(&db_vote)
                .on_conflict((votes::legislative_initiatives_id, votes::party))
                .do_nothing()
                .execute(&mut crate::establish_connection())
                .expect("msg");
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
                legis_init_once().await;
                tokio::time::sleep(tokio::time::Duration::from_secs(10000)).await;
            } => {}
        }
    }
}
