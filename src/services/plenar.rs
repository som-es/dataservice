use scraper::{legis_init::LegislativeInitiatives, AustriaScraper, plenar::{Plenar, PlenarInfo}};

use crate::{
    services::scrape_untill_success, Shutdown,
};

async fn scrape_new_plenar_infos() -> crate::Result<Vec<PlenarInfo>> {
    let scraper = AustriaScraper::default();

    let raw_plenar = scraper.post_plenar_payload().await?;
    let plenar_paths = scraper.legis_paths(raw_plenar)?;

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

    let new_plenar_infos = scrape_untill_success::<Vec<PlenarInfo>, _>(|| async {
        scrape_new_plenar_infos().await
    })
    .await;
    update_plenar_infos_in_db(&new_plenar_infos);

    new_plenar_infos
}

fn update_plenar_infos_in_db(new_plenar_infos: &[PlenarInfo]) {
    log::info!("Updating legislative initiative in database...");

    let mut conn = crate::establish_connection();
    for plenar_info in new_plenar_infos {    
        /*
        if let Err(e) = diesel::insert_into(legislative_initiatives::table)
            .values(&db_legis_init)
            .on_conflict(legislative_initiatives::id)
            .do_nothing()
            .execute(conn)
        {
            log::error!("Error inserting legislative initiative: '{e}'. Error occured for legislative initiative: {legis_init:?}");
            continue;
        }*/


        /*for speech in &legis_init.speeches {
            
            if let Err(e) = diesel::insert_into(speeches::table)
                .values(&db_speech)
                .on_conflict((speeches::legislative_initiatives_id, speeches::delegate_id))
                .do_nothing()
                .execute(conn)
            {
                log::warn!("Error inserting speech, probably delegate does not exist in database and has got inactive recently: {e}, for speech: {speech:?}")
            }
        }*/
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
                    log::info!("Starting plenar info scraper");
                    plenar_info_once().await;
                }).await {
                    log::error!("Error in plenar info scraper: {:?}", e);
                }
                // legis_init_once().await;
                tokio::time::sleep(tokio::time::Duration::from_secs(400)).await;
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
