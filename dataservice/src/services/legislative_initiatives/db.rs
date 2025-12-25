use combx::{PostgresEnhancer, VoteResult};
use somes_scraper::LegislativeInitiative;
use sqlx::PgPool;
use tokio::sync::mpsc::Sender;

use crate::{
    ai_task::AiTask,
    legislative_initiatives::inserts::{
        insert_delegates_batch, insert_documents_batch, insert_eurovoc_topics_batch,
        insert_named_vote_info, insert_named_votes, insert_other_keyword_topics_batch,
        insert_references_batch, insert_topics_batch, insert_votes_batch,
    },
    today,
};

pub async fn update_or_insert_legis_init_in_db(
    pool: &PgPool,
    legis_init: &LegislativeInitiative,
    new_legis_init: Option<Sender<Option<(i32, LegislativeInitiative)>>>,
    ai_task_tx: Option<Sender<AiTask>>,
) {
    let start = tokio::time::Instant::now();
    if !crate::should_update(legis_init, pool).await {
        log::info!("No need to update!");
        return;
    }
    log::info!(
        "ADD legislative initiative: {} {} {}",
        legis_init.gp,
        legis_init.ityp,
        legis_init.inr
    );

    let vote_result_start = tokio::time::Instant::now();
    let mut vote_result =
        VoteResult::from_legis_init(PostgresEnhancer(pool.clone()), legis_init.clone(), None).await;

    log::info!(
        "legis init to vote result conversion duration: {:?}",
        vote_result_start.elapsed()
    );

    let db_legis_init = vote_result.legislative_initiative;

    let mut tx = match pool.begin().await {
        Ok(t) => t,
        Err(e) => {
            log::error!("Failed to start transaction for legis init: {:?}", e);
            return;
        }
    };

    let legis_init_id = match sqlx::query!(
            "
                INSERT INTO legislative_initiatives (
                    title, inr, emphasis, description, ityp, gp, accepted, created_at, appeared_at, updated_at,
                    requires_simple_majority, pre_declined_type, voted_by_name, plenary_session_id, doktyp, is_law,
                    law_accepted, has_reference, is_voteable_on,
                    is_urgent, voting
                ) VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21
                ) ON CONFLICT (inr, ityp, gp)
                DO UPDATE SET
                    title = EXCLUDED.title,
                    emphasis = COALESCE(EXCLUDED.emphasis, legislative_initiatives.emphasis),
                    description = EXCLUDED.description,
                    accepted = EXCLUDED.accepted,
                    created_at = EXCLUDED.created_at,
                    appeared_at = EXCLUDED.appeared_at,
                    updated_at = EXCLUDED.updated_at,
                    requires_simple_majority = EXCLUDED.requires_simple_majority,
                    pre_declined_type = EXCLUDED.pre_declined_type,
                    voted_by_name = EXCLUDED.voted_by_name,
                    plenary_session_id = EXCLUDED.plenary_session_id,
                    doktyp = EXCLUDED.doktyp,
                    is_law = EXCLUDED.is_law,
                    law_accepted = EXCLUDED.law_accepted,
                    has_reference = EXCLUDED.has_reference,
                    is_voteable_on = EXCLUDED.is_voteable_on,
                    is_urgent = EXCLUDED.is_urgent,
                    voting = EXCLUDED.voting
                RETURNING id
            ",
            db_legis_init.title,
            db_legis_init.inr,
            db_legis_init.emphasis,
            db_legis_init.description,
            db_legis_init.ityp,
            db_legis_init.gp,
            db_legis_init.accepted,
            db_legis_init.created_at,
            db_legis_init.appeared_at,
            db_legis_init.updated_at,
            db_legis_init.requires_simple_majority,
            db_legis_init.pre_declined_type,
            db_legis_init.voted_by_name,
            db_legis_init.plenary_session_id,
            db_legis_init.doktyp,
            db_legis_init.is_law,
            db_legis_init.law_accepted,
            db_legis_init.has_reference,
            db_legis_init.is_voteable_on,
            db_legis_init.is_urgent,
            db_legis_init.voting.as_ref().map(|voting| voting.to_string()),
        )
        .fetch_one(&mut *tx)
        .await
        {
            Ok(record) => record.id,
            Err(e) => {
                log::error!("Error inserting legislative initiative: '{:?}'. Error occurred for legislative initiative: {:?}", e, legis_init);
                return;
            }
        };

    vote_result.id = Some(legis_init_id);

    #[cfg(feature = "ai")]
    if let Some(ai_task_tx) = &ai_task_tx {
        if db_legis_init.is_voteable_on {
            use crate::{ai_task::Priority, LATEST_GP};

            let priority =
                if db_legis_init.appeared_at.date() >= today() - chrono::Duration::days(60) {
                    Priority::Highest
                } else if db_legis_init.gp == LATEST_GP {
                    Priority::High
                } else {
                    Priority::Lowest
                };

            let documents = vote_result.documents.clone();
            let voting = db_legis_init.voting.clone();
            let summarization_task = AiTask {
                input: None,
                priority,
                task_id: crate::hash!(),
                db_entry_id: legis_init_id,
                task: Box::new(move |task_args| {
                    Box::pin(async move {
                        use crate::legislative_initiatives::summarize_legis_init_documents;

                        let summarization_of = match voting.as_deref() {
                            Some("Law") => "Antrag/Gesetz",
                            Some("Amendment") => "Abänderungsantrag",
                            Some("Resolution") => "Entschließungsantrag",
                            Some(other) => other,
                            None => "Unbekannt",
                        };
                        summarize_legis_init_documents(task_args, &documents, summarization_of)
                            .await
                        // TODO: update meilisearch index on finish
                    })
                }),
            };

            if let Err(e) = ai_task_tx.send(summarization_task).await {
                log::error!("Could not send AI task for legis init emphasis summarization: {e:?}");
            }
        }
    }

    if let Some(refs) = vote_result.references_to.as_ref() {
        if let Err(e) = insert_references_batch(&mut tx, refs, legis_init_id).await {
            log::error!("Could not insert references for legis init {legis_init_id}: {e:?}")
        }
    }

    let show_votes = legis_init.votes.clone();

    if legis_init.votes.is_empty() {
        if let Err(e) = sqlx::query!(
            "delete from votes where legislative_initiatives_id = $1",
            legis_init_id
        )
        .execute(&mut *tx)
        .await
        {
            log::error!("Cannot delete votes from legis init: {legis_init_id} with error {e:?}");
        }
    }

    if let Err(e) = insert_votes_batch(&mut tx, &vote_result.votes, legis_init_id).await {
        log::warn!("Error inserting votes: '{:?}'.", e);
    }

    if let Some(named_votes) = vote_result.named_votes {
        if let Ok(named_vote_info_id) =
            insert_named_vote_info(&mut *tx, &named_votes.named_vote_info, legis_init_id).await
        {
            if let Err(e) = insert_named_votes(
                &mut tx,
                &named_votes.named_votes.as_ref().unwrap_or(&vec![]),
                named_vote_info_id,
            )
            .await
            {
                log::error!(
                    "Could not insert any named votes for legis init: {legis_init:?}, error: {e:?}"
                )
            }
        }
    }

    if let Some(new_legis_init_tx) = &new_legis_init {
        if let Ok(info) = sqlx::query!(
            "select * from legis_email_info where already_sent_id = $1",
            legis_init_id
        )
        .fetch_optional(&mut *tx)
        .await
        {
            let diff = today() - legis_init.date;

            let mut legis_init = legis_init.clone();
            legis_init.votes = show_votes;
            if diff.num_days() <= *crate::legislative_initiatives::LEGIS_INIT_STILL_SEND_DAYS
                && info.is_none()
                && !legis_init.votes.is_empty()
            {
                log::info!("Sending fitting legis init: {legis_init_id}");
                if let Err(e) = new_legis_init_tx
                    .send(Some((legis_init_id, legis_init)))
                    .await
                {
                    log::error!("could not send legis init to mail notify service: {e:?}")
                }

                if let Err(e) = sqlx::query!(
                    "insert into legis_email_info (already_sent_id) values ($1)",
                    legis_init_id
                )
                .execute(&mut *tx)
                .await
                {
                    log::error!(
                        "could not insert into legis email info for id {legis_init_id}: {e:?}"
                    );
                }
            }
        };
    }

    if let Err(e) = insert_topics_batch(&mut tx, &vote_result.topics, legis_init_id).await {
        log::error!("Error inserting topics for legis init {legis_init_id}: {e:?}");
    }

    if let Err(e) =
        insert_eurovoc_topics_batch(&mut tx, &vote_result.eurovoc_topics, legis_init_id).await
    {
        log::error!("Error inserting eurovoc topics for legis init {legis_init_id}: {e:?}");
    }

    if let Err(e) =
        insert_delegates_batch(&mut tx, &legis_init.related_delegates, legis_init_id).await
    {
        log::error!("Error inserting delegates for legis init {legis_init_id}: {e:?}");
    }

    if let Err(e) =
        insert_other_keyword_topics_batch(&mut tx, &vote_result.other_keyword_topics, legis_init_id)
            .await
    {
        log::error!("Error inserting other keyword topics for legis init {legis_init_id}: {e:?}");
    }

    if let Err(e) = insert_documents_batch(&mut tx, &vote_result.documents, legis_init_id).await {
        log::error!("Error inserting documents for legis init {legis_init_id}: {e:?}");
    }

    if let Err(e) = sqlx::query!(
        "insert into
            legis_init_was_updated
            (legis_init_id)
            values ($1) on conflict(legis_init_id) do update set
            updated_at = $2",
        legis_init_id,
        crate::today_and_time().and_utc()
    )
    .execute(&mut *tx)
    .await
    {
        log::error!("Error inserting was updated info: '{:?}'. The error occurred for the following legis init: {:?}", e, legis_init_id);
    }

    if let Err(e) = tx.commit().await {
        log::error!("Cannot commit legis init transaction: {e:?}");
    }
    log::info!("Insertion duration for legis init: {:?}", start.elapsed());
}
