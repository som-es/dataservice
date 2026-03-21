use combx::{Index, PostgresEnhancer, VoteResult};
use somes_scraper::LegislativeInitiative;
use sqlx::PgPool;
use tokio::sync::mpsc::Sender;

use crate::{
    ai_task::AiTask,
    enforce_ai_run,
    legislative_initiatives::inserts::{
        insert_delegates_batch, insert_documents_batch, insert_named_vote_info, insert_named_votes,
        insert_references_batch, insert_votes_batch,
    },
    notify_update_streamed, today,
    topics::{insert_topics_generic_batch, TableType, TopicType},
    AppState,
};

pub async fn update_or_insert_legis_init_in_db(
    app_state: &AppState,
    legis_init: &LegislativeInitiative,
    new_legis_init_tx: Option<Sender<(i32, VoteResult)>>,
    ai_task_tx: Option<Sender<AiTask>>,
) {
    let pool = &app_state.pg_pool;
    let start = tokio::time::Instant::now();

    let vote_result_start = tokio::time::Instant::now();
    let mut vote_result =
        VoteResult::from_legis_init(PostgresEnhancer(pool.clone()), legis_init.clone(), None).await;

    log::info!(
        "legis init to vote result conversion duration: {:?}",
        vote_result_start.elapsed()
    );

    let (should_update, hash) = crate::should_update_by_hash(&vote_result, pool).await;

    if !enforce_ai_run() && !should_update {
        log::info!("No need to update!");
        return;
    }
    log::info!(
        "ADD legislative initiative: {} {} {}",
        legis_init.gp,
        legis_init.ityp,
        legis_init.inr
    );
    let db_legis_init = vote_result.legislative_initiative.clone();

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
                    title, inr, emphasis, description, ityp, gp, accepted, nr_plenary_activity_date, raw_data_created_at, raw_data_updated_at,
                    requires_simple_majority, pre_declined_type, voted_by_name, plenary_session_id, doktyp, is_law,
                    law_accepted, has_reference, is_voteable_on,
                    is_urgent, voting, updated_at, vote_date, hash
                ) VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24
                ) ON CONFLICT (inr, ityp, gp)
                DO UPDATE SET
                    title = EXCLUDED.title,
                    emphasis = COALESCE(EXCLUDED.emphasis, legislative_initiatives.emphasis),
                    description = EXCLUDED.description,
                    accepted = EXCLUDED.accepted,
                    nr_plenary_activity_date = EXCLUDED.nr_plenary_activity_date,
                    raw_data_created_at = EXCLUDED.raw_data_created_at,
                    raw_data_updated_at = EXCLUDED.raw_data_updated_at,
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
                    voting = EXCLUDED.voting,
                    updated_at = EXCLUDED.updated_at,
                    vote_date = EXCLUDED.vote_date,
                    hash = EXCLUDED.hash
                RETURNING id
            ",
            db_legis_init.title,
            db_legis_init.inr,
            db_legis_init.emphasis,
            db_legis_init.description,
            db_legis_init.ityp,
            db_legis_init.gp,
            db_legis_init.accepted,
            db_legis_init.nr_plenary_activity_date,
            db_legis_init.raw_data_created_at,
            db_legis_init.raw_data_updated_at,
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
            crate::today_and_time(),
            db_legis_init.vote_date,
            hash as i64,
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
                if db_legis_init.raw_data_created_at >= today() - chrono::Duration::days(60) {
                    if db_legis_init.accepted.is_some() {
                        Priority::Highest
                    } else {
                        Priority::Higher
                    }
                } else if db_legis_init.gp == LATEST_GP {
                    Priority::High
                } else {
                    Priority::Lowest
                };

            let documents = vote_result.documents.clone();
            let voting = db_legis_init.voting.clone();
            let redis_client = app_state.redis_client.clone();
            let (gp, ityp, inr) = (
                vote_result.legislative_initiative.gp.clone(),
                vote_result.legislative_initiative.ityp.clone(),
                vote_result.legislative_initiative.inr,
            );
            let mut vote_result = vote_result.clone();
            let new_legis_init_tx = new_legis_init_tx.clone();
            let summarization_task = AiTask {
                input: None,
                priority,
                task_id: crate::hash!(),
                db_entry_id: legis_init_id,
                rerunnable: false,
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
                        let pg_pool = task_args.pg_pool.clone();
                        let ai_summary =
                            summarize_legis_init_documents(task_args, &documents, summarization_of)
                                .await?;

                        let has_ai_summary = ai_summary.is_some();
                        vote_result.ai_summary = ai_summary.clone();

                        check_and_send_legis_init_mail(
                            new_legis_init_tx,
                            &vote_result,
                            &pg_pool,
                            legis_init_id,
                        )
                        .await;

                        if !has_ai_summary {
                            return Ok(());
                        }
                        if let Some(mut client) = redis_client {
                            use combx::DbLegislativeInitiativeQuery;

                            // could use patching approach in the future
                            if let Err(e) = notify_update_streamed(
                                Index::VoteResults,
                                &VoteResult {
                                    id: Some(legis_init_id),
                                    ai_summary,
                                    legislative_initiative: DbLegislativeInitiativeQuery {
                                        id: legis_init_id,
                                        gp,
                                        ityp,
                                        inr,
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                },
                                &mut client,
                            )
                            .await
                            {
                                log::error!("Could not notify for vote result update!: {e:?}");
                            }
                        }
                        Ok(())
                    })
                }),
            };

            if let Err(e) = ai_task_tx.send(summarization_task).await {
                log::error!("Could not send AI task for legis init emphasis summarization: {e:?}");
            }
        }
    }

    if enforce_ai_run() && !should_update {
        log::info!("No need to update further!");
        return;
    }

    if let Some(vote_date) = db_legis_init.vote_date {
        if should_update && (today().date_naive() - vote_date).abs().num_days() > 30 {
            let exists: bool = sqlx::query_scalar!(
                r#"
                SELECT EXISTS (
                    SELECT 1
                    FROM pg_matviews
                    WHERE
                    matviewname = 'latest_legislative_initiatives'
                )
                "#
            )
            .fetch_one(pool)
            .await
            .unwrap_or_default()
            .unwrap_or_default();

            if exists {
                if let Err(e) = sqlx::query(
                    "REFRESH MATERIALIZED VIEW CONCURRENTLY latest_legislative_initiatives",
                )
                .execute(pool)
                .await
                {
                    log::error!("Could not refresh latest legislative initiatives view: {e:?}")
                }
            }
        }
    }

    if let Some(refs) = vote_result.references_to.as_ref() {
        if let Err(e) = insert_references_batch(&mut tx, refs, legis_init_id).await {
            log::error!("Could not insert references for legis init {legis_init_id}: {e:?}")
        }
    }

    if let Err(e) = insert_votes_batch(&mut tx, &vote_result.votes, legis_init_id).await {
        log::warn!("Error inserting votes: '{:?}'.", e);
    }

    if let Some(named_votes) = &vote_result.named_votes {
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

    #[cfg(not(feature = "ai"))]
    check_and_send_legis_init_mail(new_legis_init_tx, &vote_result, pool, legis_init_id).await;

    if let Err(e) = insert_topics_generic_batch(
        &mut tx,
        &vote_result.topics,
        legis_init_id,
        TopicType::Standard,
        TableType::LegislativeInitiative,
    )
    .await
    {
        log::error!("Error inserting topics for legis init {legis_init_id}: {e:?}");
    }

    if let Err(e) = insert_topics_generic_batch(
        &mut tx,
        &vote_result.eurovoc_topics,
        legis_init_id,
        TopicType::Eurovoc,
        TableType::LegislativeInitiative,
    )
    .await
    {
        log::error!("Error inserting eurovoc topics for legis init {legis_init_id}: {e:?}");
    }
    if let Err(e) = insert_topics_generic_batch(
        &mut tx,
        &vote_result.other_keyword_topics,
        legis_init_id,
        TopicType::OtherKeyword,
        TableType::LegislativeInitiative,
    )
    .await
    {
        log::error!("Error inserting other keyword topics for legis init {legis_init_id}: {e:?}");
    }

    if let Err(e) =
        insert_delegates_batch(&mut tx, &legis_init.related_delegates, legis_init_id).await
    {
        log::error!("Error inserting delegates for legis init {legis_init_id}: {e:?}");
    }

    if let Err(e) = insert_documents_batch(&mut tx, &vote_result.documents, legis_init_id).await {
        log::error!("Error inserting documents for legis init {legis_init_id}: {e:?}");
    }

    if let Err(e) = tx.commit().await {
        log::error!("Cannot commit legis init transaction: {e:?}");
    }

    if should_update {
        if let Some(client) = &app_state.redis_client {
            // TODO: could use patching approach in the future, currently, the receiver side is reconstructing everything from scratch
            if let Err(e) =
                notify_update_streamed(Index::VoteResults, &vote_result, &mut client.clone()).await
            {
                log::error!("Could not notify for vote result update!: {e:?}");
            }
        }
    }

    log::info!("Insertion duration for legis init: {:?}", start.elapsed());
}

