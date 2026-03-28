use combx::models::*;
use combx::{DbNamedVote, DbSpeechWithLink, Decree, GovProposal, VoteResult};
use somes_common_lib::{CallToOrder, Document};
use sqlx::PgPool;

#[derive(Debug)]
pub struct NamedVoteWithVoteResult {
    infavor: Option<bool>,
    was_absent: Option<bool>,
    vote: VoteResult,
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

    let recent_gov_proposals = vec![];
    // let recent_gov_proposals = sqlx::query_as!(
    //     OptionalGovProposal,
    //     r#"
    //     SELECT * FROM gov_proposals WHERE $1 = ANY("ministerial_issuers: Vec<i32>")
    //     and ("ministrial_proposal: DbMinistrialProposalQueryMeta").raw_data_created_at >= NOW() - make_interval(days => $2);"#,
    //     delegate_id,
    //     last_n_days
    // ).fetch_all(pg_pool).await?;
    let recent_issued_proposals = vec![];

    // let recent_issued_proposals = sqlx::query_as!(
    //     OptionalVoteResult,
    //     r#"
    //     SELECT *
    //     FROM vote_results
    //     WHERE EXISTS (
    //         SELECT 1
    //         FROM UNNEST("issued_by_dels: Vec<DbRelatedDelegate>") AS del
    //         WHERE del.delegate_id = $1::int
    //     )
    //     AND ("legislative_initiative: DbLegislativeInitiativeQuery").raw_data_created_at >= NOW() - make_interval(days => $2)
    //     "#,
    //     delegate_id,
    //     last_n_days
    // )
    // .fetch_all(pg_pool)
    // .await?;

    let recent_decrees =
        sqlx::query_as!(
            OptionalDecree,
            "select * from ministrial_decrees_with_docs where gov_official_id = $1 and publication_date >= NOW() - make_interval(days => $2)",
        delegate_id, last_n_days
    )
    .fetch_all(pg_pool)
    .await?;

    Ok(RecentActivity {
        gov_poposals: recent_gov_proposals,
        issued_proposals: recent_issued_proposals,
        decrees: recent_decrees,
        named_votes: vec![],
        speeches: vec![],
        call_to_orders: vec![],
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
