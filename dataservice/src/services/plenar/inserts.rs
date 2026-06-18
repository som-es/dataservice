use std::collections::HashSet;

use chrono::{DateTime, Utc};
use log::warn;
use somes_scraper::{
    call_to_order::CallToOrderInfo,
    plenar::{Debate, DebateInfo, PlenarInfo},
    text_utils, PlenarSpeech, Reference, TextExtractor,
};

use sqlx::{postgres::PgQueryResult, Executor, Postgres};

use crate::{plenar::extract_interjection::extract_and_save_interjections, ApiLimiter};

pub async fn insert_plenar_speech_legis_init<'a>(
    pool: impl Executor<'a, Database = Postgres>,
    legis_init_id: Option<i32>,
    plenar_speech_id: i32,
    reference: &Reference,
) -> sqlx::Result<PgQueryResult> {
    sqlx::query!("
            insert into
                plenar_speech_legis_inits (gp, ityp, inr, speech_id, legis_init_id)
            values ($1, $2, $3, $4, $5)
            on conflict (gp, ityp, inr, speech_id) do update set legis_init_id = EXCLUDED.legis_init_id", reference.gp, reference.ityp, reference.inr as i32, plenar_speech_id, legis_init_id).execute(pool).await
}

pub async fn insert_debate<'a>(
    pool: impl Executor<'a, Database = Postgres>,
    debate: &Debate,
    plenar_id: i32,
) -> Result<i32, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        INSERT INTO debates (plenar_id, about)
        VALUES ($1, $2)
        ON CONFLICT (plenar_id, about)
        DO UPDATE SET about = EXCLUDED.about
        RETURNING id
        "#,
        plenar_id,
        debate.info.about
    )
    .fetch_one(pool)
    .await?;

    Ok(result.id)
}

pub async fn insert_plenar_info<'a>(
    pool: impl Executor<'a, Database = Postgres>,
    plenar_info: &PlenarInfo,
) -> Result<i32, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        INSERT INTO plenar_infos (inr, title, description, raw_data_created_at, legislative_period, raw_data_updated_at, absences_doc_url, council)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT (inr, legislative_period)
        DO UPDATE SET
            title = EXCLUDED.title,
            description = EXCLUDED.description,
            raw_data_updated_at = EXCLUDED.raw_data_updated_at,
            raw_data_created_at = EXCLUDED.raw_data_created_at,
            updated_at = now(),
            absences_doc_url = EXCLUDED.absences_doc_url
        RETURNING id
        "#,
        plenar_info.inr as i32,
        plenar_info.title,
        plenar_info.description,
        plenar_info.created_at,
        plenar_info.gp,
        plenar_info.updated_at,
        plenar_info.absences.as_ref().map(|absences| &absences.source_document.link),
        plenar_info.council,
    )
    .fetch_one(pool)
    .await?;

    Ok(result.id)
}

pub async fn insert_call_to_order<'a>(
    pool: impl Executor<'a, Database = Postgres>,
    call_to_order: &CallToOrderInfo,
    plenar_id: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO call_to_order (id, plenar_id, inr, receiver_id)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (id) DO NOTHING
        "#,
        format!("{}/{}", call_to_order.ityp, call_to_order.inr),
        plenar_id,
        call_to_order.inr as i32,
        call_to_order.receiver_id as i32
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub struct DbReference {
    origin_legis_init_id: i32,
    ref_gp: String,
    ref_ityp: String,
    ref_inr: i32,
}

