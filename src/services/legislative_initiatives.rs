mod match_delegates_with_name;
mod voted_by_name_result;

use std::collections::HashMap;

use scraper::{
    legis_init::LegislativeInitiatives, AustriaScraper, LegislativeInitiative, Opinion, Party,
    ProContra,
};
use sqlx::PgPool;

use crate::{
    connect_pg,
    models::{
        DbEurovocTopicLegisInit, DbLegisDocument, DbLegislativeInitiative,
        DbOtherKeywordTopicLegisInit, DbSpeech, DbTopicLegisInit, DbVote,
    },
    services::scrape_untill_success,
    with_data::all_parties_at_gp,
    Shutdown,
};

pub async fn scrape_new_legis_inits(
    scraper: &AustriaScraper,
) -> crate::Result<Vec<LegislativeInitiative>> {
    let legis_inits = scraper.post_legis_payload().await?;
    let legis_init_paths = scraper.legis_paths(legis_inits)?;

    let mut new_legis_inits = Vec::new();

    for legis_init_path in &legis_init_paths {
        // skipping legis inits that can't be processed right now
        // mainly many I and A
        // https://www.parlament.gv.at/gegenstand/XXVII/I/1494
        // https://www.parlament.gv.at/gegenstand/XXVII/A/2686
        let Ok(legis_init) = scraper.get_legis_init(&legis_init_path).await else {
            log::debug!("Cannot process legislative initiative: '{legis_init_path}'");
            continue;
        };

        new_legis_inits.push(legis_init);
    }

    Ok(new_legis_inits)
}

async fn legis_init_once(only_latest: bool, pg_pool: &PgPool) -> Vec<LegislativeInitiative> {
    log::info!("Try scraping legislative initiatives");

    for gp in ["XXVII", "XXVI", "XXV", "XXIV", "XXIII", "XXII", "XXI", "XX"] {
        if !only_latest && gp == "XXVII" {
            continue;
        }
        let pg_pool = pg_pool.clone();
        tokio::spawn(async move {
            let mut scraper = AustriaScraper::default();
            log::info!("LEGIS INIT once GP: {gp}");
            scraper.scrape_info.gp = vec![gp.into()];

            let new_legis_inits =
                scrape_untill_success::<Vec<LegislativeInitiative>, _>(|| async {
                    scrape_new_legis_inits(&scraper).await
                })
                .await;

            log::info!("finished scraping LEGIS INIT once GP: {gp}");

            let work_size = new_legis_inits.len() / 4;

            if new_legis_inits.len() >= 16 {
                let mut handles = vec![];

                let batch1 = new_legis_inits[0..work_size].to_vec();
                let pg_pool1 = pg_pool.clone();
                handles.push(tokio::spawn(async move {
                    log::info!("Updating batch1");
                    update_legis_inits_in_db(&pg_pool1, &batch1, gp).await;
                }));

                let batch2 = new_legis_inits[work_size..work_size * 2].to_vec();
                let pg_pool2 = pg_pool.clone();
                handles.push(tokio::spawn(async move {
                    log::info!("Updating batch2");
                    update_legis_inits_in_db(&pg_pool2, &batch2, gp).await;
                }));

                let batch3 = new_legis_inits[work_size * 2..work_size * 3].to_vec();
                let pg_pool3 = pg_pool.clone();
                handles.push(tokio::spawn(async move {
                    log::info!("Updating batch3");
                    update_legis_inits_in_db(&pg_pool3, &batch3, gp).await;
                }));

                let batch4 = new_legis_inits[work_size * 3..].to_vec();
                let pg_pool4 = pg_pool.clone();

                log::info!("Updating batch4");
                update_legis_inits_in_db(&pg_pool4, &batch4, gp).await;
            } else {
                update_legis_inits_in_db(&pg_pool, &new_legis_inits, gp).await;
            }
        });
        if only_latest {
            break;
        }
    }

    // new_legis_inits
    vec![]
}

