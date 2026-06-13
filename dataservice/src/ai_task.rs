use std::{fmt::Debug, future::Future, pin::Pin, sync::Arc};

use sqlx::PgPool;
use tokio::time::Instant;

#[derive(PartialEq, Debug, Clone, Eq, PartialOrd, Ord, Copy)]
pub enum Priority {
    Highest = 12,
    Higher = 9,
    High = 8,
    Medium = 5,
    Low = 2,
    Lowest = 0,
}

impl Priority {
    pub fn weight(&self) -> u64 {
        (*self) as u8 as u64 * 655
    }
}

#[derive(Clone)]
pub struct TaskArgs {
    pub db_entry_id: i32,
    pub ai_task_input: Option<String>,
    pub pg_pool: PgPool,
    pub som_ai_client: Arc<somai_client::SomAiClient>,
}

pub struct AiTask {
    pub input: Option<String>,
    pub priority: Priority,
    pub score: u64,
    pub task_id: u64,
    pub db_entry_id: i32,
    pub rerunnable: bool,
    pub created_at: Instant,
    pub task: Box<
        dyn (FnOnce(TaskArgs) -> Pin<Box<dyn Future<Output = combx::Result<()>> + Send>>)
            + Send
            + Sync,
    >,
}

const AGING_RATE_PER_SEC: f64 = 1.;

impl AiTask {
    pub fn new(
        priority: Priority,
        task_id: u64,
        db_entry_id: i32,
        task: Box<
            dyn (FnOnce(TaskArgs) -> Pin<Box<dyn Future<Output = combx::Result<()>> + Send>>)
                + Send
                + Sync,
        >,
    ) -> AiTask {
        AiTask {
            input: None,
            priority,
            score: priority.weight(),
            task_id,
            db_entry_id,
            rerunnable: true,
            created_at: Instant::now(),
            task,
        }
    }
    pub fn effective_score(&self, now: Instant) -> u64 {
        let waited = now.saturating_duration_since(self.created_at).as_secs();
        self.priority.weight() + (AGING_RATE_PER_SEC * waited as f64) as u64
    }
}

impl Ord for AiTask {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.score
            .cmp(&other.score)
            .then(self.priority.cmp(&other.priority))
            .then(other.created_at.cmp(&self.created_at))
            .then(self.task_id.cmp(&other.task_id))
            .then(self.db_entry_id.cmp(&other.db_entry_id))
            .then(self.input.cmp(&other.input))
    }
}

impl PartialOrd for AiTask {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for AiTask {}
impl PartialEq for AiTask {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
            && self.task_id == other.task_id
            && self.db_entry_id == other.db_entry_id
            && self.input == other.input
    }
}

impl Debug for AiTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AiTask")
            .field("input", &self.input)
            .field("task_id", &self.task_id)
            .field("db_entry_id", &self.db_entry_id)
            .field("task", &"fn() ...")
            .finish()
    }
}
