mod delegate_context;
mod embed_speeches;
mod stance_system;
mod upload_questions;

use std::sync::Arc;

use chrono::{Months, TimeDelta};
use qdrant_client::Qdrant;
use somai_client::SomAiClient;
use sqlx::PgPool;
use tokio::sync::mpsc::Sender;

use crate::{
    ai_task::AiTask,
    political_analysis::{
        embed_speeches::{get_delegate_ids, process_delegate},
        stance_system::{rag_pipeline, Stance},
    },
    today, Result, Shutdown,
};

pub(super) trait EmbeddingProvider: Send + Sync {
    /// Generates an embedding for a given chunk of text
    async fn embed_text(&self, text: &str) -> Result<Vec<f64>>;
    /// Returns the dimension of the embeddings produced
    async fn embedding_dim(&self) -> Result<u64>;
}

impl EmbeddingProvider for SomAiClient {
    async fn embed_text(&self, text: &str) -> Result<Vec<f64>> {
        Ok(self
            .embedding_embed(text.into(), None)
            .await
            .map(|out| out.embedding)?)
    }

    async fn embedding_dim(&self) -> Result<u64> {
        Ok(self.embed_text("This is a test").await?.len() as u64)
    }
}

struct Question {
    id: i32,
    question: String,
    is_left: Option<bool>,
    is_liberal: Option<bool>,
}

async fn political_analysis(pg_pool: &PgPool, ai_task_tx: Sender<AiTask>) -> Result<()> {
    let ai_client = Arc::new(
        SomAiClient::new(
            &std::env::var("SOMAI_URL").unwrap_or_else(|_| "http://127.0.0.1:8000".to_string()),
            "API_USER",
            &std::env::var("SOMAI_API_SECRET").unwrap(),
        )
        .await?,
    );

    let used_llm = ai_client.get_model().await?.result;

    let qdrant_client = Qdrant::from_url(
        &std::env::var("QDRANT_URL").unwrap_or_else(|_| "http://127.0.0.1:6334".to_string()),
    )
    .build()?;

    let vector_update_skip = std::env::var("VECTOR_UPDATE_SKIP")
        .unwrap_or("false".into())
        .parse::<bool>()
        .unwrap_or_default();

    let delegates = get_delegate_ids(&pg_pool).await?;
    let questions = sqlx::query_as!(
        Question,
        "SELECT id, question, is_liberal, is_left FROM political_questions"
    )
    .fetch_all(pg_pool)
    .await?;

    let today = today().to_utc();
    for delegate in delegates {
        let last_updated_at = sqlx::query_scalar!(
            "select updated_at from last_vector_speech_update where delegate_id = $1",
            delegate.id
        )
        .fetch_one(pg_pool)
        .await;

        let Ok(last_updated_at) = last_updated_at else {
            log::error!(
                "could not fetch last_updated for {delegate:?} because of: {last_updated_at:?}"
            );
            continue;
        };

        if !vector_update_skip && today - last_updated_at < TimeDelta::days(30 * 3) {
            continue;
        }

        if let Err(e) = embed_speeches::process_delegate(
            pg_pool,
            &qdrant_client,
            &*ai_client,
            "delegates",
            &delegate,
            !vector_update_skip,
        )
        .await
        {
            log::error!("Failed processing delegate {}: {:?}", delegate.id, e);
        }

        for question in &questions {
            let ai_client = ai_client.clone();
            let qdrant_client = qdrant_client.clone();
            let question_text = question.question.clone();
            let question_id = question.id;
            let is_left = question.is_left;
            let is_liberal = question.is_liberal;
            let delegate_id = delegate.id;
            let used_llm = used_llm.clone();

            let ai_task = AiTask {
                input: None,
                priority: crate::ai_task::Priority::Medium,
                task_id: crate::hash!(),
                db_entry_id: 0,
                rerunnable: true,
                task: Box::new(move |task_args| {
                    Box::pin(async move {
                        let stance = rag_pipeline::<Stance>(
                            &ai_client,
                            &qdrant_client,
                            delegate_id,
                            &question_text,
                        )
                        .await?;

                        let stance_llm = match stance.stance_detection {
                            stance_system::StanceDetection::Postive => "Positive",
                            stance_system::StanceDetection::Neutral => "Neutral",
                            stance_system::StanceDetection::Negative => "Negative",
                        };

                        let answer_id = sqlx::query_scalar!("insert into political_answers(question_id, delegate_id, answer, stance_llm, model_used)
                            values ($1, $2, $3, $4, $5) returning id
                        ", question_id, delegate_id, stance.political_stance, stance_llm, used_llm).fetch_one(&task_args.pg_pool).await?;

                        let ref_answers = sqlx::query!("select stance_llm, answer, question_id from political_answers pa where question_id = $1 and is_strong_reference order by stance_llm;", question_id).fetch_all(&task_args.pg_pool).await?;

                        let negative_ref = &ref_answers[0].answer;
                        let positive_ref = &ref_answers[1].answer;

                        let pro_sim = ai_client
                            .embedding_embed_and_cosine(
                                positive_ref.to_string(),
                                stance.political_stance.to_string(),
                            )
                            .await?
                            .cosine_similarity;

                        let contra_sim = ai_client
                            .embedding_embed_and_cosine(
                                negative_ref.to_string(),
                                stance.political_stance.to_string(),
                            )
                            .await?
                            .cosine_similarity;
                        let score = 10.0 * (pro_sim - contra_sim).powi(2);

                        sqlx::query!("insert into political_opinions (delegate_id, question_id, answer_id, stance, pro_strong_ref_score, contra_strong_ref_score, ref_score) values ($1, $2, $3, $4, $5, $6, $7)",
                                    delegate_id, question_id, answer_id, stance_llm, pro_sim, contra_sim, score).execute(&task_args.pg_pool).await?;
                        Ok(())
                    })
                }),
            };
            if let Err(e) = ai_task_tx.send(ai_task).await {
                log::error!("Cannot send ai task: {e:?}")
            }
        }
    }

    Ok(())
}

pub async fn run_shut(mut shutdown: Shutdown, pg_pool: PgPool, ai_task_tx: Sender<AiTask>) {
    let political_analysis_sleep = std::env::var("POLITICAL_ANALYSIS_SLEEP")
        .unwrap_or_else(|_| "800".to_string())
        .parse()
        .unwrap_or_else(|e| {
            log::warn!(
                "Could not parse POLITICAL_ANALYSIS_SLEEP env var: {e}. Using default value of 800 seconds"
            );
            800
        });

    while !shutdown.is_shutdown() {
        tokio::select! {
            _ = shutdown.recv() => {
                println!("shutting down");
                // return ok?
                break;
            }
            _ = async {
                let pg_pool = pg_pool.clone();
                log::info!("Starting political analysis service");
                if let Err(e) = political_analysis(&pg_pool, ai_task_tx.clone()).await {
                    log::error!("could not complete political analysis run: {e:?}")
                }
                // plenar_info_once(&pg_pool, true).await;
                log::info!("political analysis service sleeps for {political_analysis_sleep} seconds...");
                tokio::time::sleep(tokio::time::Duration::from_secs(political_analysis_sleep)).await;
            } => {}
        }
    }
}
