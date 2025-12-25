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
    let party_name_to_party = parties
        .values()
        .map(|party| (party.name.clone(), party.clone()))
        .collect::<HashMap<String, Party>>();

    let convert_code_to_vote = |code: &String, infavor| {
        let Some(party) = parties.get(code).cloned() else {
            return None;
        };

        Some(DbVote {
            party: party.name,
            code: Some(code.to_string()),
            fraction: party.fraction as i32,
            infavor,
        })
    };

    let convert_party_name_to_vote = |name: &String, infavor| {
        let Some(party) = party_name_to_party.get(name).cloned() else {
            log::error!(
                "Could not find party by name '{}' for gp '{}' for vote translation. Available parties: {:?}",
                name,
                gp,
                parties
            );
            return None;
        };

        Some(DbVote {
            party: party.name,
            code: Some(party.code),
            fraction: party.fraction as i32,
            infavor,
        })
    };

    let mut votes = vec![];
    votes.extend(pro_contra.contra.iter().flat_map(|maybe_code| {
        match convert_code_to_vote(maybe_code, false) {
            Some(vote) => Some(vote),
            None => {
                convert_party_name_to_vote(maybe_code, false)
            }
        }
    }));
    votes.extend(
        pro_contra
            .pro
            .iter()
            .flat_map(|code| match convert_code_to_vote(code, true) {
                Some(vote) => Some(vote),
                None => {
                    convert_party_name_to_vote(code, true) 
                }
            }),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_translate_votes() {
        let parties = vec![
            Party {
                code: "A".to_string(),
                name: "Alpha".to_string(),
                fraction: 1,
                color: "#FF0000".to_string(),
            },
            Party {
                code: "B".to_string(),
                name: "Beta".to_string(),
                fraction: 2,
                color: "#00FF00".to_string(),
            },
        ]
        .into_iter()
        .map(|p| (p.code.clone(), p))
        .collect::<HashMap<String, Party>>();

        let pro_contra = ProContra {
            pro: vec!["A".to_string()],
            contra: vec!["B".to_string()],
        };

        let votes = translate_votes(&parties, &pro_contra, "gp1").await;

        assert_eq!(votes.len(), 2);
        assert!(votes.iter().any(|v| v.party == "Alpha" && v.infavor));
        assert!(votes.iter().any(|v| v.party == "Beta" && !v.infavor));
    }

    #[tokio::test]
    async fn test_translate_votes_with_party_name_thanks_parlament_gv_at() {
        let parties = vec![
            Party {
                code: "A".to_string(),
                name: "Alpha".to_string(),
                fraction: 1,
                color: "#FF0000".to_string(),
            },
            Party {
                code: "B".to_string(),
                name: "Beta".to_string(),
                fraction: 2,
                color: "#00FF00".to_string(),
            },
        ]
        .into_iter()
        .map(|p| (p.code.clone(), p))
        .collect::<HashMap<String, Party>>();

        let pro_contra = ProContra {
            pro: vec!["Alpha".to_string()],
            contra: vec!["Beta".to_string()],
        };

        let votes = translate_votes(&parties, &pro_contra, "gp1").await;

        assert_eq!(votes.len(), 2);
        assert!(votes.iter().any(|v| v.party == "Alpha" && v.infavor));
        assert!(votes.iter().any(|v| v.party == "Beta" && !v.infavor));
    }
}
