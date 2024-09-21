mod absence_matching;
pub use absence_matching::*;
mod match_delegates_with_name;
mod voted_by_name_result;

use std::{collections::HashMap, sync::Arc};

use log::log;
use scraper::{
    legis_init::LegislativeInitiatives, AustriaScraper, LegislativeInitiative, Opinion, Party,
    ProContra,
};
use sqlx::PgPool;
use voted_by_name_result::{insert_named_vote_info, insert_named_votes};

use crate::{
    connect_pg,
    models::{
        DbEurovocTopicLegisInit, DbLegisDocument, DbLegislativeInitiative, DbNamedVoteInfo,
        DbOtherKeywordTopicLegisInit, DbSpeech, DbTopicLegisInit, DbVote,
    },
    services::scrape_untill_success,
    with_data::all_parties_at_gp,
    Shutdown,
};

pub async fn gen_code_to_parties(pool: &PgPool, gp: &str) -> HashMap<String, Party> {
    all_parties_at_gp(pool, gp)
        .await
        .map_err(|e| {
            log::error!("all parties for gp error in legis init: {e:?}");
            e
        })
        .unwrap_or_default()
        .into_iter()
        .map(|party| (party.code.clone(), party))
        .collect::<HashMap<String, Party>>()
}

pub async fn scrape_new_legis_inits(
    pool: &PgPool,
    scraper: Arc<AustriaScraper>,
    gp: &str,
) -> crate::Result<Vec<LegislativeInitiative>> {
    let legis_inits = scraper.post_legis_payload().await?;
    let legis_init_paths = scraper.legis_paths(legis_inits)?;

    const MAX_TASKS: usize = 2;
    let mut handles: Vec<tokio::task::JoinHandle<()>> = vec![];

    let code_to_parties = Arc::new(gen_code_to_parties(pool, gp).await);

    log::info!("Fetching and inserting ({gp}) now.");
    let mut info_print_counter = 0;
    for legis_init_path in legis_init_paths.iter().rev() {
        let legis_init_path = legis_init_path.clone();
        let scraper = scraper.clone();
        let pool = pool.clone();
        let code_to_parties = code_to_parties.clone();

        if handles.len() > MAX_TASKS {
            if info_print_counter % 16 == 0 {
                log::info!("Awaiting handles ({gp}) now.");
            }
            for handle in handles.drain(..MAX_TASKS) {
                if let Err(e) = handle.await {
                    log::warn!("join Error: {e:?}")
                }
            }
            info_print_counter += 1;
        }

        handles.push(tokio::spawn(async move {
            // skipping legis inits that can't be processed right now
            // mainly many I and A
            // https://www.parlament.gv.at/gegenstand/XXVII/I/1494
            // https://www.parlament.gv.at/gegenstand/XXVII/A/2686
            let Ok(legis_init) = scraper.get_legis_init(&legis_init_path).await else {
                log::debug!("Cannot process legislative initiative: '{legis_init_path}'");
                return;
            };

            update_legis_inits_in_db(&pool, code_to_parties, &[legis_init]).await;
            // new_legis_inits.push(legis_init);
        }));
    }

    Ok(vec![])
}

