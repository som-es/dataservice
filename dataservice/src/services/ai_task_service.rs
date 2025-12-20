use std::{collections::BinaryHeap, sync::Arc};

use somai_client::SomAiClient;
use sqlx::PgPool;
use tokio::{
    sync::{mpsc::Receiver, RwLock},
    time::sleep,
};

use crate::{
    ai_task::{AiTask, TaskArgs},
    Shutdown,
};

const MAX_CONCURRENT_TASKS: usize = 1; // mind context size for llama cpp

pub async fn ai_task_handler(
    tasks: Arc<RwLock<BinaryHeap<AiTask>>>,
    somai_client: Arc<SomAiClient>,
    pg_pool: PgPool,
    shutdown: Arc<Shutdown>,
) {
    let mut task_buffer = Vec::with_capacity(MAX_CONCURRENT_TASKS);
    while !shutdown.is_shutdown() {
        if task_buffer.len() >= MAX_CONCURRENT_TASKS {
            for (db_entry_id, task_id, task) in task_buffer.drain(..) {
                if let Err(e) = task.await {
                    log::error!(
                        "Cannot process ai task: {:?}, {:?}, Error: {e:?}",
                        db_entry_id,
                        task_id
                    )
                }
            }
        }
        let maybe_task = {
            let mut tasks_guard = tasks.write().await;
            tasks_guard.pop()
        };

        match maybe_task {
            Some(ai_task) => {
                let task_args = TaskArgs {
                    db_entry_id: ai_task.db_entry_id,
                    ai_task_input: ai_task.input,
                    pg_pool: pg_pool.clone(),
                    som_ai_client: somai_client.clone(),
                };
                let task = tokio::spawn((ai_task.task)(task_args));
                task_buffer.push((ai_task.db_entry_id, ai_task.task_id, task));
            }
            None => {
                sleep(std::time::Duration::from_millis(10)).await;
            }
        }
    }
}

pub async fn ai_tasks_service(
    shutdown: Shutdown,
    pg_pool: PgPool,
    mut ai_task_rx: Receiver<AiTask>,
) {
    #[cfg(feature = "ai")]
    {
        let mut somai_client = None;
        while somai_client.is_none() {
            let temp_somai_client = SomAiClient::new_local("http://127.0.0.1:8000").await;
            if let Err(ref e) = temp_somai_client {
                log::error!("Could not connect to somai API: {e}");
                sleep(std::time::Duration::from_millis(10000)).await;
            }
            somai_client = temp_somai_client.ok()
        }
        let somai_client = Arc::new(somai_client.unwrap());

        let task_queue = Arc::new(RwLock::new(std::collections::binary_heap::BinaryHeap::new()));

        let shutdown = Arc::new(shutdown);

        tokio::spawn(ai_task_handler(
            task_queue.clone(),
            somai_client.clone(),
            pg_pool.clone(),
            shutdown.clone(),
        ));

        while !shutdown.is_shutdown() {
            let Ok(ai_task) = ai_task_rx.try_recv() else {
                sleep(std::time::Duration::from_millis(4)).await;
                continue;
            };

            {
                let mut task_queue_guard = task_queue.write().await;
                task_queue_guard.push(ai_task);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::ai_task::Priority;

    #[tokio::test]
    async fn test_ai_task_ordering() {
        use super::AiTask;
        use std::future::Future;
        use std::pin::Pin;

        let task1 = AiTask {
            input: None,
            priority: Priority::High,
            task_id: 1,
            db_entry_id: 1,
            task: Box::new(|_args| {
                Box::pin(async move { Ok(()) })
                    as Pin<Box<dyn Future<Output = combx::Result<()>> + Send>>
            }),
        };

        let task2 = AiTask {
            input: None,
            priority: Priority::Low,
            task_id: 2,
            db_entry_id: 2,
            task: Box::new(|_args| {
                Box::pin(async move { Ok(()) })
                    as Pin<Box<dyn Future<Output = combx::Result<()>> + Send>>
            }),
        };

        assert!(task1 > task2);

        assert!(Priority::Highest > Priority::High);
    }
}
