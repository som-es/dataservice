use crate::{models::{DbPlenar, DbCallToOrder}, schema::plenar_infos, services::scrape_untill_success, Shutdown};
use diesel::RunQueryDsl;
use scraper::{
    legis_init::LegislativeInitiatives,
    plenar::{Plenar, PlenarInfo},
    AustriaScraper,
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

    let new_plenar_infos =
        scrape_untill_success::<Vec<PlenarInfo>, _>(|| async { scrape_new_plenar_infos().await })
            .await;
    update_plenar_infos_in_db(&new_plenar_infos);

    new_plenar_infos
}

fn update_plenar_infos_in_db(new_plenar_infos: &[PlenarInfo]) {
    log::info!("Updating legislative initiative in database...");

    let mut conn = crate::establish_connection();
    for plenar_info in new_plenar_infos {
        let db_plenar_info = DbPlenar {
            id: format!("{}/{}", plenar_info.ityp, plenar_info.inr),
            inr: plenar_info.inr as i32,
            title: plenar_info.title.clone(),
            description: plenar_info.description.clone(),
            add_date: plenar_info.date.clone(),
        };

        if let Err(e) = diesel::insert_into(plenar_infos::table)
            .values(&db_plenar_info)
            .on_conflict(plenar_infos::id)
            .do_update()
            .set(&db_plenar_info)
            .execute(&mut conn)
        {
            log::error!("Error inserting plenar info: '{e}'. Error occured for plenar info: {plenar_info:?}");
            continue;
        }

        for call_to_order_entry in &plenar_info.call_to_orders {
            let db_call_to_order = DbCallToOrder {
                id: todo!(),
                plenar_id: todo!(),
                inr: todo!(),
                receiver_id: todo!(),
            };
        }

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
