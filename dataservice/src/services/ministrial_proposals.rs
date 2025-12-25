use std::{
    collections::HashMap,
    sync::{Arc, LazyLock},
};

use chrono::Months;
use combx::{create_gov_officials_map, FromScrapedType, GovProposal};
use dotenvy_macro::dotenv;
use somes_scraper::{
    ministrial_proposals::{GegenstandInfo, MinistrialProposal, MinistrialProposalScraper},
    proposals::ProposalsPayload,
    AustriaScraper, GovOfficial,
};
use tokio::{sync::mpsc::Sender, time::Instant};

use crate::{
    ai_task::AiTask, services::scrape_untill_success, should_update::StoredUpdatedAt, today,
    today_and_time, Shutdown, GPS,
};

mod notify;
pub use notify::*;

mod ai_tasks;
pub use ai_tasks::*;

mod inserts;
pub use inserts::*;

pub async fn scrape_new_ministrial_props(
    scraper: &AustriaScraper,
) -> crate::Result<Vec<MinistrialProposal>> {
    let mut proposal_payload = ProposalsPayload::default();
    proposal_payload.scrape_info = scraper.scrape_info.clone();
    let ministrial_proposals = scraper
        .post_ministrial_proposals_payload(proposal_payload)
        .await?;
    let mut ministrial_props_paths = scraper.ministrial_prop_paths(ministrial_proposals)?;
    sort_ministerial_prop_paths_desc(&mut ministrial_props_paths);

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

pub async fn ministrial_proposal_once(
    only_latest: bool,
    pg_pool: &PgPool,
    new_ministrial_proposal_tx: Option<Sender<Option<(i32, GovProposal)>>>,
    ai_task_tx: Sender<AiTask>,
) -> Vec<MinistrialProposal> {
    log::info!("Try scraping ministiral proposals");
    let mut scraper = AustriaScraper::default();

    let gov_officials = Arc::new(create_gov_officials_map().await);

    for gp in GPS {
        log::info!("scrape ministrial proposals for GP: {gp}");
        scraper.scrape_info.gp = vec![gp.into()];
        if only_latest {
            // this seems to not work! - Minsterial Props and NRSITZ do not have date filtering???
            scraper.scrape_info.date_start = today_and_time().checked_sub_months(Months::new(3))
        }

        let new_ministrial_props = scrape_untill_success::<Vec<_>, _>(|| async {
            scrape_new_ministrial_props(&scraper).await
        })
        .await;

        log::info!(
            "scraped {} new ministrial props",
            new_ministrial_props.len()
        );

        update_ministrial_proposals_in_db2(
            pg_pool,
            new_ministrial_props,
            new_ministrial_proposal_tx.clone(),
            gov_officials.clone(),
            Some(ai_task_tx.clone()),
        )
        .await
        .unwrap();
        if only_latest {
            break;
        }
        scraper.scrape_info.date_start = None;
    }

    // new_legis_inits
    vec![]
}

const MINISTRIAL_PROP_STILL_SEND_DAYS: LazyLock<i64> = LazyLock::new(|| {
    dotenv!("LEGIS_INIT_STILL_SEND_DAYS")
        .parse::<i64>()
        .unwrap_or(10)
});

use sqlx::{postgres::PgPool, query};

impl StoredUpdatedAt for MinistrialProposal {
    async fn stored_updated_at(&self, pg: &PgPool) -> Option<chrono::NaiveDateTime> {
        query!(
            "select updated_at from ministrial_proposals where gp = $1 and ityp = $2 and inr = $3",
            self.gp,
            self.ityp,
            self.inr as i32
        )
        .fetch_optional(pg)
        .await
        .ok()
        .flatten()
        .map(|x| x.updated_at)
        .flatten()
    }
}

pub async fn update_ministrial_proposals_in_db2(
    pool: &PgPool,
    ministrial_props: Vec<MinistrialProposal>,
    new_ministrial_proposal_tx: Option<Sender<Option<(i32, GovProposal)>>>,
    gov_officials: Arc<HashMap<u64, Vec<GovOfficial>>>,
    ai_task_tx: Option<Sender<AiTask>>,
) -> Result<(), sqlx::Error> {
    log::info!("Updating ministrial proposal in database...");

    let mut started_acc = false;
    for (i, ministrial_prop) in ministrial_props.into_iter().enumerate() {
        if !crate::should_update(&ministrial_prop, pool).await {
            continue;
        }

        let gov_proposal =
            GovProposal::from_scraped_type((), (ministrial_prop, gov_officials.clone())).await;
        // log::info!("inserting single ministrial prop");

        let start = Instant::now();
        // Insert or update the ministrial proposal
        let ministrial_prop_id = sqlx::query!(
            r#"
            INSERT INTO ministrial_proposals (
                title, inr, emphasis, description, ityp, gp, created_at, updated_at,
                due_to, ressort, ressort_shortform, legis_init_gp, legis_init_inr, legis_init_ityp, has_vote_result
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
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
                legis_init_ityp = EXCLUDED.legis_init_ityp,
                has_vote_result = EXCLUDED.has_vote_result
            RETURNING id
            "#,
            gov_proposal.ministrial_proposal.title,
            gov_proposal.ministrial_proposal.inr,
            gov_proposal.ministrial_proposal.emphasis,
            gov_proposal.ministrial_proposal.description,
            gov_proposal.ministrial_proposal.ityp,
            gov_proposal.ministrial_proposal.gp,
            gov_proposal.ministrial_proposal.created_at,
            gov_proposal.ministrial_proposal.updated_at,
            gov_proposal.ministrial_proposal.due_to,
            gov_proposal.ministrial_proposal.ressort,
            gov_proposal.ministrial_proposal.ressort_shortform,
            gov_proposal.ministrial_proposal.legis_init_gp,
            gov_proposal.ministrial_proposal.legis_init_inr,
            gov_proposal.ministrial_proposal.legis_init_ityp,
            gov_proposal.ministrial_proposal.has_vote_result,
        )
        .fetch_one(pool)
        .await?
        .id;
        if i % 10 == 0 {
            log::info!("{i} insertion dur: {:?}", start.elapsed());
        }

        for issuer_id in &gov_proposal.ministerial_issuers {
            sqlx::query!(
                r#"
                INSERT INTO ministrial_issuer (delegate_id, ministrial_proposal_id)
                VALUES ($1, $2)
                ON CONFLICT (ministrial_proposal_id, delegate_id)
                DO NOTHING
                "#,
                *issuer_id,
                ministrial_prop_id
            )
            .execute(pool)
            .await?;
        }

        if let Some(new_legis_init_tx) = &new_ministrial_proposal_tx {
            if let Ok(info) = sqlx::query!(
                "select * from ministrial_email_info where already_sent_id = $1",
                ministrial_prop_id
            )
            .fetch_optional(pool)
            .await
            {
                let diff = today() - gov_proposal.ministrial_proposal.created_at.date();

                // this works, because the specific ministerial prop paths are sorted desc by inr
                const TOTAL_MINISTERIAL_PROP_SENT: usize = 30;
                if diff.num_days() <= *MINISTRIAL_PROP_STILL_SEND_DAYS
                    && info.is_none()
                    && i < TOTAL_MINISTERIAL_PROP_SENT
                {
                    log::info!("Sending fitting ministrial prop: {ministrial_prop_id}");
                    if let Err(e) = new_legis_init_tx
                        .send(Some((ministrial_prop_id, gov_proposal.clone())))
                        .await
                    {
                        log::error!("could not send ministrial prop to mail notify service: {e:?}")
                    } else {
                        started_acc = true;
                    }

                    if let Err(e) = sqlx::query!(
                        "insert into ministrial_email_info (already_sent_id) values ($1)",
                        ministrial_prop_id
                    )
                    .execute(pool)
                    .await
                    {
                        log::error!(
                            "could not insert into ministrial email info for id {ministrial_prop_id}: {e:?}"
                        );
                    }
                } else if started_acc {
                    started_acc = false;

                    // early triggering / stop accumulating
                    if let Err(e) = new_legis_init_tx.send(None).await {
                        log::error!("could not trigger mail sending service: {e:?}")
                    }
                }
            };
        }

        // Insert or update topics
        for topic in &gov_proposal.topics {
            sqlx::query!(
                r#"
                INSERT INTO topics_ministrial_proposals (topic, ministrial_proposal_id)
                VALUES ($1, $2)
                ON CONFLICT (ministrial_proposal_id, topic)
                DO UPDATE SET topic = EXCLUDED.topic
                "#,
                topic.topic,
                ministrial_prop_id
            )
            .execute(pool)
            .await?;
        }

        // Insert or update eurovoc topics
        for topic in &gov_proposal.eurovoc_topics {
            sqlx::query!(
                r#"
                INSERT INTO eurovoc_topics_ministrial_proposals (topic, ministrial_proposal_id)
                VALUES ($1, $2)
                ON CONFLICT (ministrial_proposal_id, topic)
                DO UPDATE SET topic = EXCLUDED.topic
                "#,
                topic.topic,
                ministrial_prop_id
            )
            .execute(pool)
            .await?;
        }

        // Insert or update other keyword topics
        for topic in &gov_proposal.other_keyword_topics {
            sqlx::query!(
                r#"
                INSERT INTO other_keyword_topics_ministrial_proposals (topic, ministrial_proposal_id)
                VALUES ($1, $2)
                ON CONFLICT (ministrial_proposal_id, topic)
                DO UPDATE SET topic = EXCLUDED.topic
                "#,
                topic.topic,
                ministrial_prop_id
            )
            .execute(pool)
            .await?;
        }

        let mut tx = pool.begin().await?;
        // Insert or update documents
        if let Err(e) = insert_ministerial_documents_batch(
            &mut tx,
            &gov_proposal.documents,
            ministrial_prop_id,
        )
        .await
        {
            log::error!(
                "Error inserting documents for ministerial prop {ministrial_prop_id}: {e:?}"
            );
        }
        tx.commit().await?;

        #[cfg(feature = "ai")]
        if let Some(ai_task_tx) = &ai_task_tx {
            use crate::{ai_task::Priority, LATEST_GP};

            let priority = if gov_proposal.ministrial_proposal.created_at.date()
                >= today() - chrono::Duration::days(90)
            {
                Priority::Highest
            } else if gov_proposal.ministrial_proposal.gp == LATEST_GP {
                Priority::High
            } else {
                Priority::Lowest
            };

            let documents = gov_proposal.documents.clone();
            let summarization_task = AiTask {
                input: None,
                priority,
                task_id: crate::hash!(),
                db_entry_id: ministrial_prop_id,
                task: Box::new(move |task_args| {
                    Box::pin(async move {
                        summarize_ministerial_proposal_documents(task_args, &documents).await
                        // TODO: update meilisearch index on finish
                    })
                }),
            };

            if let Err(e) = ai_task_tx.send(summarization_task).await {
                log::error!("Could not send AI task for ministerial prop summarization: {e:?}");
            }
        }
    }

    // without this, the mail sending service wouldn't be triggered if e.g. gov proposals list length  is lower than TOTAL_MINISTERIAL_PROP_SENT threshold
    if started_acc {
        if let Some(new_legis_init_tx) = &new_ministrial_proposal_tx {
            if let Err(e) = new_legis_init_tx.send(None).await {
                log::error!("could not trigger mail sending service: {e:?}")
            }
        }
    }

    Ok(())
}

pub async fn run_shut_latest_gp(
    mut shutdown: Shutdown,
    pg_pool: PgPool,
    new_ministrial_proposal_tx: Sender<Option<(i32, GovProposal)>>,
    ai_task_tx: Sender<AiTask>,
) {
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
        let new_ministrial_proposal_tx = new_ministrial_proposal_tx.clone();
        let ai_task_tx = ai_task_tx.clone();
        tokio::select! {
            _ = shutdown.recv() => {
                println!("shutting down");
                // return ok?
                break;
            }
            _ = async {
                let pg_pool = pg_pool.clone();
                if let Err(e) = tokio::task::spawn(async move {
                    log::info!("Starting ministrial proposal (latest) scraper");
                    ministrial_proposal_once(true, &pg_pool, Some(new_ministrial_proposal_tx), ai_task_tx).await;
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

pub async fn run_shut_every_gp(
    mut shutdown: Shutdown,
    pg_pool: PgPool,
    only_latest: bool,
    ai_task_tx: Sender<AiTask>,
) {
    if only_latest {
        return;
    }
    let legis_init_sleep = std::env::var("MINISTRIAL_PROPOSAL_SLEEP")
        .unwrap_or_else(|_| "10000".to_string())
        .parse()
        .unwrap_or_else(|e| {
            log::warn!(
                "Could not parse MINISTRIAL_PROPOSAL_SLEEP env var: {e}. Using default value of 10000 seconds"
            );
            10000
        });

    #[cfg(feature = "ministerial")]
    while !shutdown.is_shutdown() {
        let ai_task_tx = ai_task_tx.clone();
        tokio::select! {
            _ = shutdown.recv() => {
                println!("shutting down");
                // return ok?
                break;
            }
            _ = async {
                let pg_pool = pg_pool.clone();
                if let Err(e) = tokio::task::spawn(async move {
                    log::info!("Starting ministrial proposal scraper");

                    ministrial_proposal_once(false, &pg_pool, None, ai_task_tx).await;
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

fn convert_inr_from_path_to_num(path: &str) -> Option<usize> {
    let start = path.rfind('/')?;
    path[start + 1..].parse().ok()
}

fn sort_ministerial_prop_paths_desc(prop_paths: &mut Vec<GegenstandInfo>) {
    prop_paths.sort_by(|a, b| {
        convert_inr_from_path_to_num(&b.path)
            .unwrap()
            .cmp(&convert_inr_from_path_to_num(&a.path).unwrap())
    });
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDateTime;
    use somes_scraper::{ministrial_proposals::GegenstandInfo, AustriaScraper};

    use crate::{ministrial_proposals::sort_ministerial_prop_paths_desc, today};

    #[test]
    fn test_sort_ministerial_paths() {
        let mut paths = vec![
            GegenstandInfo {
                path: "/gegenstand/XXVIII/ME/1".into(),
                ressort: "wenig".into(),
                ressort_shortform: "nix".into(),
                due_to: today(),
            },
            GegenstandInfo {
                path: "/gegenstand/XXVIII/ME/15".into(),
                ressort: "wenig".into(),
                ressort_shortform: "nix".into(),
                due_to: today(),
            },
            GegenstandInfo {
                path: "/gegenstand/XXVIII/ME/32".into(),
                ressort: "wenig".into(),
                ressort_shortform: "nix".into(),
                due_to: today(),
            },
            GegenstandInfo {
                path: "/gegenstand/XXVIII/ME/101".into(),
                ressort: "wenig".into(),
                ressort_shortform: "nix".into(),
                due_to: today(),
            },
            GegenstandInfo {
                path: "/gegenstand/XXVIII/ME/10".into(),
                ressort: "wenig".into(),
                ressort_shortform: "nix".into(),
                due_to: today(),
            },
            GegenstandInfo {
                path: "/gegenstand/XXVIII/ME/40".into(),
                ressort: "wenig".into(),
                ressort_shortform: "nix".into(),
                due_to: today(),
            },
        ];
        sort_ministerial_prop_paths_desc(&mut paths);
        dbg!(&paths);
    }

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
