use chrono::{DateTime, NaiveDateTime, Utc};
use somes_scraper::LastUpdated;
use sqlx::PgPool;

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

pub(crate) async fn should_update<T: StoredUpdatedAt + LastUpdated>(data: &T, pg: &PgPool) -> bool {
    if !mind_update_time() {
        return true;
    }
    let Some(stored_last_updated_at) = data.stored_updated_at(pg).await else {
        return true;
    };
    stored_last_updated_at != data.last_updated()
}
