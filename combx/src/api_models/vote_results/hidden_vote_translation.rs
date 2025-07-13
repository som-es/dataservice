use std::collections::HashMap;

use chrono::NaiveDate;
use somes_scraper::{Party, ProContra};
use sqlx::PgPool;

use crate::{DbVote, with_data::all_parties_at_date};

pub async fn translate_votes(
    parties: &HashMap<String, Party>,
    pro_contra: &ProContra,
    gp: &str,
) -> Vec<DbVote> {
    let convert_to_vote = |code: &String, infavor| {
        let Some(party) = parties.get(code).cloned() else {
            log::error!("Party with code {code} not found in {gp} parties map: {parties:?}");
            return None;
        };

        Some(DbVote {
            party: party.name,
            code: code.to_string(),
            fraction: party.fraction as i32,
            infavor,
        })
    };
    let mut votes = vec![];
    votes.extend(
        pro_contra
            .contra
            .iter()
            .flat_map(|code| convert_to_vote(code, false)),
    );
    votes.extend(
        pro_contra
            .pro
            .iter()
            .flat_map(|code| convert_to_vote(code, true)),
    );
    votes
}

pub async fn pg_gen_code_to_parties_with_date(
    pool: &PgPool,
    date: &NaiveDate,
    gp: &str,
) -> HashMap<String, Party> {
    all_parties_at_date(pool, date, gp)
        .await
        .map_err(|e| {
            log::error!("all parties for gp error in legis init: {e:?}");
            e
        })
        .unwrap_or_default()
        .into_iter()
        .map(|party| (party.code.clone(), party))
        .collect::<HashMap<String, Party>>()
}
