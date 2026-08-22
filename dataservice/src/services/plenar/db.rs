use combx::{Parliament, absence_matching};
use somes_scraper::plenar::PlenarInfo;
use sqlx::PgPool;
use tokio::sync::mpsc::Sender;

use crate::{
    ai_task::AiTask,
    plenar::{
        SpeechInsertArgs, insert_absences, insert_call_to_orders, insert_debate,
        insert_plenar_info, insert_plenar_speech,
    },
};

pub async fn update_plenar_infos_in_db(
    plenar_info: &PlenarInfo,
    pg_pool: &PgPool,
    ai_task_tx: Option<&Sender<AiTask>>,
    parliament: Parliament,
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
                log::error!(
                    "Error inserting debate: '{e}'. Error occured for debate: {:?}",
                    debate.info
                );
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

        let speech_insert_args = SpeechInsertArgs {
            corresponding_debate: &debate.info,
            debate_id,
            plenar_date: plenar_info.created_at,
            council: &plenar_info.council,
            ai_task_tx,
        };

        for plenar_speech in &debate.speeches {
            if let Err(e) = insert_plenar_speech(
                pg_pool.clone(),
                &mut tx,
                plenar_speech,
                &mut text_extractor,
                &speech_insert_args,
                parliament,
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

    if let Err(e) = insert_all_absences(pg_pool, plenar_id, plenar_info, parliament).await {
        log::error!("cannot insert all absences: {e:?}");
    }
}

pub async fn insert_all_absences(
    pg_pool: &PgPool,
    plenar_id: i32,
    plenar_info: &PlenarInfo,
    parliament: Parliament,
) -> sqlx::Result<()> {
    let mut tx = pg_pool.begin().await?;

    sqlx::query!(
        "delete from absences where plenary_session_id = $1",
        plenar_id
    )
    .execute(&mut *tx)
    .await?;

    for absences in plenar_info.absences.iter() {
        let mandate_function = match parliament {
            Parliament::At => plenar_info.council.as_str(),
            Parliament::Eu => "MEP",
        };
        match absence_matching(
            pg_pool,
            &absences.absences,
            plenar_info.created_at.date_naive(),
            plenar_info.gp.clone(),
            mandate_function,
        )
        .await
        {
            Ok(matching) => {
                log::info!("inserting absences: {:?}", matching.correctly_matched);

                if let Err(e) = insert_absences(
                    &mut tx,
                    plenar_id,
                    matching,
                    absences.date,
                    &absences.source_document,
                )
                .await
                {
                    log::error!(
                        "could not insert absences for plenary session {plenar_id} because of {e:?}"
                    );
                }
            }
            Err(e) => {
                log::error!("could not match absences: {e:?}");
            }
        }
    }
    tx.commit().await?;
    Ok(())
}
