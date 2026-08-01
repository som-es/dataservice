use combx::{DbNamedVote, DbNamedVoteInfo, DbVote};
use sqlx::{Acquire, Executor, Postgres, Transaction};

pub async fn insert_votes_batch<'a>(
    tx: &mut Transaction<'a, Postgres>,
    votes: &[DbVote],
    legis_init_id: i32,
) -> sqlx::Result<()> {
    if votes.is_empty() {
        return Ok(());
    }

    let mut sp = tx.begin().await?;

    if let Err(e) = sqlx::query!(
        "delete from votes where legislative_initiatives_id = $1",
        legis_init_id
    )
    .execute(&mut *sp)
    .await
    {
        log::error!("Cannot delete votes from legis init: {legis_init_id} with error {e:?}");
    }

    let parties: Vec<String> = votes.iter().map(|v| v.party.clone()).collect();
    let fractions: Vec<i32> = votes.iter().map(|v| v.fraction.clone()).collect();
    let infavors: Vec<bool> = votes.iter().map(|v| v.infavor).collect();
    let ids = vec![legis_init_id; votes.len()];

    let res = sqlx::query!(
        "
            INSERT INTO votes (party, fraction, infavor, legislative_initiatives_id)
            SELECT * FROM UNNEST($1::text[], $2::int4[], $3::bool[], $4::int4[])
            ON CONFLICT (legislative_initiatives_id, party)
            DO UPDATE SET fraction = EXCLUDED.fraction, infavor = EXCLUDED.infavor
        ",
        &parties,
        &fractions,
        &infavors,
        &ids
    )
    .execute(&mut *sp)
    .await;

    match res {
        Ok(_) => {
            sp.commit().await?;
            Ok(())
        }
        Err(e) => Err(e),
    }
}

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
            log::error!(
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
    named_votes: &[DbNamedVote],
    named_vote_info_id: i32,
) -> sqlx::Result<()> {
    sqlx::query!(
        "delete from named_votes where named_vote_info_id = $1",
        named_vote_info_id
    )
    .execute(&mut **tx)
    .await?;

    for named_vote in named_votes {
        if let Err(e) = sqlx::query!(
            "
                INSERT INTO named_votes (
                    infavor, was_absent, lev, similiarity_score,
                    searched_with, matched_with, delegate_id, named_vote_info_id, manually_matched, was_abstention
                ) VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10
                ) ON CONFLICT (delegate_id, named_vote_info_id)
                DO UPDATE SET
                    infavor = EXCLUDED.infavor,
                    was_absent = EXCLUDED.was_absent,
                    was_abstention = EXCLUDED.was_abstention,
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
            named_vote.manually_matched,
            named_vote.was_abstention,
        )
        .execute(&mut **tx)
        .await
        {
            log::error!(
                "Error inserting named vote: '{:?}'. The error occurred for the following named vote: {:?}",
                e,
                named_vote
            );
        }
    }

    Ok(())
}

pub async fn insert_named_votes_batched<'a>(
    tx: &mut sqlx::Transaction<'a, Postgres>,
    named_votes: &[DbNamedVote],
    named_vote_info_id: i32,
) -> sqlx::Result<()> {
    if named_votes.is_empty() {
        return Ok(());
    }

    let mut sp = tx.begin().await?;

    let infavors: Vec<Option<bool>> = named_votes.iter().map(|v| v.infavor).collect();
    let was_absents: Vec<Option<bool>> = named_votes.iter().map(|v| v.was_absent).collect();
    let levs: Vec<i32> = named_votes.iter().map(|v| v.lev as i32).collect();
    let similarity_scores: Vec<i32> = named_votes
        .iter()
        .map(|v| v.similiarity_score as i32)
        .collect();
    let searched_withs: Vec<Option<String>> = named_votes
        .iter()
        .map(|v| v.searched_with.clone())
        .collect();
    let matched_withs: Vec<String> = named_votes.iter().map(|v| v.matched_with.clone()).collect();
    let delegate_ids: Vec<i32> = named_votes.iter().map(|v| v.delegate_id).collect();
    let manually_matcheds: Vec<Option<bool>> =
        named_votes.iter().map(|v| v.manually_matched).collect();
    let named_vote_info_ids = vec![named_vote_info_id; named_votes.len()];

    let res = sqlx::query!(
        "
            INSERT INTO named_votes (
                infavor, was_absent, lev, similiarity_score,
                searched_with, matched_with, delegate_id, manually_matched, named_vote_info_id
            )
            SELECT * FROM UNNEST(
                $1::bool[], $2::bool[], $3::int4[], $4::int4[],
                $5::text[], $6::text[], $7::int4[], $8::bool[], $9::int4[]
            )
            ON CONFLICT (delegate_id, named_vote_info_id)
            DO UPDATE SET
                infavor = EXCLUDED.infavor,
                was_absent = EXCLUDED.was_absent,
                lev = EXCLUDED.lev,
                similiarity_score = EXCLUDED.similiarity_score,
                searched_with = EXCLUDED.searched_with,
                matched_with = EXCLUDED.matched_with,
                manually_matched = EXCLUDED.manually_matched
        ",
        &infavors as &[Option<bool>],
        &was_absents as &[Option<bool>],
        &levs,
        &similarity_scores,
        &searched_withs as &[Option<String>],
        &matched_withs,
        &delegate_ids,
        &manually_matcheds as &[Option<bool>],
        &named_vote_info_ids
    )
    .execute(&mut *sp)
    .await;

    match res {
        Ok(_) => {
            sp.commit().await?;
            Ok(())
        }
        Err(e) => Err(e),
    }
}
