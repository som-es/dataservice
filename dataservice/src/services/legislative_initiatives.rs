use chrono::Days;
use combx::{
    document_text::{combine_texts, extract_text_from_documents_html_first},
    DbReference, DbVote, PostgresEnhancer, VoteResult,
};
use tokio::{runtime::Handle, sync::mpsc::Sender};
pub mod notify;
mod voted_by_name_result;
use voted_by_name_result::*;
mod ai_tasks;

use somes_scraper::{
    legis_init::{LegislativeInitiatives, LEGIS_IS_VOTEABLE_ON_DOKTYPES},
    AustriaScraper, LegislativeInitiative,
};
use sqlx::{postgres::PgQueryResult, query, Executor, PgPool, Postgres};
use std::sync::{Arc, LazyLock};

use crate::{
    ai_task::{self, AiTask},
    location,
    should_update::StoredUpdatedAt,
    today, today_and_time, Shutdown, GPS, LATEST_GP,
};

use dotenvy_macro::dotenv;

const LEGIS_INIT_STILL_SEND_DAYS: LazyLock<i64> = LazyLock::new(|| {
    dotenv!("LEGIS_INIT_STILL_SEND_DAYS")
        .parse::<i64>()
        .unwrap_or(10)
});

pub async fn scrape_new_legis_inits(
    pool: &PgPool,
    scraper: Arc<AustriaScraper>,
    gp: &str,
    new_legis_init: Option<Sender<Option<(i32, LegislativeInitiative)>>>,
    ai_task_tx: Sender<AiTask>,
    searched_doktyps: Option<&[&str]>,
) -> crate::Result<Vec<LegislativeInitiative>> {
    let legis_inits = scraper.post_legis_payload(searched_doktyps).await?;
    let legis_init_paths = scraper.legis_paths(legis_inits)?;

    const MAX_TASKS: usize = 32;
    let mut handles: Vec<tokio::task::JoinHandle<()>> = vec![];

    // let code_to_parties = Arc::new(gen_code_to_parties(pool, gp).await);

    log::info!("Fetching and inserting ({gp}) now.");
    let mut info_print_counter = 0;
    for legis_init_path in legis_init_paths.iter().rev() {
        let legis_init_path = legis_init_path.clone();
        let scraper = scraper.clone();
        let pool = pool.clone();
        // let code_to_parties = code_to_parties.clone();

        if handles.len() > MAX_TASKS {
            if info_print_counter % 16 == 0 {
                log::info!("Awaiting handles ({gp}) now.");
            }
            for handle in handles.drain(..MAX_TASKS) {
                if let Err(e) = handle.await {
                    if e.is_panic() {
                        log::error!("join Error: {e:?}")
                    }
                }
            }
            info_print_counter += 1;
        }
        let new_legis_init = new_legis_init.clone();
        let ai_task_tx = ai_task_tx.clone();

        handles.push(tokio::task::spawn_blocking(move || {
            Handle::current().block_on(async move {
                // skipping legis inits that can't be processed right now
                // mainly many I and A
                // https://www.parlament.gv.at/gegenstand/XXVII/I/1494
                // https://www.parlament.gv.at/gegenstand/XXVII/A/2686
                let Ok(legis_init) = scraper.get_legis_init(&legis_init_path).await else {
                    log::debug!("Cannot process legislative initiative: '{legis_init_path}'");
                    return;
                };

                update_legis_inits_in_db(&pool, &[legis_init], new_legis_init, Some(ai_task_tx))
                    .await;
                // new_legis_inits.push(legis_init);
            })
        }));

        for handle in handles.drain(..) {
            if let Err(e) = handle.await {
                if e.is_panic() {
                    log::warn!("join Error: {e:?}")
                }
            }
        }
    }

    Ok(vec![])
}

