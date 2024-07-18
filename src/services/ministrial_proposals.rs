use diesel::RunQueryDsl;
use scraper::{
    ministrial_proposals::{MinistrialProposal, MinistrialProposalScraper},
    proposals::ProposalsPayload,
    AustriaScraper,
};
use tokio::time::Instant;

use crate::{
    connect_pg, models::{
        DbEurovocTopicMinsistrialProposals, DbMinistrialProposal, DbMinistrialProposalDocument,
        DbOtherKeywordTopicMinistrialProposals, DbTopicMinistrialProposals,
    }, schema::{
        eurovoc_topics_ministrial_proposals, ministrial_proposals, ministrial_proposals_documents,
        other_keyword_topics_ministrial_proposals, topics_ministrial_proposals,
    }, services::scrape_untill_success, Shutdown
};

pub async fn scrape_new_ministrial_props(
    scraper: &AustriaScraper,
) -> crate::Result<Vec<MinistrialProposal>> {
    let proposal_payload = ProposalsPayload::default();
    let ministrial_proposals = scraper
        .post_ministrial_proposals_payload(proposal_payload)
        .await?;
    let ministrial_props_paths = scraper.ministrial_prop_paths(ministrial_proposals)?;

    let mut new_ministrial_props = Vec::with_capacity(ministrial_props_paths.len());

    for ministrial_prop_path in ministrial_props_paths {
        let Ok(ministrial_prop) = scraper
            .get_ministrial_proposal(ministrial_prop_path.clone())
            .await
        else {
            log::debug!("Cannot process ministrial proposal: '{ministrial_prop_path:?}'");
            continue;
        };

        new_ministrial_props.push(ministrial_prop);
    }

    Ok(new_ministrial_props)
}

async fn ministrial_proposal_once(only_latest: bool, pg_pool: &PgPool) -> Vec<MinistrialProposal> {
    log::info!("Try scraping ministiral proposals");
    let mut scraper = AustriaScraper::default();

    for gp in ["XXVII", "XXVI", "XXV", "XXIV", "XXIII", "XXII", "XXI", "XX"] {
        log::info!("scrape ministrial proposals for GP: {gp}");
        scraper.scrape_info.gp = vec![gp.into()];

        let new_ministrial_props = scrape_untill_success::<Vec<_>, _>(|| async {
            scrape_new_ministrial_props(&scraper).await
        })
        .await;

        update_ministrial_proposals_in_db2(pg_pool, &new_ministrial_props).await.unwrap();
        if only_latest {
            break;
        }
    }

    // new_legis_inits
    vec![]
}

use sqlx::postgres::PgPool;

