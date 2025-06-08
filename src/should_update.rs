use chrono::NaiveDateTime;
use sqlx::PgPool;

pub(crate) trait StoredUpdatedAt {
    async fn stored_updated_at(&self, pg: &PgPool) -> Option<NaiveDateTime>;
}
