use std::{
    collections::{HashMap, HashSet},
    sync::LazyLock,
};

use combx::{create_gov_officials_map, DbContact, DbDelegate, DbMandate};
use somes_scraper::{
    delegates::DelegateScraper, gov_officials::GovOfficials, AustriaScraper, Delegate, GovOfficial,
    Mandate,
};
use sqlx::{postgres::PgQueryResult, query, PgPool};

use crate::{services::scrape_untill_success, Shutdown};

async fn scrape_new_delegates_bs() -> crate::Result<Vec<Delegate>> {
    let scraper = AustriaScraper::default();

    let mut delegates = vec![];

    let persons = scraper.post_persons_payload_bs().await?;
    for person_path in scraper.person_paths_bs(persons).unwrap_or_default() {
        let Ok(delegate) = scraper.get_delegate_bs(person_path).await else {
            continue;
        };
        delegates.push(delegate);
    }
    Ok(delegates)
}

async fn scrape_new_delegates(prev_delegates: &HashSet<Delegate>) -> crate::Result<Vec<Delegate>> {
    let scraper = AustriaScraper::default();

    let mut delegates = vec![];

    let persons = scraper.post_persons_payload().await?;
    for person_path in scraper.person_paths(persons).unwrap_or_default() {
        // println!("person_path: {person_path}");
        let Ok(delegate) = scraper.get_delegate(person_path).await else {
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

pub async fn delegate_count(conn: &PgPool) -> Option<i64> {
    query!("select COUNT(*) from delegates")
        .fetch_one(conn)
        .await
        .map(|x| x.count.unwrap())
        .ok()
}

async fn update_delegates_in_db(new_delegates: &[Delegate], pg_pool: &PgPool) {
    let gov_officials = create_gov_officials_map().await;

    log::info!("Updating delegates in database...");
    for delegate in new_delegates {
        let db_contact = DbContact {
            id: delegate.id as i32,
            mail: delegate.contact.mail.clone(),
            phone_number: delegate.contact.phone_number.clone(),
            twitter_url: delegate.contact.social_media.twitter.clone(),
            facebook_url: delegate.contact.social_media.facebook.clone(),
            website_url: delegate.contact.social_media.website.clone(),
            tiktok_url: delegate.contact.social_media.tiktok.clone(),
            instagram_url: delegate.contact.social_media.instagram.clone(),
            youtube_url: delegate.contact.social_media.youtube.clone(),
        };

        if let Err(e) = sqlx::query!(
            "
            insert into contacts values ($1, $2, $3, $4, $5, $6, $7, $8, $9) 
                on conflict (id)
                DO UPDATE SET 
                    mail = EXCLUDED.mail,
                    phone_number = EXCLUDED.phone_number,
                    twitter_url = EXCLUDED.twitter_url,
                    facebook_url = EXCLUDED.facebook_url,
                    website_url = EXCLUDED.website_url,
                    tiktok_url = EXCLUDED.tiktok_url,
                    instagram_url = EXCLUDED.instagram_url,
                    youtube_url = EXCLUDED.youtube_url
        ",
            db_contact.id,
            db_contact.mail,
            db_contact.phone_number,
            db_contact.twitter_url,
            db_contact.facebook_url,
            db_contact.website_url,
            db_contact.tiktok_url,
            db_contact.instagram_url,
            db_contact.youtube_url
        )
        .execute(pg_pool)
        .await
        {
            log::error!("Error inserting contact information for delegate {delegate:?}: {e}");
        }

        // if let Err(e) = diesel::insert_into(contacts::table)
        //     .values(&db_contact)
        //     .on_conflict(contacts::id)
        //     .do_update()
        //     .set(&db_contact)
        //     .execute(&mut conn)
        // // .returning(contacts::id)
        // // .get_result::<i32>(&mut conn)
        // {
        //     log::error!("Error inserting contact information for delegate {delegate:?}: {e}");
        // };

        let db_delegate = DbDelegate {
            id: delegate.id as i32,
            name: delegate.name.clone(),
            party: delegate.party.clone(),
            image_url: delegate.image_url.clone(),
            constituency: delegate.constituency.clone(),
            council: delegate.council.clone(),
            seat_row: delegate.seat.as_ref().map(|seat| seat.row as i32),
            seat_col: delegate.seat.as_ref().map(|seat| seat.col as i32),
            gender: delegate.gender.to_string_opt(),
            is_active: Some(delegate.is_active),
            birthdate: delegate.birthdate.clone(),
        };

        if let Err(e) = sqlx::query!(
            "
                INSERT INTO delegates (
                    id, 
                    name, 
                    party, 
                    image_url, 
                    constituency, 
                    council, 
                    seat_row, 
                    seat_col, 
                    gender, 
                    is_active, 
                    birthdate
                ) VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11
                ) ON CONFLICT (id) 
                DO UPDATE SET 
                    name = EXCLUDED.name,
                    party = EXCLUDED.party,
                    image_url = EXCLUDED.image_url,
                    constituency = EXCLUDED.constituency,
                    council = EXCLUDED.council,
                    seat_row = EXCLUDED.seat_row,
                    -- seat_row = 3,
                    seat_col = EXCLUDED.seat_col,
                    gender = EXCLUDED.gender,
                    is_active = EXCLUDED.is_active,
                    birthdate = EXCLUDED.birthdate
            ",
            db_delegate.id,
            db_delegate.name,
            db_delegate.party,
            db_delegate.image_url,
            db_delegate.constituency,
            db_delegate.council,
            db_delegate.seat_row,
            db_delegate.seat_col,
            db_delegate.gender,
            db_delegate.is_active,
            db_delegate.birthdate
        )
        .execute(pg_pool)
        .await
        {
            log::error!("Error inserting delegate information: {:?}", e);
        }

        // if let Err(e) = diesel::insert_into(delegates::table)
        //     .values(&db_delegate)
        //     .on_conflict(delegates::id)
        //     .do_update()
        //     .set(&db_delegate)
        //     .execute(&mut conn)
        // {
        //     log::error!(
        //         "Error adding/updating delegate in database: {e}, for delegate {delegate:?}"
        //     );
        // }

        // let debug_query = debug_query::<diesel::pg::Pg, _>(&insert_statement);

        // println!("SQL: {}", debug_query.to_string());

        if let Some(seat) = &delegate.seat {
            if let Err(e) = insert_seat(
                delegate.id as i32,
                seat.row as i32,
                seat.col as i32,
                delegate.council.as_ref().map(|x| x.as_str()),
                pg_pool,
            )
            .await
            {
                log::error!("error adding seat: {e:?}",);
            }
        }

        for mandate in &delegate.mandates {
            if let Err(e) =
                insert_mandate(mandate, delegate.id as i32, pg_pool, &gov_officials).await
            {
                log::error!("Error adding mandate ({mandate:?}) for delegate {delegate:?}: {e}",);
            }
        }

        // if let Err(e) = delete_all_delegate_divisions(delegate.id as i32, pg_pool).await {
        //     log::error!("could not delete  divisions for {delegate:?}: {e}");
        // }
        if let Some(divisions) = &delegate.divisions {
            for division in divisions {
                if let Err(e) =
                    insert_delegate_division(division, delegate.id as i32, pg_pool).await
                {
                    log::error!(
                        "Error adding division ({division:?}) for delegate {delegate:?}: {e}",
                    );
                }
            }
        }
    }
    // Ok(())
}

pub async fn delete_all_delegate_divisions(
    delegate_id: i32,
    pg: &PgPool,
) -> sqlx::Result<PgQueryResult> {
    sqlx::query!(
        "delete from delegates_divisions where delegate_id = $1",
        delegate_id
    )
    .execute(pg)
    .await
}

pub async fn insert_delegate_division(
    division: &str,
    delegate_id: i32,
    pg: &PgPool,
) -> sqlx::Result<PgQueryResult> {
    // mind that if a delegate looses this stuff it will still be active inside it
    sqlx::query!(
        "
        insert into delegates_divisions(delegate_id, division) values ($1, $2)
        ON CONFLICT (delegate_id, division) 
        do nothing
        ",
        delegate_id,
        division
    )
    .execute(pg)
    .await
}

pub async fn insert_seat(
    delegate_id: i32,
    seat_row: i32,
    seat_col: i32,
    council: Option<&str>,
    pg: &PgPool,
) -> sqlx::Result<PgQueryResult> {
    sqlx::query!(
        "
        insert into seat_history(delegate_id, seat_row, seat_col, council, gp) values ($1, $2, $3, $4, $5)
        on conflict (delegate_id, seat_row, seat_col, council, gp)
        do nothing
        ",
        delegate_id,
        seat_row,
        seat_col,
        council,
        crate::LATEST_GP,
    )
    .execute(pg)
    .await
}

pub async fn insert_mandate(
    mandate: &Mandate,
    delegate_id: i32,
    conn: &PgPool,
    gov_officials: &HashMap<u64, Vec<GovOfficial>>,
) -> sqlx::Result<PgQueryResult> {
    let name_lowercase = mandate.name.to_lowercase();

    let default = vec![];
    let gov_officials = gov_officials.get(&(delegate_id as u64));

    let is_fitting_ministry_official = gov_officials
        .as_ref()
        .unwrap_or(&&default)
        .iter()
        .find(|x| x.start_date == mandate.start && mandate.is_ministry);

    let ministry = is_fitting_ministry_official.map(|x| &x.ministry);

    let db_mandate = DbMandate {
        delegate_id,
        name: mandate.name.clone(),
        party: mandate.party.clone(),
        start_date: mandate.start,
        end_date: mandate.end,
        ministry: ministry.cloned(),
        is_nr: mandate.is_nr,
        is_gov_official: mandate.is_gov,
        is_chancellor: mandate.is_chancellor,
        is_ministry: mandate.is_ministry,
        function: mandate.function.clone(),
    };
    if name_lowercase.contains("abgeordnete") {
        if !mandate.is_nr {
            log::error!("Mandate name contains 'abgeordnete' but is isn't set to is_nr: {db_mandate:?}");
        }
    }
    if name_lowercase.contains("minister") || name_lowercase.contains("kanzler") {
        if !mandate.is_gov {
            log::warn!("Could be a minister or kanzler thing, but no GOV in mandate!!! mandate: {mandate:?}, delegate: {delegate_id}");
        }
    }

    sqlx::query!(
        "
            INSERT INTO mandates (
                delegate_id, 
                name, 
                party, 
                start_date, 
                end_date,
                is_nr,
                is_gov_official,
                is_chancellor,
                is_ministry,
                function,
                ministry
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11
            ) ON CONFLICT (delegate_id, name, start_date) 
            DO UPDATE SET 
                is_nr = EXCLUDED.is_nr,
                is_gov_official = EXCLUDED.is_gov_official,
                function = EXCLUDED.function,
                is_chancellor = EXCLUDED.is_chancellor,
                is_ministry = EXCLUDED.is_ministry,
                party = EXCLUDED.party,
                end_date = EXCLUDED.end_date,
                ministry = EXCLUDED.ministry
        ",
        db_mandate.delegate_id,
        db_mandate.name,
        db_mandate.party,
        db_mandate.start_date,
        db_mandate.end_date,
        db_mandate.is_nr,
        db_mandate.is_gov_official,
        db_mandate.is_chancellor,
        db_mandate.is_ministry,
        db_mandate.function,
        db_mandate.ministry
    )
    .execute(conn)
    .await

    // diesel::insert_into(mandates::table)
    //     .values(&db_mandate)
    //     .on_conflict((mandates::delegate_id, mandates::name, mandates::start_date))
    //     .do_update()
    //     .set(&db_mandate)
    //     .execute(conn)
}

pub async fn delegates_once(prev_delegates: &HashSet<Delegate>, pg_pool: PgPool) -> Vec<Delegate> {
    log::info!("Scraping all delegates...");

    let new_delegates_bs =
        scrape_untill_success::<Vec<Delegate>, _>(|| async { scrape_new_delegates_bs().await })
            .await;

    let mut new_delegates = scrape_untill_success::<Vec<Delegate>, _>(|| async {
        scrape_new_delegates(&prev_delegates).await
    })
    .await;

    for new_delegate in &mut new_delegates {
        for new_delegate_bs in &new_delegates_bs {
            if new_delegate_bs.id == new_delegate.id {
                new_delegate.divisions = new_delegate_bs.divisions.clone();
            }
        }
    }

    update_delegates_in_db(&new_delegates, &pg_pool).await;

    new_delegates
}

pub async fn run_shut(mut shutdown: Shutdown, pg_pool: PgPool) {
    let prev_delegates = HashSet::new();

    while !shutdown.is_shutdown() {
        tokio::select! {
            _ = shutdown.recv() => {
                // return ok?
                break;
            }
            _ = async {
                log::info!("Starting delegates scraper");

                delegates_once(&prev_delegates, pg_pool.clone()).await;
                log::info!("Delegates scraper service sleeps for 3 hours");
                tokio::time::sleep(tokio::time::Duration::from_secs(60 * 60 * 3)).await;
                // prev_delegates.extend(delegates_once(&prev_delegates).await);

                // println!("dele: {prev_delegates:?}");

            } => {}
        }
    }
}
