use std::collections::HashSet;

use chrono::NaiveDate;
use somes_scraper::{DelegateRef, NamedVoteProContra};
use sqlx::PgPool;

use crate::{
    DbDelegate, DbNamedVote, Error::Custom, LevResult, match_delegate_names_with_delegate_ids,
    with_data::active_delegates_at_date,
};

pub async fn pg_create_db_named_votes(
    pg: &PgPool,
    votes: &NamedVoteProContra,
    voted_on: NaiveDate,
    gp: &str,
    function: &str,
) -> crate::Result<Vec<DbNamedVote>> {
    let named_votes_compact = match_all_named_votes(pg, votes, voted_on, gp, function).await?;

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
        was_abstention: false,
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
        was_abstention: false,
    });

    let absent_iter = named_votes_compact.absent.into_iter().map(|v| DbNamedVote {
        id: 0,
        infavor: None,
        was_absent: Some(true),
        lev: 0,
        similiarity_score: 0,
        searched_with: None,
        matched_with: v.name,
        delegate_id: v.id as i32,
        manually_matched: Some(false),
        was_abstention: false,
    });

    let abstention_iter = named_votes_compact
        .abstention
        .into_iter()
        .map(|v| DbNamedVote {
            id: 0,
            infavor: None,
            was_absent: Some(false),
            lev: v.lev as i64,
            similiarity_score: v.similiarity_score as i64,
            searched_with: Some(v.searched_with.clone()),
            matched_with: v.matched_with.clone(),
            delegate_id: v.delegate_id as i32,
            manually_matched: Some(v.manually_matched),
            was_abstention: true,
        });

    Ok(pro_iter
        .chain(contra_iter)
        .chain(absent_iter)
        .chain(abstention_iter)
        .collect::<Vec<_>>())
}

pub struct VotedByNameResult {
    pro: Vec<LevResult>,
    contra: Vec<LevResult>,
    abstention: Vec<LevResult>,
    absent: HashSet<DbDelegate>,
}

pub async fn match_all_named_votes(
    pg: &PgPool,
    votes: &NamedVoteProContra,
    voted_on: NaiveDate,
    gp: &str,
    function: &str,
) -> crate::Result<VotedByNameResult> {
    let mut active_delegates = active_delegates_at_date(pg, voted_on, function)
        .await?
        .into_iter()
        .collect::<HashSet<_>>();

    let (pro_already_matched, pro_to_match) =
        extract_already_matched(&votes.pro, voted_on, &mut active_delegates);

    let (contra_already_matched, contra_to_match) =
        extract_already_matched(&votes.contra, voted_on, &mut active_delegates);

    let (abstention_already_matched, abstention_to_match) =
        extract_already_matched(&votes.abstention, voted_on, &mut active_delegates);

    if (!pro_already_matched.is_empty() && !pro_to_match.is_empty())
        || (!contra_already_matched.is_empty() && !contra_to_match.is_empty())
        || (!abstention_already_matched.is_empty() && !abstention_to_match.is_empty())
    {
        return Err(
            Custom("some dels with id while some not is currently not supported".into()).into(),
        );
    }

    if (!pro_already_matched.is_empty() || pro_to_match.is_empty())
        && (!contra_already_matched.is_empty() || contra_to_match.is_empty())
        && (!abstention_already_matched.is_empty() || abstention_to_match.is_empty())
    {
        return Ok(VotedByNameResult {
            pro: collect_to_lev_result_from_del_ref(&pro_already_matched),
            contra: collect_to_lev_result_from_del_ref(&contra_already_matched),
            abstention: collect_to_lev_result_from_del_ref(&abstention_already_matched),
            absent: active_delegates,
        });
    }

    let votes = votes.clone();
    let gp = gp.to_string();
    let (pro, contra, absent) = tokio::task::spawn_blocking(move || {
        let pro_res = match_delegate_names_with_delegate_ids(
            &active_delegates,
            &votes
                .pro
                .iter()
                .map(|pro| pro.name.clone())
                .collect::<Vec<_>>(),
            &gp,
            voted_on,
        );
        let mut pro = pro_res.correctly_matched;
        pro.extend_from_slice(&pro_res.lev_or_deep_split_matched);

        let contra_res = match_delegate_names_with_delegate_ids(
            &active_delegates,
            &votes
                .contra
                .iter()
                .map(|pro| pro.name.clone())
                .collect::<Vec<_>>(),
            &gp,
            voted_on,
        );
        let mut contra = contra_res.correctly_matched;
        contra.extend_from_slice(&contra_res.lev_or_deep_split_matched);

        let absent = pro_res
            .not_matchable
            .intersection(&contra_res.not_matchable)
            .cloned()
            .collect();

        (pro, contra, absent)
    })
    .await?;

    // println!(
    //     "lev_or_deep_contra: {:?}",
    //     contra_res.lev_or_deep_split_matched
    // );

    Ok(VotedByNameResult {
        pro,
        contra,
        absent,
        abstention: vec![],
    })
}

fn extract_already_matched<'a>(
    refs: &'a [DelegateRef],
    voted_on: NaiveDate,
    active_delegates: &mut HashSet<DbDelegate>,
) -> (Vec<&'a DelegateRef>, Vec<&'a DelegateRef>) {
    refs
        .iter()
        .filter(|del| del.id.is_some() || !del.name.is_empty())
        .partition(|del| match del.id {
            Some(id) => {
                let del = active_delegates.iter().find(|del| del.id as u32 == id).cloned();
                if let Some(del) = del {
                    active_delegates.remove(&del);
                } else {
                    log::warn!("should have delegate (was absent) in active dels at date for: {voted_on}, del: {id}");
                }
                true
            }
            None => false,
        })
}

pub fn collect_to_lev_result_from_del_ref(del_refs: &[&DelegateRef]) -> Vec<LevResult> {
    del_refs
        .iter()
        .map(|absence| LevResult {
            deep_split_matched: false,
            similiarity_score: 0,
            lev: 0,
            delegate_id: absence.id.unwrap() as usize,
            manually_matched: false,
            matched_with: "".into(),
            searched_with: "".into(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use somes_scraper::{eu_parliament::EuParliamentScraper, legis_init::LegislativeInitiatives};

    use crate::{connect_pg, vote_results::named_votes::match_all_named_votes};

    #[tokio::test]
    async fn test_eu_namentlich_name_matching() {
        let scraper = EuParliamentScraper::default();
        let init = scraper.get_legis_init("2024-0003").await.unwrap();
        println!("date: {:?}", init.nr_plenary_activity_date);
        assert!(init.hidden_votes_info.voted_by_name);
        let pg = connect_pg().await;
        let voted_by_name = init.hidden_votes_info.voted_by_name_data.unwrap();
        let abstention_count = voted_by_name.abstention_count.unwrap();
        let pro_count = voted_by_name.pro_count;
        let contra_count = voted_by_name.contra_count;
        let voted_by_name = voted_by_name.delegates_pro_contra.unwrap();
        let res = match_all_named_votes(
            &pg,
            &voted_by_name,
            init.vote_date.unwrap(),
            "9".into(),
            "MEP".into(),
        )
        .await
        .unwrap();
        assert_eq!(abstention_count, res.abstention.len());
        assert_eq!(pro_count, res.pro.len());
        assert_eq!(contra_count, res.contra.len());
    }
}
