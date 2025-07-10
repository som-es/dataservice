use chrono::NaiveDate;
use sqlx::{query_as, PgPool};

use crate::models::DbPartyNamedVoteCount;

/*
with DelegateParty as (
    select
            delegates.id as id, mandates.party as party
        from
            mandates
        inner join
            delegates on delegates.id = mandates.delegate_id
        where
            mandates.name like '%Abge%National%'
        and start_date <= $2::date
        and (case when end_date is null then $2::date else end_date end) >= $2::date
)
select
    DelegateParty.party, COUNT(*)
from
    named_vote_info
inner join
    named_votes on named_vote_info.id = named_votes.named_vote_info_id
inner join
    DelegateParty on DelegateParty.id = named_votes.delegate_id
where
    named_vote_info.legis_init_id = $1
group by
    DelegateParty.party;

*/

pub async fn named_vote_pro_count_by_party(
    pg: &PgPool,
    legis_init_id: i32,
    at: NaiveDate,
) -> sqlx::Result<Vec<DbPartyNamedVoteCount>> {
    query_as!(
        DbPartyNamedVoteCount,
        "
        with DelegateParty as (
    select 
            delegates.id as id, mandates.party as party 
        from 
            mandates 
        inner join 
            delegates on delegates.id = mandates.delegate_id 
        where 
            mandates.name like '%Abge%National%' 
        and start_date <= $2::date 
        and (case when end_date is null then $2::date else end_date end) >= $2::date
)
select 
    DelegateParty.party, named_votes.infavor, COUNT(*) as count
from 
    named_vote_info 
inner join 
    named_votes on named_vote_info.id = named_votes.named_vote_info_id 
inner join 
    DelegateParty on DelegateParty.id = named_votes.delegate_id
where
    named_vote_info.legis_init_id = $1
and
    (named_votes.infavor = true or named_votes.infavor = false)

and
    DelegateParty.party is not null

group by
    DelegateParty.party,
    named_votes.infavor
order by
    count desc
    ",
        legis_init_id,
        at
    )
    .fetch_all(pg)
    .await
}
