use sqlx::PgPool;

use crate::Shutdown;

pub async fn delete_was_updated_entries(mut shutdown: Shutdown, pg_pool: PgPool) {
    while !shutdown.is_shutdown() {
        tokio::select! {
            _ = shutdown.recv() => {
                println!("shutting down");
                // return ok?
                break;
            }
            _ = async {
                let pg_pool = pg_pool.clone();
                if let Err(e) = tokio::task::spawn(async move {
                    sqlx::query!("DELETE FROM legis_init_was_updated WHERE updated_at < now() - interval '400 seconds';").execute(&pg_pool).await
                }).await {
                    log::error!("Error in was updated deleting: {:?}", e);
                }
                log::info!("Was updated deleter sleeps for 40 secs...");
                tokio::time::sleep(tokio::time::Duration::from_secs(40)).await;
            } => {}
        }
    }
}