pub async fn update_ministrial_proposals_in_db2(
    pool: &PgPool,
    ministrial_props: &[MinistrialProposal]
) -> Result<(), sqlx::Error> {
    log::info!("Updating ministrial proposal in database...");

    for ministrial_prop in ministrial_props {
        log::info!("inserting single ministrial prop");
        
        let start = Instant::now();
        // Insert or update the ministrial proposal
        let ministrial_prop_id = sqlx::query!(
            r#"
            INSERT INTO ministrial_proposals (
                title, inr, emphasis, description, ityp, gp, created_at, updated_at,
                due_to, ressort, ressort_shortform, legis_init_gp, legis_init_inr, legis_init_ityp
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            ON CONFLICT (inr, ityp, gp)
            DO UPDATE SET
                title = EXCLUDED.title,
                emphasis = EXCLUDED.emphasis,
                description = EXCLUDED.description,
                created_at = EXCLUDED.created_at,
                updated_at = EXCLUDED.updated_at,
                due_to = EXCLUDED.due_to,
                ressort = EXCLUDED.ressort,
                ressort_shortform = EXCLUDED.ressort_shortform,
                legis_init_gp = EXCLUDED.legis_init_gp,
                legis_init_inr = EXCLUDED.legis_init_inr,
                legis_init_ityp = EXCLUDED.legis_init_ityp
            RETURNING id
            "#,
            ministrial_prop.title,
            ministrial_prop.inr as i32,
            ministrial_prop.emphasis,
            ministrial_prop.description,
            ministrial_prop.ityp,
            ministrial_prop.gp,
            ministrial_prop.created_at.and_utc(),
            ministrial_prop.updated_at.map(|x| x.and_utc()),
            ministrial_prop.gegenstand_info.due_to,
            ministrial_prop.gegenstand_info.ressort,
            ministrial_prop.gegenstand_info.ressort_shortform,
            ministrial_prop.legis_gp,
            ministrial_prop.legis_init_inr.map(|x| x as i32),
            ministrial_prop.legis_init_ityp
        )
        .fetch_one(pool)
        .await?
        .id;
        log::info!("insertion dur: {:?}", start.elapsed());

        // Insert or update topics
        for topic in &ministrial_prop.topics {
            sqlx::query!(
                r#"
                INSERT INTO topics_ministrial_proposals (topic, ministrial_proposal_id)
                VALUES ($1, $2)
                ON CONFLICT (ministrial_proposal_id, topic)
                DO UPDATE SET topic = EXCLUDED.topic
                "#,
                topic,
                ministrial_prop_id
            )
            .execute(pool)
            .await?;
        }

        // Insert or update eurovoc topics
        for topic in &ministrial_prop.eurovoc_topics {
            sqlx::query!(
                r#"
                INSERT INTO eurovoc_topics_ministrial_proposals (topic, ministrial_proposal_id)
                VALUES ($1, $2)
                ON CONFLICT (ministrial_proposal_id, topic)
                DO UPDATE SET topic = EXCLUDED.topic
                "#,
                topic,
                ministrial_prop_id
            )
            .execute(pool)
            .await?;
        }

        // Insert or update other keyword topics
        for topic in &ministrial_prop.other_keyword_topics {
            sqlx::query!(
                r#"
                INSERT INTO other_keyword_topics_ministrial_proposals (topic, ministrial_proposal_id)
                VALUES ($1, $2)
                ON CONFLICT (ministrial_proposal_id, topic)
                DO UPDATE SET topic = EXCLUDED.topic
                "#,
                topic,
                ministrial_prop_id
            )
            .execute(pool)
            .await?;
        }

        // Insert or update documents
        for document in &ministrial_prop.documents {
            sqlx::query!(
                r#"
                INSERT INTO ministrial_proposals_documents (
                    ministrial_proposal_id, title, document_type, document_url
                )
                VALUES ($1, $2, $3, $4)
                ON CONFLICT (document_url, ministrial_proposal_id)
                DO UPDATE SET
                    title = EXCLUDED.title,
                    document_type = EXCLUDED.document_type
                "#,
                ministrial_prop_id,
                document.title,
                document.ftype.to_string(),
                document.link
            )
            .execute(pool)
            .await?;
        }
    }

    Ok(())
}

fn update_ministrial_proposals_in_db(ministrial_props: &[MinistrialProposal]) {
    log::info!("Updating ministiral proposal in database...");

    let conn = &mut crate::establish_connection();
    for ministiral_prop in ministrial_props {
        log::info!("inserting single ministrial prop");
        let db_ministrial_prop = DbMinistrialProposal {
            title: ministiral_prop.title.clone(),
            inr: ministiral_prop.inr as i32,
            emphasis: ministiral_prop.emphasis.clone(),
            description: ministiral_prop.description.clone(),
            ityp: ministiral_prop.ityp.clone(),
            gp: ministiral_prop.gp.clone(),
            created_at: ministiral_prop.created_at,
            updated_at: ministiral_prop.updated_at,
            due_to: ministiral_prop.gegenstand_info.due_to,
            ressort: ministiral_prop.gegenstand_info.ressort.clone(),
            ressort_shortform: ministiral_prop.gegenstand_info.ressort_shortform.clone(),
            legis_init_gp: ministiral_prop.legis_gp.clone(),
            legis_init_inr: ministiral_prop.legis_init_inr.map(|x| x as i32),
            legis_init_ityp: ministiral_prop.legis_init_ityp.clone(),
        };
        let start = Instant::now();
        let ministrial_prop_id = match diesel::insert_into(ministrial_proposals::table)
            .values(&db_ministrial_prop)
            .on_conflict((
                ministrial_proposals::inr,
                ministrial_proposals::ityp,
                ministrial_proposals::gp,
            ))
            .do_update()
            // .set((ministrial_proposals::created_at.eq(db_legis_init.created_at),))
            .set(&db_ministrial_prop)
            .returning(ministrial_proposals::id)
            .get_result(conn)
        {
            Ok(id) => id,
            Err(e) => {
                log::error!("Error inserting ministiral proposal: '{e}'. Error occured for ministiral proposal: {ministiral_prop:?}");
                continue;
            }
        };
        log::info!("insertion dur: {:?}", start.elapsed());
        for topic in ministiral_prop.topics.clone() {
            let topic = DbTopicMinistrialProposals {
                topic,
                ministrial_proposal_id: ministrial_prop_id,
            };

            if let Err(e) = diesel::insert_into(topics_ministrial_proposals::table)
                .values(&topic)
                .on_conflict((
                    topics_ministrial_proposals::ministrial_proposal_id,
                    topics_ministrial_proposals::topic,
                ))
                .do_update()
                .set(&topic)
                .execute(conn)
            {
                log::error!("Error inserting topic: '{e}'. The error occured for the following topic: {topic:?}")
            }
        }

        for topic in ministiral_prop.eurovoc_topics.clone() {
            let topic = DbEurovocTopicMinsistrialProposals {
                topic,
                ministrial_proposal_id: ministrial_prop_id,
            };

            if let Err(e) = diesel::insert_into(eurovoc_topics_ministrial_proposals::table)
                .values(&topic)
                .on_conflict((
                    eurovoc_topics_ministrial_proposals::ministrial_proposal_id,
                    eurovoc_topics_ministrial_proposals::topic,
                ))
                .do_update()
                .set(&topic)
                .execute(conn)
            {
                log::error!("Error inserting topic: '{e}'. The error occured for the following eurovoc topic: {topic:?}")
            }
        }

        for topic in ministiral_prop.other_keyword_topics.clone() {
            let topic = DbOtherKeywordTopicMinistrialProposals {
                topic,
                ministrial_proposal_id: ministrial_prop_id,
            };

            if let Err(e) = diesel::insert_into(other_keyword_topics_ministrial_proposals::table)
                .values(&topic)
                .on_conflict((
                    other_keyword_topics_ministrial_proposals::ministrial_proposal_id,
                    other_keyword_topics_ministrial_proposals::topic,
                ))
                .do_update()
                .set(&topic)
                .execute(conn)
            {
                log::error!("Error inserting topic: '{e}'. The error occured for the following other keyword topic: {topic:?}")
            }
        }

        for document in &ministiral_prop.documents {
            let legis_doc = DbMinistrialProposalDocument {
                ministrial_proposal_id: ministrial_prop_id,
                title: document.title.clone(),
                document_type: document.ftype.to_string(),
                document_url: document.link.clone(),
            };

            if let Err(e) = diesel::insert_into(ministrial_proposals_documents::table)
                .values(&legis_doc)
                .on_conflict((
                    ministrial_proposals_documents::document_url,
                    ministrial_proposals_documents::ministrial_proposal_id,
                ))
                .do_update()
                .set(&legis_doc)
                .execute(conn)
            {
                log::error!("Error inserting document: '{e}'. The error occured for the following document: {document:?}")
            }
        }
    }
}

pub async fn run_shut_latest_gp(mut shutdown: Shutdown) {
    let legis_init_sleep = std::env::var("MINISTRIAL_PROPOSAL_LATEST_SLEEP")
        .unwrap_or_else(|_| "50".to_string())
        .parse()
        .unwrap_or_else(|e| {
            log::warn!(
                "Could not parse MINISTRIAL_PROPOSAL_LATEST_SLEEP env var: {e}. Using default value of 50 seconds"
            );
            50
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
                    log::info!("Starting ministrial proposal (latest) scraper");
                    let pg_pool = connect_pg().await;
                    ministrial_proposal_once(true, &pg_pool).await;
                }).await {
                    log::error!("Error in ministrial proposal (latest) scraper: {:?}", e);
                }
                // legis_init_once().await;
                log::info!("ministrial proposal (latest) scraper service sleeps for {legis_init_sleep} seconds...");
                tokio::time::sleep(tokio::time::Duration::from_secs(legis_init_sleep)).await;
            } => {}
        }
    }
}

