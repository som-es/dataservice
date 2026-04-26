use chrono::Days;
use combx::with_data::delegates::{
    extract_call_to_orders_by_delegate, extract_last_delegate_speeches,
};
use combx::DbSpeechWithLink;
use combx::{models::*, today};
use somes_common_lib::{CallToOrder, Document, NamedVote};
use sqlx::PgPool;

#[derive(Debug)]
pub struct NamedVoteWithVoteResult {
    infavor: Option<bool>,
    was_absent: Option<bool>,
    title: Option<String>,
    // vote: VoteResult,
}

#[derive(Debug)]
pub struct RecentActivity {
    gov_poposals: Vec<OptionalGovProposal>,
    issued_proposals: Vec<OptionalVoteResult>,
    decrees: Vec<OptionalDecree>,
    named_votes: Vec<NamedVoteWithVoteResult>,
    speeches: Vec<DbSpeechWithLink>,
    call_to_orders: Vec<CallToOrder>,
}

pub async fn collect_recent_activity_by_delegate(
    delegate_id: i32,
    pg_pool: &PgPool,
    last_n_days: i32,
    // client: reqwest::Client,
) -> sqlx::Result<RecentActivity> {
    // client.get("https://somes.at/api/at/v1/gov_proposals/search")
    // http://localhost:3000/api/at/v1/gov_proposals/search?page=1&gov_proposal[ministerial_issuers][in][0]=20445&date_from=2026-01-10

    // let recent_gov_proposals = vec![];
    let recent_gov_proposals = sqlx::query_as!(
        OptionalGovProposal,
        r#"
        SELECT * FROM gov_proposals WHERE $1 = ANY("ministerial_issuers: Vec<i32>")
        and ("ministrial_proposal: DbMinistrialProposalQueryMeta").raw_data_created_at >= NOW() - make_interval(days => $2);"#,
        delegate_id,
        last_n_days
    ).fetch_all(pg_pool).await?;
    // let recent_issued_proposals = vec![];

    let recent_issued_proposals = sqlx::query_as!(
        OptionalVoteResult,
        r#"
        SELECT *
        FROM vote_results
        WHERE EXISTS (
            SELECT 1
            FROM UNNEST("issued_by_dels: Vec<DbRelatedDelegate>") AS del
            WHERE del.delegate_id = $1::int
        )
        AND ("legislative_initiative: DbLegislativeInitiativeQuery").raw_data_created_at >= NOW() - make_interval(days => $2)
        "#,
        delegate_id,
        last_n_days
    )
    .fetch_all(pg_pool)
    .await?;

    let recent_decrees =
        sqlx::query_as!(
            OptionalDecree,
            "select * from ministrial_decrees_with_docs where gov_official_id = $1 and publication_date >= NOW() - make_interval(days => $2)",
        delegate_id, last_n_days
    )
    .fetch_all(pg_pool)
    .await?;

    let named_votes = sqlx::query_as!(
        NamedVoteWithVoteResult,
        "
        select
            infavor, was_absent, COALESCE(title, description) as title
        from
            named_votes
        inner join named_vote_info nvi on nvi.id = named_vote_info_id
        inner join legislative_initiatives li on li.id = legis_init_id
        left join legislative_initiative_summaries lis on lis.legis_init_id = li.id
        left join summaries s on s.id = lis.summary_id
        where delegate_id = $1
    ",
        delegate_id
    )
    .fetch_all(pg_pool)
    .await?;

    let offset_day = today()
        .checked_sub_days(Days::new(last_n_days as u64))
        .unwrap();
    let call_to_orders = extract_call_to_orders_by_delegate(delegate_id, pg_pool)
        .await?
        .into_iter()
        .filter(|cto| (cto.date.date_naive() - offset_day).num_days() >= 0)
        .collect::<Vec<_>>();

    let speeches = extract_last_delegate_speeches(delegate_id, pg_pool, last_n_days).await?;

    Ok(RecentActivity {
        gov_poposals: recent_gov_proposals,
        issued_proposals: recent_issued_proposals,
        decrees: recent_decrees,
        named_votes,
        speeches,
        call_to_orders,
    })
}

#[cfg(test)]
mod tests {
    use crate::{connect_pg, delegates::notify::collect_recent_activity_by_delegate};

    #[tokio::test]
    async fn test_collect_recent_activity_by_delegate() {
        let pg_pool = connect_pg().await;
        let delegate_id = 32506;
        let last_n_days = 30;
        let activity = collect_recent_activity_by_delegate(delegate_id, &pg_pool, last_n_days)
            .await
            .unwrap();
        dbg!(activity);
    }
}
