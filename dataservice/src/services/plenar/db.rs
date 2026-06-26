use combx::absence_matching;
use somes_scraper::plenar::PlenarInfo;
use sqlx::PgPool;

use crate::{
    plenar::{
        insert_absences, insert_call_to_orders, insert_debate,
        insert_plenar_info, insert_plenar_speech,
    },
    ApiLimiter,
};

pub async fn update_plenar_infos_in_db(
    plenar_info: &PlenarInfo,
    pg_pool: &PgPool,
    api_limiter: ApiLimiter,
) {
    log::info!("Updating plenar infos in database...");

    // disable with setting
    let mut text_extractor = somes_scraper::TextExtractor::default();

    if !crate::should_update_by_time(plenar_info, pg_pool).await {
        return;
    }

    let pg_pool = pg_pool;

    let plenar_id = match insert_plenar_info(pg_pool, plenar_info).await {
        Ok(id) => id,
        Err(e) => {
            log::error!(
                    "Error inserting plenar info: '{e}'. Error occured for plenar info: {plenar_info:?}. Skipping adding call to orders and debates."
                );
            return;
        }
    };

    if let Err(e) = insert_call_to_orders(pg_pool, plenar_id, plenar_info).await {
        log::error!(
            "could not insert call to orders for plenary session {plenar_id} because of {e:?}"
        );
    }

    log::info!("inserting debates");
    for debate in &plenar_info.debates {
        // get id from here, log here

        let debate_id = match insert_debate(pg_pool, debate, plenar_id).await {
            Ok(debate_id) => debate_id,
            Err(e) => {
                log::error!("Error inserting debate: '{e}'. Error occured for debate: {debate:?}");
                continue;
            }
        };

        let mut tx = pg_pool.begin().await.unwrap();

        // SELECT ps.id, ty, duration_in_seconds, about, add_date, legislative_period, inr, debates.id as debate_id, pi.id as plenar_id FROM plenar_speeches ps inner join debates on debates.id = debate_id inner join plenar_infos pi on pi.id = plenar_id where spoken_text is not null and delegate_id = 2326 and legislative_period = 'XXVI' and inr = 43;
        // Add on delete cascade
        /*if let Err(e) = query!(
            "delete from plenar_speeches where debate_id = $1",
            debate_id
        )
        .execute(&mut *pg_pool)
        .await
        {
            log::error!(
                "Cannot delete from plenar speeches for debate id: {debate_id}, e: {e:?}"
            )
        }*/

        for plenar_speech in &debate.speeches {
            if let Err(e) = insert_plenar_speech(
                &mut tx,
                plenar_speech,
                &debate.info,
                debate_id,
                &mut text_extractor,
                &api_limiter,
                plenar_info.created_at,
                &plenar_info.council,
            )
            .await
            {
                log::error!(
                        "Error inserting plenar speech: '{e}'. Error occured for plenar speech: {plenar_speech:?}"
                    );
            }
        }

        if let Err(e) = tx.commit().await {
            log::error!("Cannot commit transaction: {e:}");
        }
    }

    if let Some(absences) = &plenar_info.absences {
        match absence_matching(
            pg_pool,
            &absences.absences,
            plenar_info.created_at.date_naive(),
            plenar_info.gp.clone(),
            &plenar_info.council,
        )
        .await
        {
            Ok(matching) => {
                log::info!("inserting absences: {matching:?}");
                if let Err(e) = insert_absences(pg_pool, plenar_id, matching).await {
                    log::error!("could not insert absences for plenary session {plenar_id} because of {e:?}");
                }
            }
            Err(e) => {
                log::error!("could not match absences: {e:?}");
            }
        }
    }
}
