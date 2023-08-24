use std::collections::HashSet;

use diesel::{PgConnection, QueryResult, RunQueryDsl, QueryDsl};
use scraper::{delegates::DelegateScraper, AustriaScraper, Delegate, Mandate};

use crate::{
    models::{DbContact, DbDelegate, DbMandate},
    schema::{contacts, mandates, delegates},
    services::scrape_untill_success,
    Shutdown,
};

async fn scrape_new_delegates(prev_delegates: &HashSet<Delegate>) -> crate::Result<Vec<Delegate>> {
    let scraper = AustriaScraper::default();

    let mut delegates = vec![];

    let persons = scraper.post_persons_payload().await?;
    for person_path in scraper.person_paths(persons).unwrap_or_default() {
        // println!("person_path: {person_path}");
        let Ok(delegate) = scraper.get_delegate(&person_path).await else {
            continue;
        };

        // delegates that were active before but aren't anymore remain (as active) in the database
        /*if !delegate.is_active {
            continue;
        }*/

        if prev_delegates.contains(&delegate) {
            continue;
        }

        delegates.push(delegate);
    }

    Ok(delegates)
}

pub fn delegate_count(conn: &mut PgConnection) -> QueryResult<i64> {
    delegates::table.count().get_result(conn)
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
            gender: delegate.gender.to_string_opt(),
            is_active: Some(delegate.is_active),
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

        for mandate in &delegate.mandates {
            if let Err(e) = insert_mandate(mandate, delegate.id as i32, &mut conn) {
                log::error!("Error adding mandate ({mandate:?}) for delegate {delegate:?}: {e}",);
            }
        }
    }
}

pub fn insert_mandate(
    mandate: &Mandate,
    delegate_id: i32,
    conn: &mut PgConnection,
) -> QueryResult<usize> {
    let db_mandate = DbMandate {
        delegate_id,
        name: mandate.name.clone(),
        party: mandate.party.clone(),
        start_date: mandate.start,
        end_date: mandate.end,
    };

    diesel::insert_into(mandates::table)
        .values(&db_mandate)
        .on_conflict((
            mandates::delegate_id,
            mandates::name,
            mandates::start_date,
            mandates::party,
        ))
        .do_update()
        .set(&db_mandate)
        .execute(conn)
}

pub async fn delegates_once(prev_delegates: &HashSet<Delegate>) -> Vec<Delegate> {
    log::info!("Scraping all delegates...");

    let new_delegates = scrape_untill_success::<Vec<Delegate>, _>(|| async {
        scrape_new_delegates(&prev_delegates).await
    })
    .await;

    update_delegates_in_db(&new_delegates);

    new_delegates
}

pub async fn run_shut(mut shutdown: Shutdown) {
    let prev_delegates = HashSet::new();

    while !shutdown.is_shutdown() {
        tokio::select! {
            _ = shutdown.recv() => {
                // return ok?
                break;
            }
            _ = async {
                log::info!("Starting delegates scraper");

                delegates_once(&prev_delegates).await;
                log::info!("Delegates scraper service sleeps for 3 hours");
                tokio::time::sleep(tokio::time::Duration::from_secs(60 * 60 * 3)).await;
                // prev_delegates.extend(delegates_once(&prev_delegates).await);

                // println!("dele: {prev_delegates:?}");

            } => {}
        }
    }
}