pub async fn update_legis_inits_in_db(
    pool: &PgPool,
    new_legis_inits: &[LegislativeInitiative],
    gp: &str,
) {
    log::info!("Updating legislative initiatives {gp} in database...");

    let mut code_to_parties = all_parties_at_gp(pool, gp)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|party| (party.code.clone(), party))
        .collect::<HashMap<String, Party>>();

    if let Some(jetzt_party_info) = code_to_parties.get("J").cloned() {
        code_to_parties.insert("P".to_string(), jetzt_party_info);
    }

    for (i, legis_init) in new_legis_inits.iter().enumerate() {
        if i % 10 == 0 {
            log::info!(
                "{i} inserting legis init {}{}",
                legis_init.inr,
                legis_init.ityp
            );
        }
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
        };
        // println!("db_legis_init: {db_legis_init:?}");

        let legis_init_id = match sqlx::query!(
            "
                INSERT INTO legislative_initiatives (
                    title, inr, emphasis, description, ityp, gp, accepted, created_at, appeared_at, updated_at,
                    requires_simple_majority, pre_declined_type, voted_by_name
                ) VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13
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
                    voted_by_name = EXCLUDED.voted_by_name
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

        if legis_init.votes.is_empty()
            && legis_init.accepted == Some(scraper::AcceptedState::PreDeclined)
            && legis_init.hidden_votes_info.pro_contra_parties.is_some()
        {
            for db_vote in translate_votes(
                &code_to_parties,
                legis_init_id,
                legis_init
                    .hidden_votes_info
                    .pro_contra_parties
                    .as_ref()
                    .unwrap(),
            )
            .await
            {
                insert_vote(pool, db_vote).await;
            }
        }

        for speech in &legis_init.speeches {
            let db_speech = DbSpeech {
                delegate_id: speech.delegate_id as i32,
                infavor: match speech.opinion {
                    Opinion::Infavor => Some(true),
                    Opinion::Against => Some(false),
                    _ => None,
                },
                legislative_initiatives_id: legis_init_id,
            };

            if let Err(e) = sqlx::query!(
                "
                    INSERT INTO speeches (
                        delegate_id, infavor, legislative_initiatives_id
                    ) VALUES (
                        $1, $2, $3
                    ) ON CONFLICT (legislative_initiatives_id, delegate_id) 
                    DO UPDATE SET 
                        infavor = EXCLUDED.infavor
                ",
                db_speech.delegate_id,
                db_speech.infavor,
                db_speech.legislative_initiatives_id
            )
            .execute(pool)
            .await
            {
                log::warn!("Error inserting speech, probably delegate does not exist in database and has got inactive recently: {:?}, for speech: {:?}", e, speech);
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

            if let Err(e) = sqlx::query!(
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
                log::error!("Error inserting other keyword topic: '{:?}'. The error occurred for the following other keyword topic: {:?}", e, db_other_keyword_topic);
            }
        }

        for document in &legis_init.documents {
            let db_legis_doc = DbLegisDocument {
                legislative_initiatives_id: legis_init_id,
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
                db_legis_doc.legislative_initiatives_id,
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
) -> Vec<DbVote> {
    let convert_to_vote = |code, infavor| {
        let Some(party) = parties.get(code).cloned() else {
            log::warn!("Party with code {code} not found in parties map: {parties:?}");
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
    use chrono::NaiveDateTime;
    use scraper::{legis_init::LegislativeInitiatives, AustriaScraper};

    use crate::connect_pg;

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
        crate::legislative_initiatives::update_legis_inits_in_db(&pool, &[res], "XXVII").await;

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
        crate::legislative_initiatives::update_legis_inits_in_db(&pool, &[res], "XXVI").await;

        // println!("s: {:?}", scraper.scrape_info.gp);

        // let out = crate::legislative_initiatives::scrape_new_legis_inits(&scraper).await.unwrap();
        // println!("out: {out:?}");
    }
}
