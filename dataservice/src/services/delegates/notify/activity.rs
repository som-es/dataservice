use chrono::Days;
use combx::models::*;
use combx::with_data::delegates::{
    extract_call_to_orders_by_delegate, extract_last_delegate_speeches,
};
use combx::{DbSpeechWithLink, today};
use somes_common_lib::{CallToOrder, Document};
use sqlx::PgPool;

#[derive(Debug)]
pub struct NamedVoteWithVoteResult {
    pub(super) infavor: Option<bool>,
    pub(super) was_absent: Option<bool>,
    pub(super) title: Option<String>,
    pub(super) gp: String,
    pub(super) ityp: String,
    pub(super) inr: i32,
    pub(super) vote_date: chrono::NaiveDate,
}

#[derive(Debug)]
pub struct RecentActivity {
    pub(super) delegate_name: String,
    pub(super) delegate_party: Option<String>,
    pub(super) delegate_image_url: Option<String>,
    pub(super) gov_poposals: Vec<OptionalGovProposal>,
    pub(super) issued_proposals: Vec<OptionalVoteResult>,
    pub(super) decrees: Vec<OptionalDecree>,
    pub(super) named_votes: Vec<NamedVoteWithVoteResult>,
    pub(super) speeches: Vec<DbSpeechWithLink>,
    pub(super) call_to_orders: Vec<CallToOrder>,
}

impl RecentActivity {
    pub(super) fn has_activity(&self) -> bool {
        !self.gov_poposals.is_empty()
            || !self.issued_proposals.is_empty()
            || !self.decrees.is_empty()
            || !self.named_votes.is_empty()
            || !self.speeches.is_empty()
            || !self.call_to_orders.is_empty()
    }
}

pub async fn collect_recent_activity_by_delegate(
    delegate_id: i32,
    pg_pool: &PgPool,
    last_n_days: i32,
) -> sqlx::Result<RecentActivity> {
    let delegate = sqlx::query!(
        "select name, party, image_url from delegates where id = $1",
        delegate_id
    )
    .fetch_optional(pg_pool)
    .await?;

    let recent_gov_proposals = sqlx::query_as!(
        OptionalGovProposal,
        r#"
        SELECT * FROM gov_proposals WHERE $1 = ANY("ministerial_issuers: Vec<i32>")
        and ("ministrial_proposal: DbMinistrialProposalQueryMeta").raw_data_created_at >= NOW() - make_interval(days => $2);"#,
        delegate_id,
        last_n_days
    )
    .fetch_all(pg_pool)
    .await?;

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
        AND ("legislative_initiative: DbLegislativeInitiativeQuery").is_voteable_on
        AND ("legislative_initiative: DbLegislativeInitiativeQuery").raw_data_created_at >= NOW() - make_interval(days => $2)
        "#,
        delegate_id,
        last_n_days
    )
    .fetch_all(pg_pool)
    .await?;

    let recent_decrees = sqlx::query_as!(
        OptionalDecree,
        "select * from ministrial_decrees_with_docs where gov_official_id = $1 and publication_date >= NOW() - make_interval(days => $2)",
        delegate_id,
        last_n_days
    )
    .fetch_all(pg_pool)
    .await?;

    let named_votes = sqlx::query_as!(
        NamedVoteWithVoteResult,
        "
        select
            infavor,
            was_absent,
            COALESCE(s.short_title, li.title, li.description) as title,
            li.gp,
            li.ityp,
            li.inr,
            li.nr_plenary_activity_date as vote_date
        from
            named_votes
        inner join named_vote_info nvi on nvi.id = named_vote_info_id
        inner join legislative_initiatives li on li.id = legis_init_id
        left join legislative_initiative_summaries lis on lis.legis_init_id = li.id
        left join summaries s on s.id = lis.summary_id
        where delegate_id = $1
        and li.nr_plenary_activity_date >= CURRENT_DATE - make_interval(days => $2)::interval
        order by li.nr_plenary_activity_date desc
    ",
        delegate_id,
        last_n_days
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
        delegate_name: delegate
            .as_ref()
            .map(|delegate| delegate.name.clone())
            .unwrap_or_else(|| format!("Abgeordnete:r {delegate_id}")),
        delegate_party: delegate
            .as_ref()
            .and_then(|delegate| delegate.party.clone()),
        delegate_image_url: delegate.and_then(|delegate| delegate.image_url),
        gov_poposals: recent_gov_proposals,
        issued_proposals: recent_issued_proposals,
        decrees: recent_decrees,
        named_votes,
        speeches,
        call_to_orders,
    })
}