async fn legis_init_once(only_latest: bool, pg_pool: &PgPool) -> Vec<LegislativeInitiative> {
    log::info!("Try scraping legislative initiatives");

    let mut handles = vec![];
    for gp in ["XXVII", "XXVI", "XXV", "XXIV", "XXIII", "XXII", "XXI", "XX"] {
        if !only_latest && gp == "XXVII" {
            continue;
        }
        let pg_pool = pg_pool.clone();
        let handle = tokio::spawn(async move {
            let mut scraper = AustriaScraper::default();
            log::info!("LEGIS INIT once GP: {gp}");
            scraper.scrape_info.gp = vec![gp.into()];
            let scraper = Arc::new(scraper);

            if let Err(e) = scrape_new_legis_inits(&pg_pool, scraper.clone(), gp).await {
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
        });
        handles.push(handle);
        if only_latest {
            break;
        }
    }

    for handle in handles {
        if let Err(e) = handle.await {
            log::error!("join error: {e:?}");
        }
    }
    // new_legis_inits
    vec![]
}

pub async fn update_legis_inits_in_db(
    pool: &PgPool,
    code_to_parties: Arc<HashMap<String, Party>>,
    new_legis_inits: &[LegislativeInitiative],
) {
    // if let Some(jetzt_party_info) = code_to_parties.get("J").cloned() {
    //     code_to_parties.insert("P".to_string(), jetzt_party_info);
    // }

    for (i, legis_init) in new_legis_inits.iter().enumerate() {
        log::info!("Add legis init: {} {} {}", legis_init.gp, legis_init.ityp, legis_init.inr);
        // if i % 10 == 0 {
        //     log::info!(
        //         "{i} inserting legis init {}{}",
        //         legis_init.inr,
        //         legis_init.ityp
        //     );
        // }

        let plenary_session_id = match legis_init.plenary_session {
            Some(session) => sqlx::query!(
                "select id from plenar_infos where inr = $1 and legislative_period = $2",
                session as i32,
                legis_init.gp
            )
            .fetch_one(pool)
            .await
            .ok()
            .map(|id| id.id),
            None => None,
        };

        let db_legis_init = DbLegislativeInitiative {
            title: legis_init.title.clone(),
            inr: legis_init.inr as i32,
            emphasis: legis_init.emphasis.clone(),
            description: legis_init.description.clone(),
            ityp: legis_init.ityp.clone(),
            gp: legis_init.gp.clone(),
            accepted: legis_init.accepted.map(|a| a.to_string()),
            created_at: legis_init.date,
            appeared_at: Some(legis_init.created_at),
            updated_at: legis_init.updated_at,
            requires_simple_majority: legis_init.requires_simple_majority,
            pre_declined_type: legis_init
                .hidden_votes_info
                .vote_type
                .map(|pre| pre.to_string()),
            voted_by_name: legis_init.hidden_votes_info.voted_by_name,
            plenary_session_id
        };
        // println!("db_legis_init: {db_legis_init:?}");

        let legis_init_id = match sqlx::query!(
            "
                INSERT INTO legislative_initiatives (
                    title, inr, emphasis, description, ityp, gp, accepted, created_at, appeared_at, updated_at,
                    requires_simple_majority, pre_declined_type, voted_by_name, plenary_session_id
                ) VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14
                ) ON CONFLICT (inr, ityp, gp) 
                DO UPDATE SET 
                    title = EXCLUDED.title,
                    emphasis = EXCLUDED.emphasis,
                    description = EXCLUDED.description,
                    accepted = EXCLUDED.accepted,
                    created_at = EXCLUDED.created_at,
                    appeared_at = EXCLUDED.appeared_at,
                    updated_at = EXCLUDED.updated_at,
                    requires_simple_majority = EXCLUDED.requires_simple_majority,
                    pre_declined_type = EXCLUDED.pre_declined_type,
                    voted_by_name = EXCLUDED.voted_by_name,
                    plenary_session_id = EXCLUDED.plenary_session_id
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

        for vote in &legis_init.votes {
            let db_vote = DbVote {
                party: vote.party.name.clone(),
                fraction: vote.party.fraction as i32,
                infavor: vote.infavor,
                legislative_initiatives_id: legis_init_id,
            };
            insert_vote(pool, db_vote).await;
        }

        if let Some(voted_by_name_data) = &legis_init.hidden_votes_info.voted_by_name_data {
            if let Ok(named_vote_info_id) = insert_named_vote_info(
                pool,
                &DbNamedVoteInfo {
                    id: 0,
                    legis_init_id,
                    pro_count: voted_by_name_data.pro_count as i32,
                    contra_count: voted_by_name_data.contra_count as i32,
                    given_vote_sum: voted_by_name_data.vote_sum as i32,
                    invalid_count: voted_by_name_data.invalid_count as i32,
                },
            )
            .await
            {
                if let Some(pro_contra) = &voted_by_name_data.delegates_pro_contra {
                    if let Err(e) = insert_named_votes(
                        pool,
                        &pro_contra,
                        legis_init.date,
                        &legis_init.gp,
                        named_vote_info_id,
                    )
                    .await
                    {
                        log::error!("Could not insert any named votes for legis init: {legis_init:?}, error: {e:?}")
                    }
                }
            }
        }

        if legis_init.votes.is_empty()
            && legis_init.accepted == Some(scraper::AcceptedState::PreDeclined)
            && legis_init.hidden_votes_info.pro_contra_parties.is_some()
        {
            match &legis_init.hidden_votes_info.pro_contra_parties {
                Some(pro_contra_parties) => {
                    for db_vote in translate_votes(
                        &code_to_parties,
                        legis_init_id,
                        pro_contra_parties,
                        &legis_init.gp,
                    )
                    .await
                    {
                        insert_vote(pool, db_vote).await;
                    }
                }
                None => {
                    log::warn!(
                        "Pro Contra is empty despite hidden vote info for legis init: {}, {}, {}",
                        legis_init.gp,
                        legis_init.ityp,
                        legis_init.inr
                    )
                }
            }
        }

        for speech in &legis_init.speeches {
            let opinion = match &speech.opinion {
                scraper::Opinion::Infavor => Some("Pro".to_string()),
                scraper::Opinion::Against => Some("Contra".to_string()),
                scraper::Opinion::Other(text) => Some(text.clone()),
                scraper::Opinion::None => None,
            };
            let db_speech = DbSpeech {
                delegate_id: speech.delegate_id as i32,
                infavor: match speech.opinion {
                    Opinion::Infavor => Some(true),
                    Opinion::Against => Some(false),
                    _ => None,
                },
                opinion,
                legislative_initiatives_id: legis_init_id,
            };

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
                db_speech.delegate_id,
                db_speech.infavor,
                db_speech.legislative_initiatives_id,
                db_speech.opinion,
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

            for html_url in &speech.spoken_text_html_urls {
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
                    html_url.document_url
                )
                .execute(pool)
                .await
                {
                    log::warn!("could not insert speech html url: {e:?}")
                }
            }
        }

        for topic in legis_init.topics.clone() {
            let db_topic = DbTopicLegisInit {
                topic,
                legislative_initiatives_id: legis_init_id,
            };

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
                db_topic.topic,
                db_topic.legislative_initiatives_id
            )
            .execute(pool)
            .await
            {
                log::error!("Error inserting topic: '{:?}'. The error occurred for the following topic: {:?}", e, db_topic);
            }
        }

        for topic in legis_init.eurovoc_topics.clone() {
            let db_eurovoc_topic = DbEurovocTopicLegisInit {
                topic,
                legislative_initiatives_id: legis_init_id,
            };

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
                db_eurovoc_topic.topic,
                db_eurovoc_topic.legislative_initiatives_id
            )
            .execute(pool)
            .await
            {
                log::error!("Error inserting eurovoc topic: '{:?}'. The error occurred for the following eurovoc topic: {:?}", e, db_eurovoc_topic);
            }
        }

        for topic in legis_init.other_keyword_topics.clone() {
            let db_other_keyword_topic = DbOtherKeywordTopicLegisInit {
                topic,
                legislative_initiatives_id: legis_init_id,
            };

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
                db_other_keyword_topic.topic,
                db_other_keyword_topic.legislative_initiatives_id
            )
            .execute(pool)
            .await
            {
                Err(e) => {
                    log::error!("Error inserting other keyword topic: '{:?}'. The error occurred for the following other keyword topic: {:?}", e, db_other_keyword_topic);
                    continue;
                }
                Ok(id) => id,
            };
        }

        for document in &legis_init.documents {
            let db_legis_doc = DbLegisDocument {
                // legislative_initiatives_id: legis_init_id,
                title: document.title.clone(),
                document_type: document.ftype.to_string(),
                document_url: document.link.clone(),
            };

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
                db_legis_doc.title,
                db_legis_doc.document_type,
                db_legis_doc.document_url
            )
            .execute(pool)
            .await
            {
                log::error!("Error inserting document: '{:?}'. The error occurred for the following document: {:?}", e, document);
            }
        }
    }
}

