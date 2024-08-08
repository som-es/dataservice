use std::collections::HashSet;

use chrono::NaiveDate;
use scraper::ProContra;
use sqlx::PgPool;

use crate::{
    legislative_initiatives::match_delegates_with_name::match_delegate_names_with_delegate_ids,
    with_data::activity::active_delegates_at_date,
};

use super::match_delegates_with_name::LevResult;

pub struct VotedByNameResult {
    pro: Vec<LevResult>,
    contra: Vec<LevResult>,
    absent: HashSet<(String, usize)>,
}

pub async fn match_all_named_votes(
    pg: &PgPool,
    votes: &ProContra,
    voted_on: NaiveDate,
) -> sqlx::Result<VotedByNameResult> {
    let active_delegates = active_delegates_at_date(pg, voted_on)
        .await
        .unwrap()
        .into_iter()
        .map(|delegate| (delegate.name, delegate.id as usize))
        // .inspect(|x| println!("{x:?}"))
        .collect::<HashSet<_>>();

    let pro_res = match_delegate_names_with_delegate_ids(&active_delegates, &votes.pro);
    let mut pro = pro_res.correctly_matched;
    pro.extend_from_slice(&pro_res.lev_or_deep_split_matched);
    println!("lev_or_deep_pro: {:?}", pro_res.lev_or_deep_split_matched);

    let contra_res = match_delegate_names_with_delegate_ids(&active_delegates, &votes.contra);
    let mut contra = contra_res.correctly_matched;
    contra.extend_from_slice(&contra_res.lev_or_deep_split_matched);

    println!("lev_or_deep_contra: {:?}", contra_res.lev_or_deep_split_matched);

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
    use scraper::{legis_init::LegislativeInitiatives, AustriaScraper};

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

        let x = init.hidden_votes_info.voted_by_name_data.as_ref().unwrap().delegates_pro_contra.as_ref().unwrap().clone();
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

        let x = init.hidden_votes_info.voted_by_name_data.as_ref().unwrap().delegates_pro_contra.as_ref().unwrap().clone();
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
        )
        .await
        .unwrap();

        // println!("Ares: {:?}", res.)  
    }
}
