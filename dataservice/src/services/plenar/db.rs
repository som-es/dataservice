use combx::absence_matching;
use somes_scraper::{plenar::PlenarInfo, AustriaScraper, Scraper};
use sqlx::PgPool;

use crate::plenar::{
    insert_call_to_order, insert_debate, insert_plenar_info, insert_plenar_speech,
};

pub async fn update_plenar_infos_in_db(plenar_info: &PlenarInfo, pg_pool: &PgPool) {
    log::info!("Updating plenar infos in database...");

    // disable with setting
    let mut text_extractor = somes_scraper::TextExtractor::new(AustriaScraper::BASE_URL);

    if !crate::should_update(plenar_info, pg_pool).await {
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

    for call_to_order_entry in &plenar_info.call_to_orders {
        if let Err(e) = insert_call_to_order(pg_pool, call_to_order_entry, plenar_id).await {
            log::error!("Error inserting call to order: '{e}'. Error occured for call to order: {call_to_order_entry:?}");
        }
    }

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
            )
            .await
            {
                log::warn!(
                        "Error inserting plenar speech: '{e}'. Error occured for plenar speech: {plenar_speech:?}"
                    );
            }
        }

        if let Err(e) = tx.commit().await {
            log::error!("Cannot commit transaction: {e:}");
        }
    }

    if let Some(absences) = &plenar_info.absences {
        let matching = absence_matching(
            pg_pool,
            absences,
            plenar_info.date.date(),
            plenar_info.gp.clone(),
        )
        .await;
        for match_res in matching
            .correctly_matched
            .iter()
            .chain(matching.lev_or_deep_split_matched.iter())
        {
            if let Err(e) = sqlx::query!(
                    "
                    INSERT INTO absences (lev, similiarity_score, searched_with, matched_with, delegate_id, manually_matched, plenary_session_id)
                    VALUES ($1, $2, $3, $4, $5, $6, $7)
                    ON CONFLICT (delegate_id, plenary_session_id)
                    DO UPDATE SET
                        lev = EXCLUDED.lev,
                        similiarity_score = EXCLUDED.similiarity_score,
                        searched_with = EXCLUDED.searched_with,
                        matched_with = EXCLUDED.matched_with,
                        manually_matched = EXCLUDED.manually_matched
                    ", match_res.lev as i64, match_res.similiarity_score as i64, match_res.searched_with, match_res.matched_with, match_res.delegate_id as i32, match_res.manually_matched, plenar_id
                ).execute(pg_pool).await {
                    log::warn!("Error inserting absene {e:?} for absence {match_res:?}");
                }
        }
    }
}
