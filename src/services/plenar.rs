use crate::{
    models::{DbCallToOrder, DbDebate, DbPlenar},
    schema::{call_to_order, debates, plenar_infos},
    services::scrape_untill_success,
    Shutdown,
};
use diesel::RunQueryDsl;
use scraper::{
    call_to_order::CallToOrderInfo,
    legis_init::LegislativeInitiatives,
    plenar::{Debate, Plenar, PlenarInfo},
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
        let plenar_id = format!("{}/{}", plenar_info.ityp, plenar_info.inr);
        insert_plenar_info(plenar_info, plenar_id.clone(), &mut conn);

        for call_to_order_entry in &plenar_info.call_to_orders {
            insert_call_to_order(call_to_order_entry, plenar_id.clone(), &mut conn);
        }

        for debate in &plenar_info.debates {
            // get id from here, log here
            insert_debate(debate, plenar_id.clone(), &mut conn);

            for plenar_speech in &debate.speeches {
                insert_plenar_speech(plenar_speech, debate_id, &mut conn);
            }
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

fn insert_debate(debate: &Debate, plenar_id: String, conn: &mut diesel::PgConnection) {
    let db_debate = DbDebate {
        plenar_id,
        about: debate.about.clone(),
    };
    if let Err(e) = diesel::insert_into(debates::table)
        .values(&db_debate)
        .on_conflict(debates::plenar_id)
        .do_update()
        .set(&db_debate)
        .execute(conn)
    {
        log::error!("Error inserting debate: '{e}'. Error occured for debate: {debate:?}");
    }
}

pub fn insert_plenar_info(
    plenar_info: &PlenarInfo,
    plenar_id: String,
    conn: &mut diesel::PgConnection,
) {
    let db_plenar_info = DbPlenar {
        id: plenar_id,
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
        .execute(conn)
    {
        log::error!(
            "Error inserting plenar info: '{e}'. Error occured for plenar info: {plenar_info:?}"
        );
    }
}

pub fn insert_call_to_order(
    call_to_order: &CallToOrderInfo,
    plenar_id: String,
    conn: &mut diesel::PgConnection,
) {
    let db_call_to_order = DbCallToOrder {
        id: format!("{}/{}", call_to_order.ityp, call_to_order.inr),
        plenar_id,
        inr: call_to_order.inr as i32,
        receiver_id: call_to_order.receiver_id.clone() as i32,
    };
    if let Err(e) = diesel::insert_into(call_to_order::table)
        .values(&db_call_to_order)
        .on_conflict(call_to_order::id)
        .do_update()
        .set(&db_call_to_order)
        .execute(conn)
    {
        log::error!("Error inserting call to order: '{e}'. Error occured for call to order: {call_to_order:?}");
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
