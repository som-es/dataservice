mod answers;
mod config;
pub mod delegate_context;
mod embed_speeches;
mod stance_system;

use chrono::{DateTime, Utc};
use combx::{OrientationQuestion, Parliament};
use qdrant_client::Qdrant;
use somai_client::SomAiClient;
use somes_scraper::language::Language;
use std::{collections::HashSet, sync::Arc};
use tokio::{sync::mpsc::Sender, time::Instant};

use crate::{
    AppState, Result, Shutdown,
    ai_task::AiTask,
    clients::{connect_ai_client, connect_qdrant_client},
    political_analysis::{
        answers::{process_delegate_stance, recently_answered_pairs},
        config::{is_fresh, sleep_seconds, stance_update_skip, update_days, vector_update_skip},
        embed_speeches::{Delegate, get_delegate_ids, setup_qdrant_collection},
    },
    today,
};

pub trait EmbeddingProvider: Send + Sync {
    /// Generates an embedding for a given chunk of text
    async fn embed_text(&self, text: &str) -> Result<Vec<f64>>;
    /// Returns the dimension of the embeddings produced
    async fn embedding_dim(&self) -> Result<u64>;
    async fn model(&self) -> Result<String>;
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

    async fn model(&self) -> Result<String> {
        Ok(self.get_default_embedding_model_name().await?.result)
    }
}

pub fn collection_name(parliament: Parliament) -> &'static str {
    match parliament {
        Parliament::At => "at_delegates",
        Parliament::Eu => "eu_delegates",
    }
}

async fn political_analysis(
    app_state: AppState,
    ai_task_tx: &Sender<AiTask>,
    ai_client: Arc<SomAiClient>,
    qdrant_client: &Qdrant,
) -> Result<()> {
    let pg_pool = &app_state.pg_pool;
    let used_llm = ai_client.get_model().await?.result;
    log::info!(
        "Political analysis of {:?} uses llm {used_llm}",
        app_state.parliament
    );

    setup_qdrant_collection(
        &qdrant_client,
        collection_name(app_state.parliament),
        ai_client.embedding_dim().await?,
    )
    .await?;

    let vector_update_skip = vector_update_skip();
    let stance_update_skip = stance_update_skip();

    let delegates = get_delegate_ids(pg_pool).await?;
    let questions = sqlx::query_as!(
        OrientationQuestion,
        "SELECT id, question, is_liberal, is_left, is_part_of FROM political_questions"
    )
    .fetch_all(pg_pool)
    .await?;

    let mut entry_id = 0;
    let today = today().to_utc();

    let mut to_analyse: Vec<Delegate> = if !stance_update_skip && vector_update_skip {
        delegates.clone()
    } else {
        vec![]
    };
    if !vector_update_skip {
        for delegate in delegates {
            run_embed_for_delegate(
                pg_pool,
                &ai_client,
                &qdrant_client,
                vector_update_skip,
                today,
                &mut to_analyse,
                delegate,
                app_state.parliament,
            )
            .await;
        }
    }

    if !stance_update_skip {
        let recently_answered = if vector_update_skip {
            recently_answered_pairs(pg_pool, today).await?
        } else {
            HashSet::new()
        };
        let mut skipped_answers = 0;

        for delegate in to_analyse {
            let priority = crate::ai_task::Priority::Medium;
            for question in &questions {
                if recently_answered.contains(&(delegate.id, question.id)) {
                    skipped_answers += 1;
                    continue;
                }
                let ai_client = ai_client.clone();
                let qdrant_client = qdrant_client.clone();
                let question_text = question.question.clone();
                let question_id = question.id;
                let delegate_id = delegate.id;
                let used_llm = used_llm.clone();

                let language = match app_state.parliament {
                    combx::Parliament::At => Language::De,
                    combx::Parliament::Eu => Language::En,
                };
                let parliament = app_state.parliament;

                let ai_task = AiTask {
                    input: None,
                    priority,
                    task_id: crate::hash!(app_state.parliament),
                    db_entry_id: entry_id,
                    score: priority.weight(),
                    pool: app_state.pg_pool.clone(),
                    rerunnable: true,
                    created_at: Instant::now(),
                    task: Box::new(move |task_args| {
                        Box::pin(async move {
                            process_delegate_stance(
                                &task_args,
                                &ai_client,
                                &qdrant_client,
                                delegate_id,
                                question_id,
                                &question_text,
                                &used_llm,
                                language,
                                parliament,
                            )
                            .await?;
                            Ok(())
                        })
                    }),
                };
                if let Err(e) = ai_task_tx.send(ai_task).await {
                    log::error!("Cannot send ai task: {e:?}")
                }
                entry_id += 1;
            }
        }

        if skipped_answers > 0 {
            log::info!(
                "Skipped {skipped_answers} answers that are newer than {} days",
                update_days()
            );
        }
    }

    Ok(())
}

async fn run_embed_for_delegate(
    pg_pool: &sqlx::PgPool,
    ai_client: &Arc<SomAiClient>,
    qdrant_client: &Qdrant,
    vector_update_skip: bool,
    today: DateTime<Utc>,
    to_analyse: &mut Vec<Delegate>,
    delegate: Delegate,
    parliament: Parliament,
) {
    let last_updated_at = sqlx::query_scalar!(
        "select updated_at from last_vector_speech_update where delegate_id = $1",
        delegate.id
    )
    .fetch_optional(pg_pool)
    .await;
    let Ok(last_updated_at) = last_updated_at else {
        log::error!(
            "could not fetch last_updated for {delegate:?} because of: {last_updated_at:?}"
        );
        return;
    };
    if is_fresh(last_updated_at, today) {
        log::info!("Skip delegate {}, time delta too low", delegate.id);
        return;
    }
    let collection_name = collection_name(parliament);
    match embed_speeches::process_delegate(
        pg_pool,
        qdrant_client,
        &**ai_client,
        collection_name,
        &delegate,
        !vector_update_skip,
    )
    .await
    {
        Err(e) => {
            log::error!("Failed processing delegate {}: {:?}", delegate.id, e);
        }
        Ok(()) => to_analyse.push(delegate),
    };
}

pub async fn run_shut(mut shutdown: Shutdown, app_state: AppState, ai_task_tx: Sender<AiTask>) {
    let political_analysis_sleep = sleep_seconds();

    let Some(ai_client) = connect_ai_client().await else {
        log::error!("political analysis service stops because it has no somes ai client");
        return;
    };
    let Some(qdrant_client) = connect_qdrant_client().await else {
        log::error!("political analysis service stops because it has no qdrant client");
        return;
    };

    while !shutdown.is_shutdown() {
        tokio::select! {
            _ = shutdown.recv() => {
                println!("shutting down");
                // return ok?
                break;
            }
            _ = async {
                let app_state = app_state.clone();
                log::info!("Starting political analysis service");
                if let Err(e) =
                    political_analysis(app_state, &ai_task_tx, ai_client.clone(), &qdrant_client)
                        .await
                {
                    log::error!("could not complete political analysis run: {e:?}")
                }
                // plenar_info_once(&pg_pool, true).await;
                log::info!("political analysis service sleeps for {political_analysis_sleep} seconds...");
                tokio::time::sleep(tokio::time::Duration::from_secs(political_analysis_sleep)).await;
            } => {}
        }
    }
}
