use std::sync::Arc;

use somai_client::SomAiClient;
use sqlx::PgPool;
use tokio::{sync::mpsc::Receiver, time::sleep};

use crate::{
    ai_task::{AiTask, TaskArgs},
    Shutdown,
};

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

        while !shutdown.is_shutdown() {
            let Ok(ai_task) = ai_task_rx.try_recv() else {
                sleep(std::time::Duration::from_millis(4)).await;
                continue;
            };
            let task_args = TaskArgs {
                db_entry_id: ai_task.db_entry_id,
                ai_task_input: ai_task.input,
                pg_pool: pg_pool.clone(),
                som_ai_client: somai_client.clone(),
            };

            if let Err(e) = tokio::spawn((ai_task.task)(task_args)).await {
                log::error!(
                    "Cannot process ai task: {:?}, {:?}, Error: {e:?}",
                    ai_task.db_entry_id,
                    ai_task.task_id
                )
            }
        }
    }
}
