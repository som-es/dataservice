use std::collections::HashSet;

use chrono::NaiveDate;
use sqlx::PgPool;

use crate::with_data::active_delegates_at_date;

use super::match_delegates_with_name::{match_delegate_names_with_delegate_ids, MatchResult};

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

pub async fn absence_matching(
    pg: &PgPool,
    absences_names: &[String],
    plenar_date: NaiveDate,
    gp: String,
) -> MatchResult {
    let active_delegates = active_delegates_at_date(pg, plenar_date)
        .await
        .unwrap()
        .into_iter()
        .map(|delegate| (delegate.name, delegate.id as usize))
        .collect::<HashSet<_>>();

    match_delegate_names_with_delegate_ids(
        &active_delegates,
        &absences_names
            .iter()
            .map(|x| x.as_str())
            .map(reverse_name)
            .collect::<Vec<_>>(),
        &gp,
    )
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use somes_scraper::{plenar::Plenar, AustriaScraper};

    use crate::{
        connect_pg,
        legislative_initiatives::{
            match_delegates_with_name::match_delegate_names_with_delegate_ids, reverse_name,
        },
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
        let active_delegates = active_delegates_at_date(&pg, plenar_info.date.date())
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
}