pub async fn insert_vote(pool: &PgPool, db_vote: DbVote) {
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
        db_vote.legislative_initiatives_id
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

pub async fn translate_votes(
    parties: &HashMap<String, Party>,
    legislative_initiatives_id: i32,
    pro_contra: &ProContra,
    gp: &str,
) -> Vec<DbVote> {
    let convert_to_vote = |code, infavor| {
        let Some(party) = parties.get(code).cloned() else {
            log::error!("Party with code {code} not found in {gp} parties map: {parties:?}");
            return None;
        };

        Some(DbVote {
            party: party.name,
            fraction: party.fraction as i32,
            infavor,
            legislative_initiatives_id,
        })
    };
    let mut votes = vec![];
    votes.extend(
        pro_contra
            .contra
            .iter()
            .flat_map(|code| convert_to_vote(code, false)),
    );
    votes.extend(
        pro_contra
            .pro
            .iter()
            .flat_map(|code| convert_to_vote(code, true)),
    );
    votes
}

pub async fn run_shut_latest_gp(mut shutdown: Shutdown) {
    let legis_init_sleep = std::env::var("LEGISLATIVE_INITIATIVE_SLEEP_LATEST")
        .unwrap_or_else(|_| "300".to_string())
        .parse()
        .unwrap_or_else(|e| {
            log::warn!(
                "Could not parse LEGISLATIVE_INITIATIVE_SLEEP_LATEST env var: {e}. Using default value of 300 seconds"
            );
            300
        });

    #[cfg(feature = "legis_init_latest")]
    while !shutdown.is_shutdown() {
        tokio::select! {
            _ = shutdown.recv() => {
                println!("shutting down");
                // return ok?
                break;
            }
            _ = async {
                if let Err(e) = tokio::task::spawn(async move {
                    log::info!("Starting legislative initiative (latest) scraper");

                    let pg_pool = connect_pg().await;
                    legis_init_once(true, &pg_pool).await;
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

pub async fn run_shut_every_gp(mut shutdown: Shutdown) {
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
        tokio::select! {
            _ = shutdown.recv() => {
                println!("shutting down");
                // return ok?
                break;
            }
            _ = async {
                if let Err(e) = tokio::task::spawn(async move {
                    log::info!("Starting legislative initiative scraper");

                    let pg_pool = connect_pg().await;
                    legis_init_once(false, &pg_pool).await;
                    log::info!("lol");
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

    use chrono::NaiveDateTime;
    use scraper::{legis_init::LegislativeInitiatives, AustriaScraper};

    use crate::{connect_pg, legislative_initiatives::gen_code_to_parties};

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
        let mut scraper: AustriaScraper = AustriaScraper::default();
        let res = scraper
            .get_legis_init("/gegenstand/XXVI/A/238")
            .await
            .unwrap();

        let pool = connect_pg().await;
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
        let code_to_parties = Arc::new(gen_code_to_parties(&pool, &res.gp).await);
        crate::legislative_initiatives::update_legis_inits_in_db(&pool, code_to_parties, &[res])
            .await;
    }


    #[tokio::test]
    async fn test_legis_init_empty_gp_scrape_xxvii_a_3945() {
        let mut scraper: AustriaScraper = AustriaScraper::default();
        let res = scraper
            .get_legis_init("/gegenstand/XXVII/A/3945")
            .await
            .unwrap();

        let pool = connect_pg().await;
        let code_to_parties = Arc::new(gen_code_to_parties(&pool, &res.gp).await);
        crate::legislative_initiatives::update_legis_inits_in_db(&pool, code_to_parties, &[res])
            .await;

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
        let code_to_parties = Arc::new(gen_code_to_parties(&pool, &res.gp).await);
        crate::legislative_initiatives::update_legis_inits_in_db(&pool, code_to_parties, &[res])
            .await;
    }

    #[tokio::test]
    async fn test_legis_init_empty_gp_scrape_xxvii_a_4141() {
        let scraper: AustriaScraper = AustriaScraper::default();
        let res = scraper
            .get_legis_init("/gegenstand/XXVII/A/4141")
            .await
            .unwrap();

        let pool = connect_pg().await;
        let code_to_parties = Arc::new(gen_code_to_parties(&pool, &res.gp).await);
        crate::legislative_initiatives::update_legis_inits_in_db(&pool, code_to_parties, &[res])
            .await;
    }
}