async fn check_and_send_legis_init_mail(
    new_legis_init_tx: Option<Sender<(i32, VoteResult)>>,
    vote_result: &VoteResult,
    pool: &PgPool,
    legis_init_id: i32,
) {
    if vote_result.eurovoc_topics.is_empty()
        && vote_result
            .ai_summary
            .as_ref()
            .map(|ai_summary| ai_summary.full_summary.topics.is_empty())
            .unwrap_or(true)
    {
        return;
    }
    if let Some(new_legis_init_tx) = &new_legis_init_tx {
        if let Ok(info) = sqlx::query!(
            "select * from legis_email_info where already_sent_id = $1",
            legis_init_id
        )
        .fetch_optional(pool)
        .await
        {
            // TODO: use proper timezone handling for nr_plenary_activity_date
            let diff =
                today().date_naive() - vote_result.legislative_initiative.nr_plenary_activity_date;

            if diff.num_days() <= *crate::legislative_initiatives::LEGIS_INIT_STILL_SEND_DAYS
                && info.is_none()
                && vote_result.legislative_initiative.is_voteable_on
            {
                log::info!("Sending fitting legis init: {legis_init_id}");
                if let Err(e) = new_legis_init_tx
                    .send((legis_init_id, vote_result.clone()))
                    .await
                {
                    log::error!("could not send legis init to mail notify service: {e:?}")
                }

                if let Err(e) = sqlx::query!(
                    "insert into legis_email_info (already_sent_id) values ($1)",
                    legis_init_id
                )
                .execute(pool)
                .await
                {
                    log::error!(
                        "could not insert into legis email info for id {legis_init_id}: {e:?}"
                    );
                }
            }
        };
    }
}
