use std::collections::HashSet;

use diesel::RunQueryDsl;
use scraper::{delegates::DelegateScraper, AustriaScraper, Delegate};

use crate::{models::DbDelegate, services::scrape_untill_success, Shutdown};

async fn scrape_new_delegates(prev_delegates: &HashSet<Delegate>) -> crate::Result<Vec<Delegate>> {
    let scraper = AustriaScraper::default();

    let mut delegates = vec![];

    let persons = scraper.post_persons_payload().await?;
    for person_path in scraper.person_paths(persons).unwrap_or_default() {
        println!("person_path: {person_path}");
        let Ok(delegate) = scraper.get_delegate(&person_path).await else {
            continue;
        };

        // delegates that were active before but aren't anymore remain (as active) in the database
        if !delegate.is_active {
            continue;
        }

        if prev_delegates.contains(&delegate) {
            continue;
        }

        delegates.push(delegate);
    }

    Ok(delegates)
}

fn update_delegates_in_db(new_delegates: &[Delegate]) {
    use crate::schema::delegates;

    log::info!("Updating delegates in database...");
    for delegate in new_delegates {
        let db_delegate = DbDelegate {
            id: delegate.id as i32,
            name: delegate.name.clone(),
            party: delegate.party.clone(),
            image_url: delegate.image_url.clone(),
            constituency: delegate.constituency.clone(),
            council: delegate.council.clone(),
            seat_row: delegate.seat.as_ref().map(|seat| seat.row as i32),
            seat_col: delegate.seat.as_ref().map(|seat| seat.col as i32),
        };

        if let Err(e) = diesel::insert_into(delegates::table)
            .values(&db_delegate)
            .on_conflict(delegates::id)
            .do_update()
            .set(&db_delegate)
            .execute(&mut crate::establish_connection())
        {
            log::error!("Error adding/updating delegate in database: {e}, for delegate {delegate:?}");
        }
    }
}

pub async fn delegates_once(prev_delegates: &HashSet<Delegate>) -> Vec<Delegate> {
    log::info!("Scraping new delegates...");

    let new_delegates = scrape_untill_success::<Vec<Delegate>, _>(|| async {
        scrape_new_delegates(&prev_delegates).await
    })
    .await;

    update_delegates_in_db(&new_delegates);

    new_delegates
}

pub async fn run_shut(mut shutdown: Shutdown) {
    let mut prev_delegates = HashSet::new();

    while !shutdown.is_shutdown() {
        tokio::select! {
            _ = shutdown.recv() => {
                // return ok?
                break;
            }
            _ = async {
                prev_delegates.extend(delegates_once(&prev_delegates).await);

                // println!("dele: {prev_delegates:?}");

                tokio::time::sleep(tokio::time::Duration::from_secs(60 * 60 * 24)).await;

            } => {}
        }
    }
}