pub async fn insert_plenar_speech<'a>(
    pool: &mut sqlx::Transaction<'a, Postgres>,
    plenar_speech: &PlenarSpeech,
    corresponding_debate: &DebateInfo,
    val_debate_id: i32,
    text_extractor: &mut TextExtractor,
    api_limiter: &ApiLimiter,
    plenar_date: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    log::info!("insert plenar speech");
    let disable_speech_text =
        std::env::var("DISABLE_SPEECH_TEXT").unwrap_or_else(|_| "true".into()) == "true";

    let opinion = match &plenar_speech.speech.opinion {
        somes_scraper::Opinion::Infavor => Some("Pro".to_string()),
        somes_scraper::Opinion::Against => Some("Contra".to_string()),
        somes_scraper::Opinion::Other(text) => Some(text.clone()),
        somes_scraper::Opinion::None => None,
    };

    // check if delegate exists
    let delegate_exists = sqlx::query!(
        "select id from delegates where id = $1",
        plenar_speech.speech.delegate_id as i32
    )
    .fetch_optional(&mut **pool)
    .await?;

    if delegate_exists.is_none() {
        log::error!(
            "Delegate with id {} does not exist in database. Skipping plenar speech insertion.",
            plenar_speech.speech.delegate_id
        );
        return Ok(());
    }

    let plenar_speech_id = sqlx::query!(
        r#"
        INSERT INTO plenar_speeches (delegate_id, debate_id, opinion, spoken_text, duration_in_seconds, ty, delegate_display_name)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        ON CONFLICT (debate_id, delegate_id, duration_in_seconds, ty)
        DO UPDATE SET
            opinion = EXCLUDED.opinion,
            duration_in_seconds = EXCLUDED.duration_in_seconds,
            ty = EXCLUDED.ty
        RETURNING id
        "#,
        plenar_speech.speech.delegate_id as i32,
        val_debate_id,
        opinion,
        None::<String>, // spoken_text is initially None
        plenar_speech.duration_in_seconds.map(|duration| duration as i32),
        plenar_speech.ty,
        plenar_speech.speech.delegate_display_name,
    )
    .fetch_one(&mut **pool)
    .await?
    .id;

    for reference in &corresponding_debate.references {
        let legis_init_id = if let Ok(x) = sqlx::query!(
            "select id from legislative_initiatives where gp = $1 and ityp = $2 and inr = $3",
            reference.gp,
            reference.ityp,
            reference.inr as i32
        )
        .fetch_one(&mut **pool)
        .await
        {
            Some(x.id)
        } else {
            None
        };

        // let own_references = i
        if let Err(e) =
            insert_plenar_speech_legis_init(&mut **pool, legis_init_id, plenar_speech_id, reference)
                .await
        {
            log::error!("could not insert plenar speech legis init for legis int {legis_init_id:?} and speech {plenar_speech_id}: {e:?}")
        }
        if let Some(legis_init_id) = legis_init_id {
            let own_references = sqlx::query_as!(DbReference, "select origin_legis_init_id, ref_gp, ref_ityp, ref_inr from legis_inits_refs where origin_legis_init_id = $1", legis_init_id).fetch_all(&mut **pool).await.unwrap_or_default();

            for reference in own_references {
                let legis_init_id = if let Ok(x) = sqlx::query!(
                    "select id from legislative_initiatives where gp = $1 and ityp = $2 and inr = $3",
                    reference.ref_gp,
                    reference.ref_ityp,
                    reference.ref_inr
                )
                .fetch_one(&mut **pool)
                .await
                {
                    Some(x.id)
                } else {
                    None
                };

                if let Err(e) = insert_plenar_speech_legis_init(
                    &mut **pool,
                    legis_init_id,
                    plenar_speech_id,
                    &Reference {
                        gp: reference.ref_gp,
                        ityp: reference.ref_ityp,
                        inr: reference.ref_inr as u32,
                    },
                )
                .await
                {
                    log::error!("could not insert plenar speech legis init for legis int {legis_init_id:?} and speech {plenar_speech_id}: {e:?}")
                }
            }
        }
    }
    let mut newly_inserted = HashSet::new();
    // log::info!(
    //     "delegate_id: {} about: {}",
    //     plenar_speech.speech.delegate_id,
    //     corresponding_debate.about
    // );

    // this is important for speeches because of https://github.com/som-es/somes/issues/85
    if let Err(e) = sqlx::query!(
        "delete from plenar_speech_links where plenar_speech_id = $1",
        plenar_speech_id
    )
    .execute(&mut **pool)
    .await
    {
        log::error!("could not delete plenar speech links for speech {plenar_speech_id}: {e:?}")
    }

    for url in &plenar_speech.speech.spoken_text_html_urls {
        let id = sqlx::query_scalar!(
            r#"
            INSERT INTO plenar_speech_links (plenar_speech_id, document_url, site_start, site_end)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (plenar_speech_id, document_url) DO UPDATE SET
                site_start = EXCLUDED.site_start,
                site_end = EXCLUDED.site_end

            RETURNING id
            "#,
            plenar_speech_id,
            url.document_url.to_string(),
            url.site_start.map(|i| i as i32),
            url.site_end.map(|i| i as i32)
        )
        .fetch_optional(&mut **pool)
        .await;
        match id {
            Ok(id) => {
                // this will also return something if an conflict happens (do update set)
                if let Some(id) = id {
                    // log::info!("newly_inserted: {url:?}");
                    newly_inserted.insert(id);
                }
            }
            Err(e) => {
                log::error!("could not insert plenar speech link for speech {plenar_speech_id} and url {url:?}: {e:?}");
            }
        }
    }
    // let existing_spoken_text = sqlx::query!(
    //     r#"
    //     SELECT spoken_text FROM plenar_speeches
    //     WHERE debate_id = $1 AND delegate_id = $2
    //     "#,
    //     val_debate_id,
    //     plenar_speech.speech.delegate_id as i32
    // )
    // .fetch_optional(pool)
    // .await?;

    // if let Some(record) = existing_spoken_text {
    // if record.spoken_text.is_none() {
    if let Some(text_html_url) = plenar_speech.speech.spoken_text_html_urls.first() {
        let updated_spoken_text = if disable_speech_text {
            None
        } else {
            api_limiter
                .run(text_extractor.speech_extraction_with_fit(
                    &text_html_url,
                    &plenar_speech.speech.delegate_display_name,
                ))
                .await
        };
        if updated_spoken_text.is_none() {
            warn!(
                        "DEBATE_ID: {}, Missing spoken text for plenar speech id: {plenar_speech_id}, and plenar info: {:?}",
                        val_debate_id, plenar_speech
                    );
        }
        if let Some(updated_spoken_text) = &updated_spoken_text {
            let (removed_parans, _) =
                text_utils::remove_paran_contents(updated_spoken_text, '(', ')');
            log::info!("removed parans: {removed_parans:?}");
            if let Err(e) = extract_and_save_interjections(
                removed_parans,
                plenar_speech_id as i64,
                plenar_speech.speech.delegate_id as i64,
                plenar_date,
                pool,
            )
            .await
            {
                log::error!(
                    "Could not extract and save interjections for {plenar_speech_id}: {e:?}"
                )
            }
        }

        sqlx::query!(
            r#"
                    UPDATE plenar_speeches
                    SET spoken_text = $1
                    WHERE id = $2
                    "#,
            updated_spoken_text,
            plenar_speech_id,
        )
        .execute(&mut **pool)
        .await?;
    } else {
        warn!(
            "DEBATE_ID: {}, No spoken text html urls for plenar speech: {:?}",
            val_debate_id, plenar_speech
        );
    }
    // }
    // }

    Ok(())
}
