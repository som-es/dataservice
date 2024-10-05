use somes_scraper::{
    ministrial_proposals::{MinistrialProposal, MinistrialProposalScraper},
    proposals::ProposalsPayload,
    AustriaScraper,
};
use tokio::time::Instant;

use crate::{connect_pg, services::scrape_untill_success, Shutdown};

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

        update_ministrial_proposals_in_db2(pg_pool, &new_ministrial_props)
            .await
            .unwrap();
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
    ministrial_props: &[MinistrialProposal],
) -> Result<(), sqlx::Error> {
    log::info!("Updating ministrial proposal in database...");

    for (i, ministrial_prop) in ministrial_props.iter().enumerate() {
        // log::info!("inserting single ministrial prop");

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
            ministrial_prop.created_at,
            ministrial_prop.updated_at,
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
        if i % 10 == 0 {
            log::info!("{i} insertion dur: {:?}", start.elapsed());
        }

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

pub async fn run_shut_latest_gp(mut shutdown: Shutdown) {
    let legis_init_sleep = std::env::var("MINISTRIAL_PROPOSAL_LATEST_SLEEP")
        .unwrap_or_else(|_| "400".to_string())
        .parse()
        .unwrap_or_else(|e| {
            log::warn!(
                "Could not parse MINISTRIAL_PROPOSAL_LATEST_SLEEP env var: {e}. Using default value of 400 seconds"
            );
            400
        });
    #[cfg(feature = "ministerial_latest")]
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
        .unwrap_or_else(|_| "900".to_string())
        .parse()
        .unwrap_or_else(|e| {
            log::warn!(
                "Could not parse MINISTRIAL_PROPOSAL_SLEEP env var: {e}. Using default value of 1000 seconds"
            );
            1000
        });

    #[cfg(feature = "ministerial")]
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
    use somes_scraper::AustriaScraper;

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