async fn legis_init_once(
    only_latest: bool,
    only_laws: bool,
    pg_pool: &PgPool,
    new_legis_init: Option<Sender<Option<(i32, LegislativeInitiative)>>>,
    ai_task_ts: Sender<AiTask>,
) -> Vec<LegislativeInitiative> {
    log::info!("Try scraping legislative initiatives");

    let mut handles = vec![];
    for gp in GPS {
        if only_latest && gp != LATEST_GP {
            continue;
        }
        let pg_pool = pg_pool.clone();
        let new_legis_init = new_legis_init.clone();
        let ai_task_ts = ai_task_ts.clone();
        let handle = tokio::task::spawn_blocking(move || {
            Handle::current().block_on(async move {
                let mut scraper = AustriaScraper::default();
                log::info!("LEGIS INIT once GP: {gp}");
                scraper.scrape_info.gp = vec![gp.into()];
                if only_latest {
                    let days = if only_laws {
                        // this number is a magic number -> maybe reason it with average completion time + n or something
                        // could still be possible that very old (+1001 days old) newly completed legis inits are never sent as email!
                        333
                    } else {
                        100
                    };
                    scraper.scrape_info.date_start =
                        today_and_time().checked_sub_days(Days::new(days));
                } else {
                    scraper.scrape_info.date_start = None;
                }

                let scraper = Arc::new(scraper);

                let searched_doktyps = if only_laws {
                    Some(LEGIS_IS_VOTEABLE_ON_DOKTYPES.as_slice())
                } else {
                    None
                };

                if let Err(e) = scrape_new_legis_inits(
                    &pg_pool,
                    scraper.clone(),
                    gp,
                    new_legis_init,
                    ai_task_ts,
                    searched_doktyps,
                )
                .await
                {
                    log::error!("could not scrape and insert legis inits: {e}");
                }
                // scrape_untill_success::<Vec<LegislativeInitiative>, _>(|| async {
                //     scrape_new_legis_inits(&pg_pool, scraper.clone(), gp).await
                // })
                // .await;

                log::info!("finished scraping LEGIS INIT once GP: {gp}");

                // let work_size = new_legis_inits.len() / 4;

                // if new_legis_inits.len() >= 16 {
                //     let mut handles = vec![];

                //     let batch1 = new_legis_inits[0..work_size].to_vec();
                //     let pg_pool1 = pg_pool.clone();
                //     handles.push(tokio::spawn(async move {
                //         log::info!("Updating batch1");
                //         update_legis_inits_in_db(&pg_pool1, &batch1, gp).await;
                //     }));

                //     let batch2 = new_legis_inits[work_size..work_size * 2].to_vec();
                //     let pg_pool2 = pg_pool.clone();
                //     handles.push(tokio::spawn(async move {
                //         log::info!("Updating batch2");
                //         update_legis_inits_in_db(&pg_pool2, &batch2, gp).await;
                //     }));

                //     let batch3 = new_legis_inits[work_size * 2..work_size * 3].to_vec();
                //     let pg_pool3 = pg_pool.clone();
                //     handles.push(tokio::spawn(async move {
                //         log::info!("Updating batch3");
                //         update_legis_inits_in_db(&pg_pool3, &batch3, gp).await;
                //     }));

                //     let batch4 = new_legis_inits[work_size * 3..].to_vec();
                //     let pg_pool4 = pg_pool.clone();

                //     log::info!("Updating batch4");
                //     update_legis_inits_in_db(&pg_pool4, &batch4, gp).await;
                //     for handle in handles {
                //         if let Err(e) = handle.await {
                //             log::error!("join error batching: {e:?}");
                //         }
                //     }
                // } else {
                //     update_legis_inits_in_db(&pg_pool, &new_legis_inits, gp).await;
                // }
            })
        });
        handles.push(handle);
        if only_latest {
            break;
        }
    }

    for handle in handles {
        if let Err(e) = handle.await {
            if e.is_panic() {
                log::error!("join error: {e:?}");
            }
        }
    }
    // new_legis_inits
    vec![]
}

