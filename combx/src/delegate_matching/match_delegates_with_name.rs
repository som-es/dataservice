use std::collections::HashSet;

// use crate::with_data::activity::active_delegates_at_date;

const MAX_STEPS: usize = 10000000;
// const MAX_STEPS: usize = 100000;

pub fn levenshtein_distance(lhs: &str, rhs: &str) -> usize {
    let mut steps = 0;
    fn levenshtein_distance_inner_inner(
        lhs: &[char],
        rhs: &[char],
        m: usize,
        n: usize,
        steps: &mut usize,
    ) -> usize {
        if lhs == rhs {
            return 0;
        }
        if m == 0 {
            return n;
        }
        if n == 0 {
            return m;
        }
        if lhs[m - 1] == rhs[n - 1] {
            levenshtein_distance_inner(lhs, rhs, m - 1, n - 1, steps)
        } else {
            1 + [
                levenshtein_distance_inner(lhs, rhs, m, n - 1, steps),
                levenshtein_distance_inner(lhs, rhs, m - 1, n, steps),
                levenshtein_distance_inner(lhs, rhs, m - 1, n - 1, steps),
            ]
            .iter()
            .min()
            .unwrap()
        }
    }
    fn levenshtein_distance_inner(
        lhs: &[char],
        rhs: &[char],
        m: usize,
        n: usize,
        steps: &mut usize,
    ) -> usize {
        if *steps >= MAX_STEPS {
            return usize::MAX;
        }
        let dist = levenshtein_distance_inner_inner(lhs, rhs, m, n, steps);
        *steps += 1;
        dist
    }
    let lhs = lhs.chars().collect::<Vec<_>>();
    let rhs = rhs.chars().collect::<Vec<_>>();
    levenshtein_distance_inner(&lhs, &rhs, lhs.len(), rhs.len(), &mut steps)
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct LevResult {
    pub deep_split_matched: bool,
    pub similiarity_score: usize,
    pub lev: usize,
    pub delegate_id: usize,
    pub manually_matched: bool,
    pub matched_with: String,
    pub searched_with: String,
}

pub fn match_delegate_name_with_delegate_id<'a, I: Iterator<Item = &'a (String, usize)>>(
    // delegates: &[(String, usize)],
    delegates: I,
    delegate_name: &str,
    deep_search: bool,
) -> Vec<LevResult> {
    let delegate_split = delegate_name.split(' ').collect::<Vec<_>>();
    let mut levenshtein_scores = delegates
        .into_iter()
        .cloned()
        .map(|(mut name, delegate_id)| {
            let orig_name = name.clone();
            // this removes "Dr.", "BA", etc "name, <>"
            if let Some(has_title_idx) = name.find(',') {
                name = name[..has_title_idx].to_string()
            }


            let name_split = name.split(' ').collect::<Vec<_>>();
            
            let mut deep_split_matched = false;
            let mut split_names_are_the_same = true;

            let mut deep_search_mismatch = 0;

            for delegate_name_split in &delegate_split {
                if deep_search {
                    for name in &name_split {
                        if !name.contains(delegate_name_split) {
                            deep_search_mismatch += 1;
                        } else {
                            deep_split_matched = true;
                        }
                    }
                }

                if !name_split.contains(delegate_name_split) {
                    split_names_are_the_same = false;
                }
            }

            let surname = name_split[0].to_string();
            let fullname = name;

            let def_the_same = delegate_name == surname || delegate_name == fullname;

            let lev = if deep_split_matched || split_names_are_the_same || def_the_same {
                0
            } else if delegate_split.len() > 1 && deep_search {
                levenshtein_distance(&fullname, delegate_name)
            } else if deep_search {
                levenshtein_distance(&surname, delegate_name)
            } else {
                usize::MAX - 10
            };

            let similiarity_ranking = if def_the_same {
                0
            } else if split_names_are_the_same {
                1
            } else if deep_split_matched {
                1 + deep_search_mismatch
            } else {
                lev + 3
            };

            LevResult {
                deep_split_matched,
                lev,
                delegate_id,
                matched_with: orig_name,
                searched_with: delegate_name.to_string(),
                manually_matched: false,
                similiarity_score: similiarity_ranking,
            }
        })
        .collect::<Vec<_>>();

    levenshtein_scores.sort_by(|a, b| a.similiarity_score.cmp(&b.similiarity_score));
    levenshtein_scores
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchResult {
    pub correctly_matched: Vec<LevResult>,
    pub lev_or_deep_split_matched: Vec<LevResult>,
    pub not_matchable: HashSet<(String, usize)>,
}

pub fn match_delegate_names_with_delegate_ids(
    active_delegates: &HashSet<(String, usize)>,
    delegate_names: &[String],
    gp: &str,
) -> MatchResult {
    println!("delegate_names: {delegate_names:?}");
    let mut find_not_sure_if_matched_correctly = active_delegates.clone();
    let mut prot_delegate_names_not_sure_if_matched_correctly =
        delegate_names.iter().cloned().collect::<HashSet<_>>();

    let matched_delegates = delegate_names
        .iter()
        .map(|delegate_name| {
            if gp == "XXVII" && delegate_name == "Strache" {
                return (
                    delegate_name,
                    vec![LevResult {
                        deep_split_matched: false,
                        similiarity_score: 0,
                        lev: 0,
                        delegate_id: 44127,
                        matched_with: "Beck Pia Philippa".into(),
                        searched_with: delegate_name.clone(),
                        manually_matched: true,
                    }],
                );
            }
            (
                delegate_name,
                match_delegate_name_with_delegate_id(active_delegates.iter(), delegate_name, false),
            )
        })
        // .inspect(|x| println!("done"))
        .filter_map(|(del_name, x)| {
            let res = x[0].clone();
            
            if res.lev == 0 {
                // println!("removed");
                prot_delegate_names_not_sure_if_matched_correctly.remove(del_name);
                assert!(
                    find_not_sure_if_matched_correctly
                        .remove(&(res.matched_with.clone(), res.delegate_id))
                );
                Some(res)
            } else {
                // println!("not removed");
                None
            }
        })
        .collect::<Vec<_>>();

    println!("find not: {find_not_sure_if_matched_correctly:?}");

    let mut find_not_sure_if_matched_correctly_final = find_not_sure_if_matched_correctly.clone();

    // e.g. Holzinger-Vogtenhuber Daniela, BA back then was "Holzinger"
    // (name changes etc)
    let other_matched_delegates = prot_delegate_names_not_sure_if_matched_correctly
        .clone()
        .iter()
        .map(|delegate_name| {
            (
                delegate_name.clone(),
                match_delegate_name_with_delegate_id(
                    find_not_sure_if_matched_correctly.clone().iter(),
                    &delegate_name,
                    true,
                ),
            )
        })
        .flat_map(|(del_name, x)| {
            let res = x[0].clone();
            if res.similiarity_score > 8 {
                return None;
            }
            println!(
                "other matched delegate: del_name: {del_name:?} res: {:?}",
                res
            );
            prot_delegate_names_not_sure_if_matched_correctly.remove(&del_name);
            let x = find_not_sure_if_matched_correctly_final
                    .remove(&(res.matched_with.clone(), res.delegate_id));
            assert!(x);
            Some(res)
        })
        .collect::<Vec<_>>();

    // println!("other_matched: {other_matched_delegates:?}");

    assert!(prot_delegate_names_not_sure_if_matched_correctly.is_empty());
    let not_matchable = find_not_sure_if_matched_correctly_final;

    MatchResult {
        correctly_matched: matched_delegates,
        lev_or_deep_split_matched: other_matched_delegates,
        not_matchable,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use somes_scraper::{AustriaScraper, legis_init::LegislativeInitiatives, plenar::Plenar};

    use crate::{connect_pg, reverse_name, with_data::active_delegates_at_date};

    use super::{
        levenshtein_distance, match_delegate_name_with_delegate_id,
        match_delegate_names_with_delegate_ids,
    };

    #[tokio::test]
    async fn test_namentlich_name_matching_xxv_i_1() {
        let scraper = AustriaScraper::default();
        let init = scraper.get_legis_init("/gegenstand/XXV/I/1").await.unwrap();
        println!("date: {:?}", init.nr_plenary_activity_date);
        assert!(init.hidden_votes_info.voted_by_name);
        let pro_contra = init
            .hidden_votes_info
            .voted_by_name_data
            .unwrap()
            .delegates_pro_contra
            .unwrap();

        let pg = connect_pg().await;
        let active_delegates = active_delegates_at_date(&pg, init.nr_plenary_activity_date)
            .await
            .unwrap()
            .into_iter()
            .map(|delegate| (delegate.name, delegate.id as usize))
            .collect::<HashSet<_>>();

        let delegate_names = pro_contra
            .pro
            .iter()
            .chain(pro_contra.contra.iter())
            .cloned()
            .collect::<Vec<_>>();
        let res =
            match_delegate_names_with_delegate_ids(&active_delegates, &delegate_names, &init.gp);
        // let res = res.correctly_matched.into_iter().filter(|x| x.lev > 0).collect::<Vec<_>>();
        // assert_eq!(res.not_matchable.len(), 0);
        assert_eq!(res.lev_or_deep_split_matched.len(), 2);
        println!("res: {:?}", res.lev_or_deep_split_matched);
    }

    // gegenstand/XXV/UEA/133
    #[tokio::test]
    async fn test_namentlich_name_matching_xxv_uea_133() {
        let scraper = AustriaScraper::default();
        let init = scraper
            .get_legis_init("/gegenstand/XXV/UEA/133")
            .await
            .unwrap();
        println!("date: {:?}", init.nr_plenary_activity_date);
        assert!(init.hidden_votes_info.voted_by_name);
        let pro_contra = init
            .hidden_votes_info
            .voted_by_name_data
            .unwrap()
            .delegates_pro_contra
            .unwrap();

        let pg = connect_pg().await;
        let active_delegates = active_delegates_at_date(&pg, init.nr_plenary_activity_date)
            .await
            .unwrap()
            .into_iter()
            .map(|delegate| (delegate.name, delegate.id as usize))
            .collect::<HashSet<_>>();

        let delegate_names = pro_contra
            .pro
            .iter()
            .chain(pro_contra.contra.iter())
            .cloned()
            .collect::<Vec<_>>();
        let res =
            match_delegate_names_with_delegate_ids(&active_delegates, &delegate_names, &init.gp);
        // let res = res.correctly_matched.into_iter().filter(|x| x.lev > 0).collect::<Vec<_>>();
        // assert_eq!(res.not_matchable.len(), 0);
        // assert_eq!(res.lev_or_deep_split_matched.len(), 2);
        // println!("res: {:?}", res.lev_or_deep_split_matched);
    }

    #[tokio::test]
    async fn test_named_vote_matching_xxv() {
        let mut scraper = AustriaScraper::default();
        scraper.scrape_info.gp = vec!["XXV".to_string()];
        let legis_inits = scraper
            .post_legis_payload(Some(
                &somes_scraper::legis_init::LEGIS_IS_PLENARY_VOTEABLE_ON_DOKTYPES,
            ))
            .await
            .unwrap();
        let legis_init_paths = scraper.legis_paths(legis_inits).unwrap();

        for path in legis_init_paths {
            let init = scraper.get_legis_init(&path).await.unwrap();

            if !init.hidden_votes_info.voted_by_name {
                continue;
            }

            println!("path: {path:?} date: {:?}", init.nr_plenary_activity_date);
            assert!(init.hidden_votes_info.voted_by_name);

            let Some(pro_contra) = init
                .hidden_votes_info
                .voted_by_name_data
                .unwrap()
                .delegates_pro_contra
            else {
                continue;
            };

            let pg = connect_pg().await;
            let active_delegates = active_delegates_at_date(&pg, init.nr_plenary_activity_date)
                .await
                .unwrap()
                .into_iter()
                .map(|delegate| (delegate.name, delegate.id as usize))
                .collect::<HashSet<_>>();

            let delegate_names = pro_contra
                .pro
                .iter()
                .chain(pro_contra.contra.iter())
                .cloned()
                .collect::<Vec<_>>();
            let res = match_delegate_names_with_delegate_ids(
                &active_delegates,
                &delegate_names,
                &init.gp,
            );
            // let res = res.correctly_matched.into_iter().filter(|x| x.lev > 0).collect::<Vec<_>>();
            // assert_eq!(res.not_matchable.len(), 0);
            // assert_eq!(res.lev_or_deep_split_matched.len(), 1);
            // println!("res: {:?}", res.lev_or_deep_split_matched);
        }
    }

    #[tokio::test]
    async fn test_namentlich_name_matching_xxvi_i_514() {
        // /gegenstand/XXVI/I/514
        let scraper = AustriaScraper::default();
        let init = scraper
            .get_legis_init("/gegenstand/XXVI/I/514")
            .await
            .unwrap();
        println!("date: {:?}", init.nr_plenary_activity_date);
        assert!(init.hidden_votes_info.voted_by_name);
        println!("hidden_vot_infos: {:?}", init.hidden_votes_info);
        let pro_contra = init
            .hidden_votes_info
            .voted_by_name_data
            .unwrap()
            .delegates_pro_contra
            .unwrap();

        let pg = connect_pg().await;
        let active_delegates = active_delegates_at_date(&pg, init.nr_plenary_activity_date)
            .await
            .unwrap()
            .into_iter()
            .map(|delegate| (delegate.name, delegate.id as usize))
            .collect::<HashSet<_>>();

        let res =
            match_delegate_names_with_delegate_ids(&active_delegates, &pro_contra.pro, &init.gp);

        // let res = res.correctly_matched.into_iter().filter(|x| x.lev > 0).collect::<Vec<_>>();
        // println!("res: {:?}", res.lev_or_deep_split_matched);
    }

    #[tokio::test]
    async fn test_all_plenar_absences_matching() {
        let mut scraper = AustriaScraper::default();

        // for gp in ["XXVII", "XXVI", "XXV", "XXIV", "XXIII", "XXII", "XXI", "XX"] {
        for gp in ["XXI", "XX"] {
            scraper.scrape_info.gp = vec![gp.to_string()];

            let raw_plenar = scraper.post_plenar_payload().await.unwrap();
            for path in scraper.plenar_paths(raw_plenar).unwrap() {
                let plenar_info = scraper.get_plenar(&path).await.unwrap();

                if plenar_info.absences.is_none() {
                    println!("skip {path}");
                    continue;
                }

                // println!("plenar_info: {plenar_info:?}");

                let pg = connect_pg().await;
                let active_delegates =
                    active_delegates_at_date(&pg, plenar_info.created_at.date_naive())
                        .await
                        .unwrap()
                        .into_iter()
                        .map(|delegate| (delegate.name, delegate.id as usize))
                        .collect::<HashSet<_>>();

                let res = match_delegate_names_with_delegate_ids(
                    &active_delegates,
                    &plenar_info
                        .absences
                        .as_ref()
                        .unwrap()
                        .iter()
                        .map(|x| x.as_str())
                        .map(reverse_name)
                        .collect::<Vec<_>>(),
                    &plenar_info.gp,
                );

                println!(
                    "res correctly matched: {:?}",
                    res.correctly_matched
                        .iter()
                        .map(|x| x.matched_with.clone())
                        .collect::<Vec<_>>()
                );
                println!(
                    "res deep split matched: {:?}",
                    res.lev_or_deep_split_matched
                        .iter()
                        .map(|x| (x.lev, x.searched_with.clone(), x.matched_with.clone()))
                        .collect::<Vec<_>>()
                );
            }
        }
    }

    #[tokio::test]
    async fn test_plenar_absences_matching() {
        let scraper = AustriaScraper::default();

        let plenar_info = scraper
            .get_plenar("/gegenstand/XXVII/NRSITZ/151")
            .await
            .unwrap();
        println!("plenar_info: {plenar_info:?}");

        let pg = connect_pg().await;
        let active_delegates = active_delegates_at_date(&pg, plenar_info.created_at.date_naive())
            .await
            .unwrap()
            .into_iter()
            .map(|delegate| (delegate.name, delegate.id as usize))
            .collect::<HashSet<_>>();

        let res = match_delegate_names_with_delegate_ids(
            &active_delegates,
            &plenar_info
                .absences
                .as_ref()
                .unwrap()
                .iter()
                .map(|x| x.as_str())
                .map(reverse_name)
                .collect::<Vec<_>>(),
            &plenar_info.gp,
        );

        println!("res: {:?}", res.correctly_matched);
        println!("res: {:?}", res.lev_or_deep_split_matched);
    }

    #[test]
    fn test_distance_delegate_match() {
        let delegates = vec![
            ("Amon Werner", 1),
            ("Belakowitsch Dagmar", 2),
            ("Berlakovich Nikolaus", 3),
            ("Bacher Walter", 4),
            ("Bacher Stefan", 4),
            ("Becher Ruth", 5),
            ("Duzdar Muna", 6),
        ]
        .into_iter()
        .map(|(a, b)| (a.to_string(), b))
        .collect::<Vec<_>>();
        // .iter()
        // .map(|(name, id)| {
        //     let name = name.split(' ').collect::<Vec<_>>();
        //     let firstname = name[1].to_string();
        //     let surname = name[0].to_string();
        //     (firstname, surname, *id)
        // })
        // .collect::<Vec<_>>();
        let delegate_name = "Bacher Stefono";
        let scores = match_delegate_name_with_delegate_id(delegates.iter(), delegate_name, false);
        print!("scores: {scores:?}");
    }

    #[test]
    fn test_calc_levensthein_distance_with_2_byte_thing() {
        let lhs = "abgegene";
        let rhs = "aöbgegebene";

        assert_eq!(levenshtein_distance(&lhs, &rhs), 3);
    }
    #[test]
    fn test_calc_levensthein_distance() {
        let lhs = "abgegene";
        let rhs = "abgegebene";

        assert_eq!(levenshtein_distance(lhs, rhs,), 2);
    }
}
