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
    DelegateParty.party