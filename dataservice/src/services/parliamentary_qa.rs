pub mod ai_tasks;
pub use ai_tasks::*;
pub mod inserts;
pub use inserts::*;
use std::sync::Arc;
mod patch_references;
use chrono::Days;
use combx::{AbortReason, DbQuestionEntry};
pub use patch_references::*;
use somes_scraper::{
    parliamentary_questions::ParliamentaryQAScraper, proposals::ProposalScraper, AustriaScraper,
    ParliamentaryQAEntity, QaType,
};
use sqlx::{types::Json, PgPool};
use tokio::{
    sync::{mpsc::Sender, Semaphore},
    time::Instant,
};

use crate::{ai_task::AiTask, enforce_ai_run, today_and_time, AppState, Shutdown, StoredUpdatedAt};

async fn pqa_pipeline(
    app_state: AppState,
    ai_task_tx: Sender<AiTask>,
    only_latest: bool,
) -> crate::Result<()> {
    let mut scraper = AustriaScraper::default();

    if only_latest {
        scraper.scrape_info.date_start = today_and_time()
            .naive_local()
            .checked_sub_days(Days::new(30));
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
                if let Err(e) = handle_question(app_state, ai_task_tx, qa, &scraper).await {
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
) -> crate::Result<()> {
    let should_update = crate::should_update_by_time(&question, &app_state.pg_pool).await;

    if !enforce_ai_run() && !should_update {
        return Ok(());
    }

    let question_id = insert_pqa(&app_state.pg_pool, &question).await?;

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
                    use combx::QuestionEntry;
                    let question_entry = sqlx::query_as!(
                        DbQuestionEntry,
                        r#"
                        select
                            id,
                            pqa_meta_id,
                            short_title,
                            once_sentence_short_info_summary,
                            very_short_info_summary,
                            short_info_summary,
                            info_summary,
                            detailed_info_summary,
                            question_summary,
                            detailed_question_summary,
                            full_question_entry as "full_question_entry: Json<QuestionEntry>",
                            model_used,
                            version,
                            generated_at
                        from pqa_questions
                        where id = $1
                        "#,
                        question_id
                    )
                    .fetch_optional(&task_args.pg_pool)
                    .await?;

                    let unique_topics = sqlx::query_scalar!(
                        "select topic_name as topic from unique_eurovoc_topics order by topic"
                    )
                    .fetch_all(&task_args.pg_pool)
                    .await?;

                    let questions = match question_entry {
                        Some(entry) => entry.full_question_entry.questions.clone(),
                        None => {
                            let extracted_question = extract_question(
                                &task_args.som_ai_client,
                                &unique_topics,
                                &question
                                    .documents
                                    .iter()
                                    .map(|doc| somes_common_lib::Document {
                                        title: Some(doc.title.clone()),
                                        document_url: Some(doc.link.clone()),
                                        document_type: Some(doc.ftype.to_string()),
                                    })
                                    .collect::<Vec<_>>(),
                            )
                            .await?;
                            if extracted_question.abort_reason != AbortReason::None || extracted_question.question_entry.is_none() {
                                log::error!("(question) No documents? provided in context for question {}, documents: {:?}, abort reason: {:?}, skipping", question_id, question.documents, extracted_question.abort_reason);
                                Err(combx::Error::Custom("No documents provided in context for question, skipping".to_string()))?;
                            }
                            let extracted_question = extracted_question.question_entry.unwrap();

                            insert_question_entry(
                                &task_args.pg_pool,
                                question_id,
                                &extracted_question,
                                &task_args.som_ai_client.get_model().await?.result,
                                "1",
                            )
                            .await?;
                            extracted_question.questions
                        }
                    };

                    let mut extracted_answers = vec![];
                    for (answer_id, answer) in answers {
                        if sqlx::query_scalar!(
                            "select id from pqa_answers where pqa_meta_id = $1",
                            answer_id
                        )
                        .fetch_optional(&task_args.pg_pool)
                        .await?
                        .is_some()
                        {
                            continue;
                        }
                        let extracted_answer = extract_answer(
                            &task_args.som_ai_client,
                            &unique_topics,
                            &answer
                                .documents
                                .iter()
                                .map(|doc| somes_common_lib::Document {
                                    title: Some(doc.title.clone()),
                                    document_url: Some(doc.link.clone()),
                                    document_type: Some(doc.ftype.to_string()),
                                })
                                .collect::<Vec<_>>(),
                            &questions,
                        )
                        .await?;

                        if extracted_answer.abort_reason != AbortReason::None || extracted_answer.answer_entry.is_none() {
                            log::error!("(answer) No documents? provided in context for answer {}, documents: {:?}, abort reason: {:?}, skipping", answer_id, answer.documents, extracted_answer.abort_reason);
                            continue
                        }
                        let extracted_answer = extracted_answer.answer_entry.unwrap();
                        insert_answer_entry(
                            &task_args.pg_pool,
                            answer_id,
                            &extracted_answer,
                            &task_args.som_ai_client.get_model().await?.result,
                            "1",
                        )
                        .await?;
                        extracted_answers.push(extracted_answer);
                    }

                    //
                    Ok(())
                })
            }),
        })
        .await?;
    Ok(())
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
