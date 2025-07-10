use chrono::NaiveDate;
use sqlx::{query_as, PgPool};

use crate::db::models::DbDelegate;

pub async fn active_delegates_at_date(pg: &PgPool, at: NaiveDate) -> sqlx::Result<Vec<DbDelegate>> {
    query_as!(
        DbDelegate,
        "
        select 
            delegates.* 
        from 
            mandates 
        inner join 
            delegates on delegates.id = mandates.delegate_id 
        where 
            mandates.name like '%Abge%National%' 
        and start_date <= $1::date 
        and (case when end_date is null then $1::date else end_date end) >= $1::date;",
        at
    )
    .fetch_all(pg)
    .await
}
