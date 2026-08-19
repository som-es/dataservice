pub mod delegate_context;
mod embed_speeches;
mod stance_system;
mod upload_questions;

use std::ops::ControlFlow;

use chrono::TimeDelta;
use qdrant_client::{
    Qdrant,
    qdrant::{GetPoints, PointId},
};
use somai_client::SomAiClient;
use somes_scraper::language::Language;
use tokio::{sync::mpsc::Sender, time::Instant};

use crate::{
    AppState, Result, Shutdown,
    ai_task::{AiTask, TaskArgs},
    ai_tasks::somai_connect,
    political_analysis::{
        embed_speeches::get_delegate_ids,
        stance_system::{Citation, Stance, rag_pipeline},
    },
    today,
};

pub trait EmbeddingProvider: Send + Sync {
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

pub async fn process_delegate_stance(
    task_args: &TaskArgs,
    ai_client: &SomAiClient,
    qdrant_client: &Qdrant,
    delegate_id: i32,
    question_id: i32,
    question_text: &str,
    used_llm: &str,
    language: Language,
) -> crate::Result<()> {
    let stance = rag_pipeline::<Stance>(
        ai_client,
        qdrant_client,
        delegate_id,
        question_text,
        language,
    )
    .await?;

    let stance_llm = match stance.stance_detection {
        stance_system::StanceDetection::Postive => "Positive",
        stance_system::StanceDetection::Neutral => "Neutral",
        stance_system::StanceDetection::Negative => "Negative",
    };

    let answer_id = sqlx::query_scalar!(
        r#"
        INSERT INTO political_answers(question_id, delegate_id, answer, stance_llm, model_used)
        VALUES ($1, $2, $3, $4, $5) RETURNING id
        "#,
        question_id,
        delegate_id,
        stance.political_stance,
        stance_llm,
        used_llm
    )
    .fetch_one(&task_args.pg_pool)
    .await?;

    save_citations(
        &task_args.pg_pool,
        qdrant_client,
        answer_id,
        &stance.citations,
    )
    .await?;

    evaluate_and_save_opinion(
        &task_args.pg_pool,
        ai_client,
        delegate_id,
        question_id,
        answer_id,
        stance_llm,
        &stance.political_stance,
    )
    .await?;

    Ok(())
}

async fn save_citations(
    pg_pool: &sqlx::PgPool,
    qdrant_client: &Qdrant,
    answer_id: i32,
    citations: &[Citation],
) -> crate::Result<()> {
    for citation in citations {
        let point_id = PointId::from(citation.point_uuid.clone());

        let get_points_req = GetPoints {
            collection_name: "delegates".to_string(),
            ids: vec![point_id],
            with_payload: Some(true.into()),
            ..Default::default()
        };

        let qdrant_res = qdrant_client.get_points(get_points_req).await?;

        let mut text_snippet: Option<String> = None;

        if let Some(point) = qdrant_res.result.first()
            && let Some(text_value) = point.payload.get("text")
            && let Some(qdrant_client::qdrant::value::Kind::StringValue(text)) = &text_value.kind
        {
            text_snippet = Some(text.clone());
        }

        let Ok(parsed_uuid) = uuid::Uuid::parse_str(&citation.point_uuid) else {
            log::warn!("Skipping citation insertion because of malformed uuid: {citation:?}");
            continue;
        };

        sqlx::query!(
            r#"
            INSERT INTO stance_citations
                (answer_id, speech_id, point_uuid, text_snippet)
            VALUES
                ($1, $2, $3, $4)
            ON CONFLICT (answer_id, point_uuid) DO NOTHING
            "#,
            answer_id,
            citation.speech_id,
            parsed_uuid,
            text_snippet
        )
        .execute(pg_pool)
        .await?;
    }

    Ok(())
}

async fn evaluate_and_save_opinion(
    pg_pool: &sqlx::PgPool,
    ai_client: &SomAiClient,
    delegate_id: i32,
    question_id: i32,
    answer_id: i32,
    stance_llm: &str,
    political_stance: &str,
) -> crate::Result<()> {
    let ref_answers = sqlx::query!(
        r#"
        SELECT stance_llm, answer, question_id
        FROM political_answers
        WHERE question_id = $1 AND is_strong_reference
        ORDER BY stance_llm;
        "#,
        question_id
    )
    .fetch_all(pg_pool)
    .await?;

    let negative_ref = &ref_answers[0].answer;
    let positive_ref = &ref_answers[1].answer;

    let pro_sim = ai_client
        .embedding_embed_and_cosine(positive_ref.to_string(), political_stance.to_string())
        .await?
        .cosine_similarity;

    let contra_sim = ai_client
        .embedding_embed_and_cosine(negative_ref.to_string(), political_stance.to_string())
        .await?
        .cosine_similarity;

    let score = 10.0 * (pro_sim - contra_sim).powi(2);

    sqlx::query!(
        r#"
        INSERT INTO political_opinions
        (delegate_id, question_id, answer_id, stance, pro_strong_ref_score, contra_strong_ref_score, ref_score)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
        delegate_id,
        question_id,
        answer_id,
        stance_llm,
        pro_sim,
        contra_sim,
        score
    )
    .execute(pg_pool)
    .await?;

    Ok(())
}

async fn political_analysis(app_state: AppState, ai_task_tx: &Sender<AiTask>) -> Result<()> {
    let pg_pool = &app_state.pg_pool;
    let ai_client = somai_connect().await?;
    let used_llm = ai_client.get_model().await?.result;

    let qdrant_client = Qdrant::from_url(
        &std::env::var("QDRANT_URL").unwrap_or_else(|_| "http://127.0.0.1:6334".to_string()),
    )
    .build()?;

    let vector_update_skip = std::env::var("VECTOR_UPDATE_SKIP")
        .unwrap_or("false".into())
        .parse::<bool>()
        .unwrap_or_default();

    let stance_update_skip = std::env::var("STANCE_UPDATE_SKIP")
        .unwrap_or("false".into())
        .parse::<bool>()
        .unwrap_or_default();

    let delegates = get_delegate_ids(pg_pool).await?;
    let questions = sqlx::query_as!(
        Question,
        "SELECT id, question, is_liberal, is_left FROM political_questions"
    )
    .fetch_all(pg_pool)
    .await?;

    let mut entry_id = 0;
    let today = today().to_utc();

    let mut to_analyse = vec![];
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
            )
            .await;
        }
    }

    if !stance_update_skip {
        for delegate in to_analyse {
            let priority = crate::ai_task::Priority::Medium;
            for question in &questions {
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
    }

    Ok(())
}

async fn run_embed_for_delegate(
    pg_pool: &sqlx::Pool<sqlx::Postgres>,
    ai_client: &std::sync::Arc<SomAiClient>,
    qdrant_client: &Qdrant,
    vector_update_skip: bool,
    today: chrono::prelude::DateTime<chrono::prelude::Utc>,
    to_analyse: &mut Vec<embed_speeches::Delegate>,
    delegate: embed_speeches::Delegate,
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
    if today - last_updated_at.unwrap_or(today) < TimeDelta::days(30 * 3)
        && last_updated_at.is_some()
    {
        log::info!("Skip delegate {}, time delta too low", delegate.id);
        return;
    }
    match embed_speeches::process_delegate(
        pg_pool,
        qdrant_client,
        &**ai_client,
        "delegates",
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
                let app_state = app_state.clone();
                log::info!("Starting political analysis service");
                if let Err(e) = political_analysis(app_state, &ai_task_tx).await {
                    log::error!("could not complete political analysis run: {e:?}")
                }
                // plenar_info_once(&pg_pool, true).await;
                log::info!("political analysis service sleeps for {political_analysis_sleep} seconds...");
                tokio::time::sleep(tokio::time::Duration::from_secs(political_analysis_sleep)).await;
            } => {}
        }
    }
}
