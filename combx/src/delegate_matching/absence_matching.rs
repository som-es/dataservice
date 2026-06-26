use std::collections::HashSet;

use chrono::NaiveDate;
use sqlx::{Executor, PgPool, Postgres};

use crate::with_data::active_delegates_at_date;

use super::match_delegates_with_name::{MatchResult, match_delegate_names_with_delegate_ids};

pub fn reverse_name(name: &str) -> String {
    let name = name.split_inclusive(' ').collect::<Vec<_>>();
    if name.len() > 1 {
        format!(
            "{} {}",
            name.last().unwrap(),
            name[..name.len() - 1]
                .iter()
                .fold(String::new(), |acc, x| format!("{acc}{x}"))
        )
        .trim_matches(char::is_whitespace)
        .to_string()
    } else {
        name[0].to_string()
    }
}

pub async fn absence_matching<'a>(
    pool: impl Executor<'a, Database = Postgres>,
    absences_names: &[String],
    plenar_date: NaiveDate,
    gp: String,
    function: &str,
) -> crate::Result<MatchResult> {
    let active_delegates = active_delegates_at_date(pool, plenar_date, function)
        .await?
        .into_iter()
        .collect::<HashSet<_>>();

    let absences_names_owned = absences_names.to_vec();

    Ok(tokio::task::spawn_blocking(move || {
        let reversed_names = absences_names_owned
            .iter()
            .map(|x| x.as_str())
            .map(reverse_name)
            .collect::<Vec<_>>();

        match_delegate_names_with_delegate_ids(&active_delegates, &reversed_names, &gp, plenar_date)
    })
    .await?)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use somes_scraper::{AustriaScraper, plenar::Plenar};

    use crate::{
        LevResult, connect_pg, match_delegate_names_with_delegate_ids, reverse_name,
        with_data::active_delegates_at_date,
    };

    #[tokio::test]
    async fn test_absence_extraction_xxiv_29() {
        let scraper = AustriaScraper::default();

        let plenar_info = scraper
            .get_plenar("/gegenstand/XXIV/NRSITZ/29")
            .await
            .unwrap();
        println!("plenar_info: {plenar_info:?}");

        let pg = connect_pg().await;
        let active_delegates = active_delegates_at_date(
            &pg,
            plenar_info.created_at.date_naive(),
            &plenar_info.council,
        )
        .await
        .unwrap()
        .into_iter()
        .collect::<HashSet<_>>();

        let res = match_delegate_names_with_delegate_ids(
            &active_delegates,
            &&plenar_info
                .absences
                .as_ref()
                .unwrap()
                .absences
                .iter()
                .map(|x| x.as_str())
                .map(reverse_name)
                .collect::<Vec<_>>(),
            &plenar_info.gp,
            plenar_info.created_at.date_naive(),
        );

        println!("res: {:?}", res.correctly_matched);
        println!("res: {:?}", res.lev_or_deep_split_matched);
        assert!(res.lev_or_deep_split_matched.is_empty())
    }

    // /gegenstand/XXVII/NRSITZ/243

    #[tokio::test]
    async fn test_absence_extraction_xxvii_243() {
        let scraper = AustriaScraper::default();

        let plenar_info = scraper
            .get_plenar("/gegenstand/XXVII/NRSITZ/243")
            .await
            .unwrap();
        println!("plenar_info: {plenar_info:?}");

        let pg = connect_pg().await;
        let active_delegates =
            active_delegates_at_date(&pg, plenar_info.created_at.date_naive(), "NR")
                .await
                .unwrap()
                .into_iter()
                .collect::<HashSet<_>>();

        let res = match_delegate_names_with_delegate_ids(
            &active_delegates,
            &&plenar_info
                .absences
                .as_ref()
                .unwrap()
                .absences
                .iter()
                .map(|x| x.as_str())
                .map(reverse_name)
                .collect::<Vec<_>>(),
            &plenar_info.gp,
            plenar_info.created_at.date_naive(),
        );

        println!("res: {:?}", res.correctly_matched);
        println!("res: {:?}", res.lev_or_deep_split_matched);
        assert!(res.lev_or_deep_split_matched.is_empty());
    }

    #[tokio::test]
    async fn test_absence_extraction_br_979() {
        let scraper = AustriaScraper::default();

        let plenar_info = scraper
            .get_plenar("/gegenstand/BR/BRSITZ/979")
            .await
            .unwrap();
        println!("plenar_info: {plenar_info:?}");
        let src = &plenar_info.absences.as_ref().unwrap().source_document;
        dbg!(&src);

        let pg = connect_pg().await;
        let active_delegates =
            active_delegates_at_date(&pg, plenar_info.created_at.date_naive(), "BR")
                .await
                .unwrap()
                .into_iter()
                .collect::<HashSet<_>>();

        let res = match_delegate_names_with_delegate_ids(
            &active_delegates,
            &&plenar_info
                .absences
                .as_ref()
                .unwrap()
                .absences
                .iter()
                .map(|x| x.as_str())
                .map(reverse_name)
                .collect::<Vec<_>>(),
            &plenar_info.gp,
            plenar_info.created_at.date_naive(),
        );

        println!("res: {:?}", res.correctly_matched);
        assert_eq!(
            res.correctly_matched,
            [
                LevResult {
                    deep_split_matched: false,
                    similiarity_score: 0,
                    lev: 0,
                    delegate_id: 23967,
                    manually_matched: false,
                    matched_with: "Jagl Simone".into(),
                    searched_with: "Jagl Simone".into()
                },
                LevResult {
                    deep_split_matched: false,
                    similiarity_score: 0,
                    lev: 0,
                    delegate_id: 31963,
                    manually_matched: false,
                    matched_with: "Kober Herbert".into(),
                    searched_with: "Kober Herbert".into()
                }
            ]
        );
        assert!(res.lev_or_deep_split_matched.is_empty());
    }

    #[tokio::test]
    async fn test_absence_extraction_br_991() {
        let scraper = AustriaScraper::default();

        let plenar_info = scraper
            .get_plenar("/gegenstand/BR/BRSITZ/991")
            .await
            .unwrap();
        println!("plenar_info: {plenar_info:?}");

        let pg = connect_pg().await;
        let active_delegates =
            active_delegates_at_date(&pg, plenar_info.created_at.date_naive(), "BR")
                .await
                .unwrap()
                .into_iter()
                .collect::<HashSet<_>>();

        let res = match_delegate_names_with_delegate_ids(
            &active_delegates,
            &&plenar_info
                .absences
                .as_ref()
                .unwrap()
                .absences
                .iter()
                .map(|x| x.as_str())
                .map(reverse_name)
                .collect::<Vec<_>>(),
            &plenar_info.gp,
            plenar_info.created_at.date_naive(),
        );

        println!("res: {:?}", res.correctly_matched);
        assert_eq!(
            res.correctly_matched,
            vec![
                LevResult {
                    deep_split_matched: false,
                    similiarity_score: 0,
                    lev: 0,
                    delegate_id: 86618,
                    manually_matched: false,
                    matched_with: "Samt Peter".into(),
                    searched_with: "Samt Peter".into()
                },
                LevResult {
                    deep_split_matched: false,
                    similiarity_score: 0,
                    lev: 0,
                    delegate_id: 24111,
                    manually_matched: false,
                    matched_with: "Theuermann Isabella, Mag.".into(),
                    searched_with: "Theuermann Isabella".into()
                },
                LevResult {
                    deep_split_matched: false,
                    similiarity_score: 0,
                    lev: 0,
                    delegate_id: 39606,
                    manually_matched: false,
                    matched_with: "Viertler Wolfgang, Dr.".into(),
                    searched_with: "Viertler Wolfgang".into()
                }
            ]
        );
        println!("res: {:?}", res.lev_or_deep_split_matched);
        assert!(res.lev_or_deep_split_matched.is_empty());
    }
}
