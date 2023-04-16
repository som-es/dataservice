use dotenvy::dotenv;
use scraper::{legis_init::{LegislativeInitiative, LegislativeInitiatives}, AustriaScraper};

use crate::{services::scrape_untill_success, today, Shutdown};

async fn scrape_new_legis_inits(
) -> crate::Result<Vec<LegislativeInitiative>> {
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
    for legis_init in new_legis_inits {
        println!("legis_init id: {}, title: {}", legis_init.id, legis_init.title);
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
