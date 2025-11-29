use chrono::NaiveDate;
use combx::{DbNamedVote, DbNamedVoteInfo, OptionalDbNamedVote};
use sqlx::{Executor, PgPool, Postgres};

pub async fn insert_named_vote_info<'a>(
    pg: impl Executor<'a, Database = Postgres>,
    named_vote_info: &DbNamedVoteInfo,
    legis_init_id: i32,
) -> Result<i32, sqlx::Error> {
    let result = sqlx::query!(
        "
            INSERT INTO named_vote_info (
                legis_init_id, pro_count, contra_count, given_vote_sum, invalid_count
            ) VALUES (
                $1, $2, $3, $4, $5
            ) ON CONFLICT (legis_init_id)
            DO UPDATE SET
                pro_count = EXCLUDED.pro_count,
                contra_count = EXCLUDED.contra_count,
                given_vote_sum = EXCLUDED.given_vote_sum,
                invalid_count = EXCLUDED.invalid_count
            RETURNING id
        ",
        legis_init_id,
        named_vote_info.pro_count,
        named_vote_info.contra_count,
        named_vote_info.given_vote_sum,
        named_vote_info.invalid_count
    )
    .fetch_one(pg)
    .await;

    match result {
        Ok(record) => Ok(record.id),
        Err(e) => {
            log::warn!(
                "Error inserting named vote info: '{:?}'. The error occurred for the following named vote info: {:?}",
                e,
                named_vote_info
            );
            Err(e)
        }
    }
}

pub async fn insert_named_votes<'a>(
    tx: &mut sqlx::Transaction<'a, Postgres>,
    named_votes: &[OptionalDbNamedVote],
    named_vote_info_id: i32,
) -> sqlx::Result<()> {
    for named_vote in named_votes {
        if let Err(e) = sqlx::query!(
            "
                INSERT INTO named_votes (
                    infavor, was_absent, lev, similiarity_score,
                    searched_with, matched_with, delegate_id, named_vote_info_id, manually_matched
                ) VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9
                ) ON CONFLICT (delegate_id, named_vote_info_id)
                DO UPDATE SET
                    infavor = EXCLUDED.infavor,
                    was_absent = EXCLUDED.was_absent,
                    lev = EXCLUDED.lev,
                    similiarity_score = EXCLUDED.similiarity_score,
                    searched_with = EXCLUDED.searched_with,
                    matched_with = EXCLUDED.matched_with,
                    manually_matched = EXCLUDED.manually_matched
            ",
            named_vote.infavor,
            named_vote.was_absent,
            named_vote.lev,
            named_vote.similiarity_score,
            named_vote.searched_with,
            named_vote.matched_with,
            named_vote.delegate_id,
            named_vote_info_id,
            named_vote.manually_matched
        )
        .execute(&mut **tx)
        .await
        {
            log::warn!(
                "Error inserting named vote: '{:?}'. The error occurred for the following named vote: {:?}",
                e,
                named_vote
            );
        }
    }

    Ok(())
}
