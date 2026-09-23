use chrono::NaiveDate;
use sqlx::{Executor, Postgres, query_as};

use crate::{DelegateParty, models::DbDelegate};

pub async fn active_delegates_with_party_at_date<'a>(
    pg: impl Executor<'a, Database = Postgres>,
    at: NaiveDate,
    function: &str,
) -> sqlx::Result<Vec<DelegateParty>> {
    query_as!(
        DelegateParty,
        "
        select
            delegates.id,
            mandates.party
        from
            mandates
        inner join
            delegates on delegates.id = mandates.delegate_id
        where
            mandates.function = $2
        and start_date <= $1::date
        and (case when end_date is null then $1::date else end_date end) >= $1::date;",
        at,
        function
    )
    .fetch_all(pg)
    .await
}

pub async fn active_delegates_at_date<'a>(
    pg: impl Executor<'a, Database = Postgres>,
    at: NaiveDate,
    function: &str,
) -> sqlx::Result<Vec<DbDelegate>> {
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
            mandates.function = $2
        and start_date <= $1::date
        and (case when end_date is null then $1::date else end_date end) >= $1::date;",
        at,
        function
    )
    .fetch_all(pg)
    .await
}