pub async fn run_shut_every_gp(mut shutdown: Shutdown) {
    let legis_init_sleep = std::env::var("MINISTRIAL_PROPOSAL_SLEEP")
        .unwrap_or_else(|_| "300".to_string())
        .parse()
        .unwrap_or_else(|e| {
            log::warn!(
                "Could not parse MINISTRIAL_PROPOSAL_SLEEP env var: {e}. Using default value of 1000 seconds"
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
                    log::info!("Starting ministrial proposal scraper");

                    let pg_pool = connect_pg().await;
                    ministrial_proposal_once(false, &pg_pool).await;
                }).await {
                    log::error!("Error in ministrial proposal scraper: {:?}", e);
                }
                // legis_init_once().await;
                log::info!("ministrial proposal scraper service sleeps for {legis_init_sleep} seconds...");
                tokio::time::sleep(tokio::time::Duration::from_secs(legis_init_sleep)).await;
            } => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDateTime;
    use scraper::AustriaScraper;

    #[test]
    fn test_date_time_parse() {
        let x = NaiveDateTime::parse_from_str("2014-5-17T12:34:56", "%Y-%m-%dT%H:%M:%S").unwrap();
        println!("x: {x:?}");
    }

    #[tokio::test]
    async fn test_legis_init_empty_gp_scrape() {
        let mut scraper: AustriaScraper = AustriaScraper::default();
        // println!("s: {:?}", scraper.scrape_info.gp);

        // let out = crate::legislative_initiatives::scrape_new_legis_inits(&scraper).await.unwrap();
        // println!("out: {out:?}");
    }
}
