use std::collections::HashSet;

use diesel::RunQueryDsl;
use scraper::{delegates::DelegateScraper, AustriaScraper, Delegate};

use crate::{
    models::{DbContact, DbDelegate},
    schema::contacts,
    services::scrape_untill_success,
    Shutdown,
};

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
    let mut conn = crate::establish_connection();

    log::info!("Updating delegates in database...");
    for delegate in new_delegates {
        let db_contact = DbContact {
            mail: delegate.contact.mail.clone(),
            phone_number: delegate.contact.phone_number.clone(),
            twitter_url: delegate.contact.social_media.twitter.clone(),
            facebook_url: delegate.contact.social_media.facebook.clone(),
            website_url: delegate.contact.social_media.website.clone(),
            tiktok_url: delegate.contact.social_media.tiktok.clone(),
            instagram_url: delegate.contact.social_media.instagram.clone(),
            youtube_url: delegate.contact.social_media.youtube.clone(),
        };

        let contacts_id = match diesel::insert_into(contacts::table)
            .values(&db_contact)
            .returning(contacts::id)
            .get_result::<i32>(&mut conn)
        {
            Ok(contacts_id) => Some(contacts_id),
            Err(e) => {
                log::error!("Error inserting contact information for delegate {delegate:?}: {e}");
                None
            }
        };

        let db_delegate = DbDelegate {
            id: delegate.id as i32,
            name: delegate.name.clone(),
            party: delegate.party.clone(),
            image_url: delegate.image_url.clone(),
            constituency: delegate.constituency.clone(),
            council: delegate.council.clone(),
            seat_row: delegate.seat.as_ref().map(|seat| seat.row as i32),
            seat_col: delegate.seat.as_ref().map(|seat| seat.col as i32),
            contacts_id,
        };

        if let Err(e) = diesel::insert_into(delegates::table)
            .values(&db_delegate)
            .on_conflict(delegates::id)
            .do_update()
            .set(&db_delegate)
            .execute(&mut conn)
        {
            log::error!(
                "Error adding/updating delegate in database: {e}, for delegate {delegate:?}"
            );
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
