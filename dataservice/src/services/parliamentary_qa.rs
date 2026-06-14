pub mod ai_tasks;
pub use ai_tasks::*;
pub mod inserts;
pub use inserts::*;
use std::sync::Arc;
mod patch_references;
use chrono::Days;
pub use patch_references::*;
use somes_scraper::{
    parliamentary_questions::ParliamentaryQAScraper, proposals::ProposalScraper, AustriaScraper,
    ParliamentaryQAEntity, QaType,
};
use sqlx::PgPool;
use tokio::{
    sync::{mpsc::Sender, Semaphore},
    time::Instant,
};

use crate::{
    ai_task::AiTask, enforce_ai_run, today_and_time, AppState, Shutdown, StoredUpdatedAt, GPS,
};

const LATEST_PQA_DAYS: u64 = 65;

async fn pqa_pipeline(
    app_state: AppState,
    ai_task_tx: Sender<AiTask>,
    only_latest: bool,
) -> crate::Result<()> {
    for gp in GPS {
        let mut scraper = AustriaScraper::default();
        scraper.scrape_info.gp = vec![gp.into()];

        if only_latest {
            scraper.scrape_info.date_start = today_and_time()
                .naive_local()
                .checked_sub_days(Days::new(LATEST_PQA_DAYS));
        }

        let scraper = Arc::new(scraper);

        let paths = app_state
            .parliament_api_limiter
            .run(scraper.post_parliamentary_qa_payload(true))
            .await?;
        let wait = Arc::new(Semaphore::new(16));
        let mut handles = vec![];
        for path in scraper.proposal_paths(paths)? {
            let scraper = scraper.clone();
            let app_state = app_state.clone();
            let wait = wait.clone();
            let ai_task_tx = ai_task_tx.clone();
            handles.push(tokio::task::spawn(async move {
                let Ok(qa) = app_state
                    .parliament_api_limiter
                    .run(scraper.get_parliamentary_qa(&path))
                    .await
                else {
                    log::error!("could not fetch parliamentary qa entry");
                    return Ok(());
                };

                let permit = wait.acquire().await.unwrap();

                if qa.qa_type == QaType::Question {
                    if let Err(e) =
                        handle_question(app_state, ai_task_tx, qa, &scraper, only_latest).await
                    {
                        log::error!("could not await handle (inner error): {e:?}");
                    }
                }

                drop(permit);
                crate::Result::Ok(())
            }));
        }

        for handle in handles {
            match handle.await {
                Ok(info) => {
                    if let Err(e) = info {
                        log::error!("could not await handle (inner error): {e:?}");
                    }
                }
                Err(e) => {
                    log::error!("could not await handle (panic?): {e:?}");
                }
            }
        }
        if only_latest {
            break;
        }
    }
    Ok(())
}

impl StoredUpdatedAt for ParliamentaryQAEntity {
    async fn stored_updated_at(&self, pg: &PgPool) -> Option<chrono::DateTime<chrono::Utc>> {
        sqlx::query!(
            "select raw_data_updated_at from pqa_meta where inr = $1 and gp = $2 and ityp = $3",
            self.inr as i32,
            self.gp,
            self.ityp
        )
        .fetch_optional(pg)
        .await
        .ok()
        .flatten()
        .map(|x| x.raw_data_updated_at)
        .flatten()
    }
}

async fn handle_question(
    app_state: AppState,
    ai_task_tx: Sender<AiTask>,
    question: ParliamentaryQAEntity,
    scraper: &AustriaScraper,
    is_latest: bool,
) -> crate::Result<()> {
    let should_update = crate::should_update_by_time(&question, &app_state.pg_pool).await;

    if !enforce_ai_run() && !should_update {
        return Ok(());
    }

    let (question_id, answers) =
        insert_qa_and_collect_answers(app_state, &question, scraper).await?;

    if is_latest {
        ai_task_tx
            .send(AiTask {
                input: None,
                priority: crate::ai_task::Priority::Medium,
                score: crate::ai_task::Priority::Medium.weight(),
                task_id: crate::hash!(),
                db_entry_id: question_id as i32,
                rerunnable: true,
                created_at: Instant::now(),
                task: Box::new(move |task_args| {
                    Box::pin(async move {
                        summarize_and_extract_qa(task_args, &question, question_id, &answers).await
                    })
                }),
            })
            .await?;
    }
    Ok(())
}

async fn insert_qa_and_collect_answers(
    app_state: AppState,
    question: &ParliamentaryQAEntity,
    scraper: &AustriaScraper,
) -> crate::Result<(i64, Vec<(i64, ParliamentaryQAEntity)>)> {
    let question_id = insert_pqa(&app_state.pg_pool, question).await?;
    let mut answers = vec![];
    for reference in question
        .references
        .iter()
        .filter(|reference| reference.qa_type == QaType::Answer)
    {
        let answer = app_state
            .parliament_api_limiter
            .run(scraper.get_parliamentary_qa(&format!(
                "/gegenstand/{}/{}/{}",
                reference.reference.gp, reference.reference.ityp, reference.reference.inr
            )))
            .await?;

        answers.push((insert_pqa(&app_state.pg_pool, &answer).await?, answer));
    }
    Ok((question_id, answers))
}

pub async fn run_shut_latest_gp(
    mut shutdown: Shutdown,
    app_state: AppState,
    ai_task_tx: Sender<AiTask>,
    only_latest: bool,
) {
    let pqa_sleep =
    std::env::var("PARLIAMENTARY_QA_SLEEP")
            .unwrap_or_else(|_| "400000".to_string())
            .parse()
            .unwrap_or_else(|e| {
                log::warn!(
                    "Could not parse POLITICAL_ANALYSIS_SLEEP env var: {e}. Using default value of 400000 seconds"
                );
                400000
            });
    let pqa_latest_sleep =
        std::env::var("PARLIAMENTARY_QA_SLEEP_LATEST")
                .unwrap_or_else(|_| "800".to_string())
                .parse()
                .unwrap_or_else(|e| {
                    log::warn!(
                        "Could not parse POLITICAL_ANALYSIS_SLEEP_LATEST env var: {e}. Using default value of 800 seconds"
                    );
                    800
                });

    let sleep = if only_latest {
        pqa_latest_sleep
    } else {
        pqa_sleep
    };

    while !shutdown.is_shutdown() {
        let ai_task_tx = ai_task_tx.clone();
        tokio::select! {
            _ = shutdown.recv() => {
                println!("shutting down");
                break;
            }
            _ = async {
                let app_state = app_state.clone();
                if let Err(e) = tokio::task::spawn(async move {
                    log::info!("Starting parliamentary qa service");

                    pqa_pipeline(app_state, ai_task_tx, only_latest).await
                }).await {
                    log::error!("Error in parliamentary qa service: {:?}", e);
                }
                log::info!("parliamentary qa service sleeps for {sleep} seconds...");
                tokio::time::sleep(tokio::time::Duration::from_secs(sleep)).await;
            } => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use somes_scraper::{parliamentary_questions::ParliamentaryQAScraper, AustriaScraper};

    #[tokio::test]
    async fn test_xxviii_j_5810() {
        let scraper = AustriaScraper::default();
        // XXVIII/J/5810
        let pqa = scraper
            .get_parliamentary_qa("/gegenstand/XXVIII/J/5810")
            .await
            .unwrap();
        // pqa
    }
}
