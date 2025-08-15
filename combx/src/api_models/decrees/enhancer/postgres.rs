use somes_scraper::Thing;
use sqlx::PgPool;

use crate::{api_models::decrees::enhancer::DecreeEnhancer, PostgresEnhancer};

impl DecreeEnhancer for PostgresEnhancer {
    fn gov_officials_at_date(
        &self,
        decree: &Thing,
    ) -> impl Future<Output = Result<Vec<GovOfficialAtDate>, Self::Error>> {
        pg_gov_officials_at_date(&self.0, decree)
    }
}

pub struct GovOfficialAtDate {
    pub delegate_id: i32,
    pub ministry: Option<String>
}

pub async fn pg_gov_officials_at_date(pg_pool: &PgPool, decree: &Thing) -> sqlx::Result<Vec<GovOfficialAtDate>> {
    sqlx::query_as!(GovOfficialAtDate,
        "
        SELECT 
            mandates.delegate_id, 
            ministry
        FROM 
            mandates 
        WHERE 
            mandates.is_gov_official
            AND ministry is not null
            AND mandates.start_date <= $1::date 
            AND name not like '%sekretär%'
            AND (CASE WHEN mandates.end_date IS NULL THEN $1::date ELSE mandates.end_date END) >= $1::date;

        ",
        decree.date
    )
    .fetch_all(pg_pool)
    .await
}