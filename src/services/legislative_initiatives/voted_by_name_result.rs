use std::collections::HashSet;

use chrono::NaiveDate;
use somes_scraper::{LegislativeInitiative, ProContra};
use sqlx::PgPool;

use crate::{
    legislative_initiatives::match_delegates_with_name::match_delegate_names_with_delegate_ids,
    models::{DbNamedVote, DbNamedVoteInfo},
    with_data::active_delegates_at_date,
};

use super::match_delegates_with_name::LevResult;

pub async fn insert_named_vote_info(
    pg: &sqlx::PgPool,
    named_vote_info: &DbNamedVoteInfo,
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
        named_vote_info.legis_init_id,
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

pub async fn insert_named_votes(
    pg: &PgPool,
    votes: &ProContra,
    voted_on: NaiveDate,
    gp: &str,
    named_vote_info_id: i32,
) -> sqlx::Result<()> {
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
        named_vote_info_id,
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
        named_vote_info_id,
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
        named_vote_info_id,
        manually_matched: Some(false),
    });

    for named_vote in pro_iter.chain(contra_iter).chain(absent_iter) {
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
            named_vote.named_vote_info_id,
            named_vote.manually_matched
        )
        .execute(pg)
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

    println!(
        "lev_or_deep_contra: {:?}",
        contra_res.lev_or_deep_split_matched
    );

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

// /gegenstand/XXVII/A/2068

#[cfg(test)]
mod tests {
    use super::match_all_named_votes;
    use crate::connect_pg;
    use somes_scraper::{legis_init::LegislativeInitiatives, AustriaScraper};

    #[tokio::test]
    async fn test_get_legis_init_was_voted_by_name_extraction() {
        let mut scraper = AustriaScraper::default();

        let start = std::time::Instant::now();
        let pg = connect_pg().await;

        // for gp in ["XXV", "XXIV"] {
        for gp in ["XXVII", "XXVI", "XXV", "XXIV", "XXIII", "XXII", "XXI", "XX"] {
            scraper.scrape_info.gp = vec![gp.to_string()];

            let initiatives = scraper.post_legis_payload().await.unwrap();
            for path in scraper.legis_paths(initiatives).unwrap() {
                println!("path: {path:?}");
                let Ok(init) = scraper.get_legis_init(&path).await else {
                    continue;
                };
                if init.hidden_votes_info.voted_by_name {
                    let res = match_all_named_votes(
                        &pg,
                        &init
                            .hidden_votes_info
                            .voted_by_name_data
                            .unwrap()
                            .delegates_pro_contra
                            .unwrap(),
                        init.date,
                        &init.gp,
                    )
                    .await
                    .unwrap();
                    println!("res: {:?}", res.absent);

                    // println!(" {}{path}", AustriaScraper::BASE_URL,);
                }
            }
            println!("elapsed: {:?}", start.elapsed());
        }
    }

    // /gegenstand/XXVII/A/3945
    #[tokio::test]
    async fn test_namentlich_name_matching_xxvii_a_3945() {
        let scraper = AustriaScraper::default();
        let init = scraper
            .get_legis_init("/gegenstand/XXVII/A/3945")
            .await
            .unwrap();

        let pg = connect_pg().await;
        let res = match_all_named_votes(
            &pg,
            &init
                .hidden_votes_info
                .voted_by_name_data
                .unwrap()
                .delegates_pro_contra
                .unwrap(),
            init.date,
            &init.gp,
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_namentlich_name_matching_xxvii_a_2068() {
        let scraper = AustriaScraper::default();
        let init = scraper
            .get_legis_init("/gegenstand/XXVII/A/2068")
            .await
            .unwrap();

        let pg = connect_pg().await;
        let res = match_all_named_votes(
            &pg,
            &init
                .hidden_votes_info
                .voted_by_name_data
                .unwrap()
                .delegates_pro_contra
                .unwrap(),
            init.date,
            &init.gp,
        )
        .await
        .unwrap();

        // println!("res: {:?}", res.)
    }

    // //gegenstand/XXVII/I/1929
    #[tokio::test]
    async fn test_namentlich_name_matching_xxvii_i_1929() {
        let scraper = AustriaScraper::default();
        let init = scraper
            .get_legis_init("/gegenstand/XXVII/I/1929")
            .await
            .unwrap();

        let x = init
            .hidden_votes_info
            .voted_by_name_data
            .as_ref()
            .unwrap()
            .delegates_pro_contra
            .as_ref()
            .unwrap()
            .clone();
        println!("x: {x:?}");

        let pg = connect_pg().await;
        let res = match_all_named_votes(
            &pg,
            &init
                .hidden_votes_info
                .voted_by_name_data
                .unwrap()
                .delegates_pro_contra
                .unwrap(),
            init.date,
            &init.gp,
        )
        .await
        .unwrap();

        // println!("Ares: {:?}", res.)
    }

    // /gegenstand/XXV/A/2254
    #[tokio::test]
    async fn test_namentlich_name_matching_xxv_a_2254() {
        let scraper = AustriaScraper::default();
        let init = scraper
            .get_legis_init("/gegenstand/XXV/A/2254")
            .await
            .unwrap();

        let x = init
            .hidden_votes_info
            .voted_by_name_data
            .as_ref()
            .unwrap()
            .delegates_pro_contra
            .as_ref()
            .unwrap()
            .clone();
        println!("x: {x:?}");

        let pg = connect_pg().await;
        let res = match_all_named_votes(
            &pg,
            &init
                .hidden_votes_info
                .voted_by_name_data
                .unwrap()
                .delegates_pro_contra
                .unwrap(),
            init.date,
            &init.gp,
        )
        .await
        .unwrap();

        // println!("Ares: {:?}", res.)
    }
}