impl StoredUpdatedAt for LegislativeInitiative {
    async fn stored_updated_at(&self, pg: &PgPool) -> Option<chrono::NaiveDateTime> {
        query!("select updated_at from legislative_initiatives where gp = $1 and ityp = $2 and inr = $3", self.gp, self.ityp, self.inr as i32)
            .fetch_optional(pg)
            .await
            .ok()
            .flatten()
            .map(|x| x.updated_at).flatten()
    }
}

pub async fn update_legis_inits_in_db(
    pool: &PgPool,
    new_legis_inits: &[LegislativeInitiative],
    new_legis_init: Option<Sender<Option<(i32, LegislativeInitiative)>>>,
    ai_task_tx: Option<Sender<AiTask>>,
) {
    // if let Some(jetzt_party_info) = code_to_parties.get("J").cloned() {
    //     code_to_parties.insert("P".to_string(), jetzt_party_info);
    // }

    for (i, legis_init) in new_legis_inits.iter().enumerate() {
        if !crate::should_update(legis_init, pool).await {
            log::info!("No need to update!");
            continue;
        }
        log::info!(
            "ADD legislative initiative: {} {} {}",
            legis_init.gp,
            legis_init.ityp,
            legis_init.inr
        );

        let mut vote_result =
            VoteResult::from_legis_init(PostgresEnhancer(pool.clone()), legis_init.clone(), None)
                .await;

        // println!("db_legis_init: {db_legis_init:?}");

        // let maybe_legis_init_id = sqlx::query!("select id, created_at from legislative_initiatives where inr = $1 and ityp = $2 and gp = $3", db_legis_init.inr, db_legis_init.ityp, db_legis_init.gp).fetch_optional(pool).await;
        let db_legis_init = vote_result.legislative_initiative;

        // let mut tx = pool.begin().await.unwrap();

        let legis_init_id = match sqlx::query!(
            "
                INSERT INTO legislative_initiatives (
                    title, inr, emphasis, description, ityp, gp, accepted, created_at, appeared_at, updated_at,
                    requires_simple_majority, pre_declined_type, voted_by_name, plenary_session_id, doktyp, is_law,
                    law_accepted, law_come_into_effect_date, law_expires_on_date, by_publication, has_reference, is_voteable_on,
                    is_urgent, voting
                ) VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24
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
                    law_come_into_effect_date = EXCLUDED.law_come_into_effect_date,
                    law_expires_on_date = EXCLUDED.law_expires_on_date,
                    by_publication = EXCLUDED.by_publication,
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
            db_legis_init.law_come_into_effect_date,
            db_legis_init.law_expires_on_date,
            db_legis_init.by_publication,
            db_legis_init.has_reference,
            db_legis_init.is_voteable_on,
            db_legis_init.is_urgent,
            db_legis_init.voting.map(|voting| voting.to_string()),
        )
        .fetch_one(pool)
        .await
        {
            Ok(record) => record.id,
            Err(e) => {
                log::error!("Error inserting legislative initiative: '{:?}'. Error occurred for legislative initiative: {:?}", e, legis_init);
                continue;
            }
        };

        vote_result.id = Some(legis_init_id);

        #[cfg(feature = "ai")]
        if let Some(ai_task_tx) = &ai_task_tx {
            if db_legis_init.is_voteable_on {
                let documents = vote_result.documents.clone();
                match tokio::spawn(async move {
                    use crate::OCR_ENGINE;

                    combine_texts(&extract_text_from_documents_html_first(&documents, OCR_ENGINE.as_ref()).await)
                })
                .await
                {
                    Ok(input) => {
                        if input.len() < 50 {
                            log::error!(
                                "Cannot summarize legis init {legis_init_id}, input is too short / empty for documents: {:?}",
                                vote_result.documents
                            );
                        } else {
                            let summarization_task = AiTask {
                                input,
                                task_id: crate::hash!(),
                                db_entry_id: legis_init_id,
                                task: Box::new(|task_args| {
                                    Box::pin(ai_tasks::summarize_laws(task_args))
                                }),
                            };

                            if let Err(e) = ai_task_tx.send(summarization_task).await {
                                log::error!(
                        "Could not send AI task for legis init emphasis summarization: {e:?}"
                    );
                            }
                        }
                    }
                    Err(e) => log::error!(
                        "cannot extract text input from {:?} because of {e:?}",
                        vote_result.documents
                    ),
                }
            }
        }

        for reference in vote_result.references_to.as_ref().unwrap_or(&vec![]) {
            if let Err(e) = insert_reference(pool, reference, legis_init_id).await {
                log::error!("Could not insert reference for legis init {legis_init_id}: {e:?}")
            }
        }

        let mut show_votes = legis_init.votes.clone();

        if legis_init.votes.is_empty() {
            if let Err(e) = sqlx::query!(
                "delete from votes where legislative_initiatives_id = $1",
                legis_init_id
            )
            .execute(pool)
            .await
            {
                log::error!(
                    "Cannot delete votes from legis init: {legis_init_id} with error {e:?}"
                );
            }
        }

        for vote in &vote_result.votes {
            insert_vote(pool, vote, legis_init_id).await;
        }
        if let Some(named_votes) = vote_result.named_votes {
            if let Ok(named_vote_info_id) =
                insert_named_vote_info(pool, &named_votes.named_vote_info, legis_init_id).await
            {
                let mut tx = pool.begin().await.unwrap();
                if let Err(e) = insert_named_votes(
                    &mut tx,
                    &named_votes.named_votes.as_ref().unwrap_or(&vec![]),
                    named_vote_info_id,
                )
                .await
                {
                    log::error!("Could not insert any named votes for legis init: {legis_init:?}, error: {e:?}")
                }

                if let Err(e) = tx.commit().await {
                    log::error!("Cannot commit legis init transaction: {e:?}");
                }
            }
        }

        if let Some(new_legis_init_tx) = &new_legis_init {
            if let Ok(info) = sqlx::query!(
                "select * from legis_email_info where already_sent_id = $1",
                legis_init_id
            )
            .fetch_optional(pool)
            .await
            {
                let diff = today() - legis_init.date;

                let mut legis_init = legis_init.clone();
                legis_init.votes = show_votes;
                if diff.num_days() <= *LEGIS_INIT_STILL_SEND_DAYS
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

        // Usage of this is deprecated! Use plenar speeches! This will combine two or more speeches held from a single delegate (on conflict clause)
        for speech in &vote_result.speeches {
            let speech_id = match sqlx::query!(
                "
                    INSERT INTO speeches (
                        delegate_id, infavor, legislative_initiatives_id, opinion
                    ) VALUES (
                        $1, $2, $3, $4
                    ) ON CONFLICT (legislative_initiatives_id, delegate_id)
                    DO UPDATE SET
                        infavor = EXCLUDED.infavor,
                        opinion = EXCLUDED.opinion
                    RETURNING id
                ",
                speech.delegate_id,
                speech.infavor,
                legis_init_id,
                speech.opinion,
            )
            .fetch_one(pool)
            .await
            {
                Err(e) => {
                    log::warn!("Error inserting speech, probably delegate does not exist in database and has got inactive recently: {:?}, for speech: {:?}", e, speech);
                    continue;
                }
                Ok(id) => id.id,
            };

            for html_url in &speech.document_url {
                if let Err(e) = sqlx::query!(
                    "
                    insert into speeches_html_urls(
                        speech_id, document_url
                    ) values (
                        $1, $2
                    ) on conflict (speech_id, document_url)
                    do nothing;
                ",
                    speech_id,
                    html_url
                )
                .execute(pool)
                .await
                {
                    log::error!("could not insert speech html url: {e:?}")
                }
            }
        }

        for topic in vote_result.topics.clone() {
            if let Err(e) = sqlx::query!(
                "
                    INSERT INTO unique_topics (
                        topic_name
                    ) VALUES (
                        $1
                    ) ON CONFLICT (topic_name)
                    DO NOTHING
                ",
                topic.topic,
            )
            .execute(pool)
            .await
            {
                log::error!("Error inserting UNIQUE (parent) topic: '{:?}'. The error occurred for the following eurovoc topic: {:?}", e, topic);
            }
            if let Err(e) = sqlx::query!(
                "
                    INSERT INTO topics_legis_init (
                        topic, legislative_initiatives_id
                    ) VALUES (
                        $1, $2
                    ) ON CONFLICT (legislative_initiatives_id, topic)
                    DO UPDATE SET
                        topic = EXCLUDED.topic
                ",
                topic.topic,
                legis_init_id
            )
            .execute(pool)
            .await
            {
                log::error!("Error inserting topic: '{:?}'. The error occurred for the following topic: {:?}", e, topic);
            }
        }

        for topic in vote_result.eurovoc_topics.clone() {
            if let Err(e) = sqlx::query!(
                "
                    INSERT INTO unique_eurovoc_topics (
                        topic_name
                    ) VALUES (
                        $1
                    ) ON CONFLICT (topic_name)
                    DO NOTHING
                ",
                topic.topic,
            )
            .execute(pool)
            .await
            {
                log::error!("Error inserting UNIQUE (eurovoc) topic: '{:?}'. The error occurred for the following eurovoc topic: {:?}", e, topic);
            }

            if let Err(e) = sqlx::query!(
                "
                    INSERT INTO eurovoc_topics_legis_init (
                        topic, legislative_initiatives_id
                    ) VALUES (
                        $1, $2
                    ) ON CONFLICT (legislative_initiatives_id, topic)
                    DO UPDATE SET
                        topic = EXCLUDED.topic
                ",
                topic.topic,
                legis_init_id,
            )
            .execute(pool)
            .await
            {
                log::error!("Error inserting eurovoc topic: '{:?}'. The error occurred for the following eurovoc topic: {:?}", e, topic);
            }
        }

        for delegate in &legis_init.related_delegates {
            let res = sqlx::query!("insert into legis_init_delegates (legis_init_id, delegate_id, delegate_text) values ($1, $2, $3) on conflict (legis_init_id, delegate_id) do update set delegate_text = EXCLUDED.delegate_text",
                legis_init_id, delegate.delegate_id as i32, delegate.text)
            .execute(pool).await;
            if let Err(e) = res {
                log::error!(
                    "Error inserting delegate into legis init delegates, 'eingebracht von': {e:?}"
                )
            }
        }

        for topic in vote_result.other_keyword_topics.clone() {
            let speech_id = match sqlx::query!(
                "
                    INSERT INTO other_keyword_topics_legis_init (
                        topic, legislative_initiatives_id
                    ) VALUES (
                        $1, $2
                    ) ON CONFLICT (legislative_initiatives_id, topic)
                    DO UPDATE SET
                        topic = EXCLUDED.topic
                ",
                topic.topic,
                legis_init_id,
            )
            .execute(pool)
            .await
            {
                Err(e) => {
                    log::error!("Error inserting other keyword topic: '{:?}'. The error occurred for the following other keyword topic: {:?}", e, topic);
                    continue;
                }
                Ok(id) => id,
            };
        }

        let mut tx = pool.begin().await.unwrap();

        if let Err(e) = sqlx::query!(
            "delete from legislative_documents where legislative_initiatives_id = $1",
            legis_init_id
        )
        .execute(&mut *tx)
        .await
        {
            log::error!("Error deleting old documents for legis init id {legis_init_id}: {e:?}");
        }

        for document in &vote_result.documents {
            if let Err(e) = sqlx::query!(
                "
                    INSERT INTO legislative_documents (
                        legislative_initiatives_id, title, document_type, document_url
                    ) VALUES (
                        $1, $2, $3, $4
                    ) ON CONFLICT (document_url, legislative_initiatives_id)
                    DO UPDATE SET
                        title = EXCLUDED.title,
                        document_type = EXCLUDED.document_type,
                        document_url = EXCLUDED.document_url
                ",
                legis_init_id,
                document.title,
                document.document_type,
                document.document_url
            )
            .execute(&mut *tx)
            .await
            {
                log::error!("Error inserting document: '{:?}'. The error occurred for the following document: {:?}", e, document);
            }
        }
        if let Err(e) = tx.commit().await {
            log::error!("Cannot commit legis init document transaction: {e:?}");
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
        .execute(pool)
        .await
        {
            log::error!("Error inserting was updated info: '{:?}'. The error occurred for the following legis init: {:?}", e, legis_init_id);
        }
    }
}

async fn insert_reference<'a>(
    pool: impl Executor<'a, Database = Postgres>,
    reference: &DbReference,
    legislative_initiatives_id: i32,
) -> sqlx::Result<PgQueryResult> {
    sqlx::query!("insert into legis_inits_refs (origin_legis_init_id, ref_gp, ref_ityp, ref_inr) values ($1, $2, $3, $4) on conflict do nothing", legislative_initiatives_id, reference.gp, reference.ityp, reference.inr as i32).execute(pool).await
}

pub async fn insert_vote<'a>(
    pool: impl Executor<'a, Database = Postgres>,
    db_vote: &DbVote,
    legis_init_id: i32,
) {
    if let Err(e) = sqlx::query!(
        "
            INSERT INTO votes (
                party, fraction, infavor, legislative_initiatives_id
            ) VALUES (
                $1, $2, $3, $4
            ) ON CONFLICT (legislative_initiatives_id, party)
            DO UPDATE SET
                fraction = EXCLUDED.fraction,
                infavor = EXCLUDED.infavor
        ",
        db_vote.party,
        db_vote.fraction,
        db_vote.infavor,
        legis_init_id
    )
    .execute(pool)
    .await
    {
        log::warn!(
            "Error inserting vote: '{:?}'. The error occurred for the following vote: {:?}",
            e,
            db_vote
        );
    }
}

pub async fn run_shut_latest_gp(
    mut shutdown: Shutdown,
    pg_pool: PgPool,
    new_legis_init: Sender<Option<(i32, LegislativeInitiative)>>,
    ai_task_ts: Sender<AiTask>,
    legis_init_sleep: u64,
    only_latest: bool,
    only_laws: bool,
) {
    #[cfg(feature = "legis_init_latest")]
    while !shutdown.is_shutdown() {
        let new_legis_init = new_legis_init.clone();
        let ai_task_ts = ai_task_ts.clone();
        tokio::select! {
            _ = shutdown.recv() => {
                println!("shutting down");
                // return ok?
                break;
            }
            _ = async {
                let pg_pool = pg_pool.clone();
                if let Err(e) = tokio::task::spawn(async move {
                    log::info!("Starting legislative initiative (latest) scraper");

                    legis_init_once(only_latest, only_laws, &pg_pool, Some(new_legis_init.clone()), ai_task_ts.clone()).await;

                    // triggers the email service
                    let res = new_legis_init.send(None).await;
                    if let Err(e) = res {
                        log::error!("Could not send trigger value to email service: {e:?}");
                    }
                }).await {
                    log::error!("Error in legislative initiative (latest) scraper: {:?}", e);
                }


                // legis_init_once().await;
                log::info!("Legislative initiative (latest) scraper service sleeps for {legis_init_sleep} seconds...");
                tokio::time::sleep(tokio::time::Duration::from_secs(legis_init_sleep)).await;
            } => {}
        }
    }
}

pub async fn run_shut_every_gp(
    mut shutdown: Shutdown,
    pg_pool: PgPool,
    ai_task_ts: Sender<AiTask>,
) {
    let legis_init_sleep = std::env::var("LEGISLATIVE_INITIATIVE_SLEEP")
        .unwrap_or_else(|_| "1000".to_string())
        .parse()
        .unwrap_or_else(|e| {
            log::warn!(
                "Could not parse LEGISLATIVE_INITIATIVE_SLEEP env var: {e}. Using default value of 1000 seconds"
            );
            1000
        });

    #[cfg(feature = "legis_init")]
    while !shutdown.is_shutdown() {
        let ai_task_ts = ai_task_ts.clone();
        tokio::select! {
            _ = shutdown.recv() => {
                println!("shutting down");
                // return ok?
                break;
            }
            _ = async {
                let pg_pool = pg_pool.clone();
                if let Err(e) = tokio::task::spawn(async move {
                    log::info!("Starting legislative initiative scraper");

                    legis_init_once(false, true, &pg_pool, None, ai_task_ts.clone()).await;
                }).await {
                    log::error!("Error in legislative initiative scraper: {:?}", e);
                }
                // legis_init_once().await;
                log::info!("Legislative initiative scraper service sleeps for {legis_init_sleep} seconds...");
                tokio::time::sleep(tokio::time::Duration::from_secs(legis_init_sleep)).await;
            } => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::{NaiveDate, NaiveDateTime};
    use somes_scraper::{legis_init::LegislativeInitiatives, AustriaScraper};
    use sqlx::postgres::PgPoolOptions;

    use crate::{connect_pg, DATABASE_URL};

    #[test]
    fn test_date_time_parse() {
        let x = NaiveDateTime::parse_from_str("2014-5-17T12:34:56", "%Y-%m-%dT%H:%M:%S").unwrap();
        println!("x: {x:?}");
    }

    #[tokio::test]
    async fn test_legis_init_empty_gp_scrape() {
        let mut scraper: AustriaScraper = AustriaScraper::default();
        let res = scraper
            .get_legis_init("/gegenstand/XXVII/I/2269")
            .await
            .unwrap();

        let pool = connect_pg().await;
        // crate::legislative_initiatives::update_legis_inits_in_db(&pool, &[res], "XXVII").await;

        // println!("s: {:?}", scraper.scrape_info.gp);

        // let out = crate::legislative_initiatives::scrape_new_legis_inits(&scraper).await.unwrap();
        // println!("out: {out:?}");
    }

    #[tokio::test]
    async fn test_legis_init_empty_gp_scrape_xxvi_a_110() {
        // let mut scraper: AustriaScraper = AustriaScraper::default();
        // let res = scraper
        //     .get_legis_init("/gegenstand/XXVI/A/238")
        //     .await
        //     .unwrap();

        // let pool = pollster::block_on(connect_pg());
        let pool = PgPoolOptions::new()
            .max_connections(150)
            .connect(DATABASE_URL)
            .await
            .unwrap();
        println!("ppoll");

        // let date = "18.05.2016";
        let date = NaiveDate::from_ymd_opt(2013, 11, 18).unwrap();
        // let parties = gen_code_to_parties_with_date(&pool, &date, "XXV").await;
        // println!("{parties:?}");

        // crate::legislative_initiatives::update_legis_inits_in_db(&pool, &[res], "XXVI").await;

        // println!("s: {:?}", scraper.scrape_info.gp);

        // let out = crate::legislative_initiatives::scrape_new_legis_inits(&scraper).await.unwrap();
        // println!("out: {out:?}");
    }
    #[tokio::test]
    async fn test_legis_init_empty_gp_scrape_xxvii_a_3431() {
        let scraper: AustriaScraper = AustriaScraper::default();
        let res = scraper
            .get_legis_init("/gegenstand/XXVII/A/3431")
            .await
            .unwrap();

        let pool = connect_pg().await;
        crate::legislative_initiatives::update_legis_inits_in_db(&pool, &[res], None, None).await;
    }

    #[tokio::test]
    async fn test_legis_init_empty_gp_scrape_xxvii_a_3945() {
        let mut scraper: AustriaScraper = AustriaScraper::default();
        let res = scraper
            .get_legis_init("/gegenstand/XXVII/A/3945")
            .await
            .unwrap();

        let pool = connect_pg().await;
        crate::legislative_initiatives::update_legis_inits_in_db(&pool, &[res], None, None).await;

        // println!("s: {:?}", scraper.scrape_info.gp);

        // let out = crate::legislative_initiatives::scrape_new_legis_inits(&scraper).await.unwrap();
        // println!("out: {out:?}");
    }

    #[tokio::test]
    async fn test_legis_init_empty_gp_scrape_xxvii_i_2711() {
        let scraper: AustriaScraper = AustriaScraper::default();
        let res = scraper
            .get_legis_init("/gegenstand/XXVII/I/2711")
            .await
            .unwrap();

        let pool = connect_pg().await;
        crate::legislative_initiatives::update_legis_inits_in_db(&pool, &[res], None, None).await;
    }

    #[tokio::test]
    async fn test_legis_init_empty_gp_scrape_xxvii_a_4141() {
        let scraper: AustriaScraper = AustriaScraper::default();
        let res = scraper
            .get_legis_init("/gegenstand/XXVII/I/1858")
            .await
            .unwrap();

        let pool = connect_pg().await;
        crate::legislative_initiatives::update_legis_inits_in_db(&pool, &[res], None, None).await;
    }

    #[tokio::test]
    async fn test_legis_init_empty_gp_scrape_xxvii_iii_835() {
        let scraper: AustriaScraper = AustriaScraper::default();
        let res = scraper
            .get_legis_init("/gegenstand/XXVII/III/85")
            .await
            .unwrap();

        let pool = connect_pg().await;
        crate::legislative_initiatives::update_legis_inits_in_db(&pool, &[res], None, None).await;
    }

    #[tokio::test]
    async fn test_legis_init_empty_gp_scrape_xxviii_uea_8() {
        simple_logger::init_with_level(log::Level::Info).unwrap();
        let scraper: AustriaScraper = AustriaScraper::default();
        let res = scraper
            .get_legis_init("/gegenstand/XXVIII/UEA/8")
            .await
            .unwrap();

        let pool = connect_pg().await;
        crate::legislative_initiatives::update_legis_inits_in_db(&pool, &[res], None, None).await;
    }

    #[tokio::test]
    async fn test_legis_init_empty_gp_scrape_xxviii_a_104() {
        simple_logger::init_with_level(log::Level::Info).unwrap();
        let scraper: AustriaScraper = AustriaScraper::default();
        let res = scraper
            .get_legis_init("/gegenstand/XXVIII/A/104")
            .await
            .unwrap();

        let pool = connect_pg().await;
        crate::legislative_initiatives::update_legis_inits_in_db(&pool, &[res], None, None).await;
    }

    #[tokio::test]
    async fn test_legis_init_empty_gp_scrape_xxviii_a_64() {
        simple_logger::init_with_level(log::Level::Info).unwrap();
        let scraper: AustriaScraper = AustriaScraper::default();
        let res = scraper
            .get_legis_init("/gegenstand/XXVIII/A/64")
            .await
            .unwrap();

        log::info!("legis init: {res:?}");

        let pool = connect_pg().await;
        crate::legislative_initiatives::update_legis_inits_in_db(&pool, &[res], None, None).await;
    }

    #[tokio::test]
    async fn test_legis_init_empty_gp_scrape_xxviii_a_77() {
        simple_logger::init_with_level(log::Level::Info).unwrap();
        let scraper: AustriaScraper = AustriaScraper::default();
        let res = scraper
            .get_legis_init("/gegenstand/XXVIII/A/77")
            .await
            .unwrap();

        log::info!("legis init: {res:?}");

        let pool = connect_pg().await;
        crate::legislative_initiatives::update_legis_inits_in_db(&pool, &[res], None, None).await;
    }
}
