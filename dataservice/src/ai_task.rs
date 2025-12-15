use std::{fmt::Debug, future::Future, pin::Pin, sync::Arc};

use sqlx::PgPool;

#[derive(PartialEq, Debug, Clone, Eq, PartialOrd, Ord)]
pub enum Priority {
    Highest = 10,
    High = 8,
    Medium = 5,
    Low = 2,
    Lowest = 0,
}

pub struct TaskArgs {
    pub db_entry_id: i32,
    pub ai_task_input: Option<String>,
    pub pg_pool: PgPool,
    pub som_ai_client: Arc<somai_client::SomAiClient>,
}

pub struct AiTask {
    pub input: Option<String>,
    pub priority: Priority,
    pub task_id: u64,
    pub db_entry_id: i32,
    pub task: Box<
        dyn (FnOnce(TaskArgs) -> Pin<Box<dyn (Future<Output = combx::Result<()>>) + Send>>)
            + Send
            + Sync,
    >,
}

impl Ord for AiTask {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.priority.cmp(&other.priority)
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
