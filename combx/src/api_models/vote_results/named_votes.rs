use std::collections::HashSet;

use chrono::NaiveDate;
use somes_scraper::ProContra;
use sqlx::PgPool;

use crate::{match_delegate_names_with_delegate_ids, with_data::active_delegates_at_date, DbNamedVote, LevResult};


pub async fn create_db_named_votes(
    pg: &PgPool,
    votes: &ProContra,
    voted_on: NaiveDate,
    gp: &str,
) -> sqlx::Result<Vec<DbNamedVote>> {
    let named_votes_compact = match_all_named_votes(pg, votes, voted_on, gp).await?;

    let pro_iter = named_votes_compact.pro.iter().map(|v| DbNamedVote {
        id: 0,
        infavor: Some(true),
        was_absent: Some(false),
        lev: v.lev as i64,
        similiarity_score: v.similiarity_score as i64,
        searched_with: Some(v.searched_with.clone()),
        matched_with: v.matched_with.clone(),
        delegate_id: v.delegate_id as i32,
        manually_matched: Some(v.manually_matched),
    });
    let contra_iter = named_votes_compact.contra.iter().map(|v| DbNamedVote {
        id: 0,
        infavor: Some(false),
        was_absent: Some(false),
        lev: v.lev as i64,
        similiarity_score: v.similiarity_score as i64,
        searched_with: Some(v.searched_with.clone()),
        matched_with: v.matched_with.clone(),
        delegate_id: v.delegate_id as i32,
        manually_matched: Some(v.manually_matched),
    });

    let absent_iter = named_votes_compact.absent.into_iter().map(|v| DbNamedVote {
        id: 0,
        infavor: None,
        was_absent: Some(true),
        lev: 0,
        similiarity_score: 0,
        searched_with: None,
        matched_with: v.0,
        delegate_id: v.1 as i32,
        manually_matched: Some(false),
    });

    Ok(pro_iter.chain(contra_iter).chain(absent_iter).collect::<Vec<_>>())
}

pub struct VotedByNameResult {
    pro: Vec<LevResult>,
    contra: Vec<LevResult>,
    absent: HashSet<(String, usize)>,
}

pub async fn match_all_named_votes(
    pg: &PgPool,
    votes: &ProContra,
    voted_on: NaiveDate,
    gp: &str,
) -> sqlx::Result<VotedByNameResult> {
    let active_delegates = active_delegates_at_date(pg, voted_on)
        .await
        .unwrap()
        .into_iter()
        .map(|delegate| (delegate.name, delegate.id as usize))
        // .inspect(|x| println!("{x:?}"))
        .collect::<HashSet<_>>();

    let pro_res = match_delegate_names_with_delegate_ids(&active_delegates, &votes.pro, gp);
    let mut pro = pro_res.correctly_matched;
    pro.extend_from_slice(&pro_res.lev_or_deep_split_matched);
    println!("lev_or_deep_pro: {:?}", pro_res.lev_or_deep_split_matched);

    let contra_res = match_delegate_names_with_delegate_ids(&active_delegates, &votes.contra, gp);
    let mut contra = contra_res.correctly_matched;
    contra.extend_from_slice(&contra_res.lev_or_deep_split_matched);

    // println!(
    //     "lev_or_deep_contra: {:?}",
    //     contra_res.lev_or_deep_split_matched
    // );

    let absent = pro_res
        .not_matchable
        .intersection(&contra_res.not_matchable)
        .cloned()
        .collect();

    Ok(VotedByNameResult {
        pro,
        contra,
        absent,
    })
}
