use std::hash::Hasher;

use chrono::{DateTime, Utc};
use combx::Index;
use redis::AsyncCommands;
use serde::Serialize;
use somes_scraper::LastUpdated;
use sqlx::PgPool;

pub async fn notify_update_streamed<T: Serialize>(
    index: Index,
    data: &T,
    client: &redis::Client,
) -> combx::Result<()> {
    let stream_id = index.as_str();
    let mut con = client.get_multiplexed_async_connection().await?;
    let vote_result = serde_json::to_string(data)?;
    let _: () = con.xadd(stream_id, "*", &[("data", vote_result)]).await?;
    Ok(())
}

#[inline]
pub(crate) fn enforce_ai_run() -> bool {
    std::env::var("ENFORCE_AI_RUN").unwrap_or("true".into()) == "true"
}

#[inline]
fn mind_update_time() -> bool {
    std::env::var("MIND_UPDATE_TIME").unwrap_or("true".into()) == "true"
}

pub(crate) trait StoredUpdatedAt {
    async fn stored_updated_at(&self, pg: &PgPool) -> Option<DateTime<Utc>>;
}

pub(crate) trait CompareHash: std::hash::Hash {
    async fn stored_hash(&self, pg: &PgPool) -> Option<u64>;
    fn hash_of(&self) -> u64 {
        let mut hasher = std::hash::DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }
}

pub(crate) async fn should_update_by_hash<T: CompareHash>(data: &T, pg: &PgPool) -> (bool, u64) {
    let hash = data.hash_of();
    if !mind_update_time() {
        return (true, hash);
    }
    let Some(stored_last_hash) = data.stored_hash(pg).await else {
        return (true, hash);
    };
    (stored_last_hash != hash, hash)
}

pub(crate) async fn should_update_by_time<T: StoredUpdatedAt + LastUpdated>(
    data: &T,
    pg: &PgPool,
) -> bool {
    if !mind_update_time() {
        return true;
    }
    let Some(stored_last_updated_at) = data.stored_updated_at(pg).await else {
        return true;
    };
    stored_last_updated_at != data.last_updated()
}
