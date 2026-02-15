use std::collections::HashMap;

use chrono::NaiveDate;
use somes_scraper::{Party, ProContra};
use sqlx::PgPool;

use crate::{DbVote, with_data::all_parties_at_date};

pub async fn translate_votes(
    all_parties: &Vec<Party>,
    parties_at_date: &HashMap<String, Party>,
    pro_contra: &ProContra,
    gp: &str,
) -> Vec<DbVote> {
    let all_party_name_to_party = all_parties
        .iter()
        .map(|party| (party.name.clone(), party.clone()))
        .collect::<HashMap<String, Party>>();

    let party_name_to_party = parties_at_date
        .values()
        .map(|party| (party.name.clone(), party.clone()))
        .collect::<HashMap<String, Party>>();

    let convert_code_to_vote = |code: &String, infavor| {
        let Some(party) = parties_at_date.get(code).cloned() else {
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
        let name = if ["XX", "XXI", "XXII"].contains(&gp) && name == "FPÖ" {
            "F"
        } else {
            name
        };
        let Some(party) = party_name_to_party.get(name).cloned() else {
            log::error!(
                "Could not find party by name '{}' for gp '{}' for vote translation. Available parties: {:?}. Trying all parties next.",
                name,
                gp,
                parties_at_date
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
        maybe_convert_vote(
            &all_party_name_to_party,
            &convert_code_to_vote,
            &convert_party_name_to_vote,
            maybe_code,
            false,
        )
    }));
    votes.extend(pro_contra.pro.iter().flat_map(|code| {
        maybe_convert_vote(
            &all_party_name_to_party,
            &convert_code_to_vote,
            &convert_party_name_to_vote,
            code,
            true,
        )
    }));
    votes
}

fn maybe_convert_vote(
    all_party_name_to_party: &HashMap<String, Party>,
    convert_code_to_vote: &impl Fn(&String, bool) -> Option<DbVote>,
    convert_party_name_to_vote: &impl Fn(&String, bool) -> Option<DbVote>,
    maybe_code: &String,
    infavor: bool,
) -> Option<DbVote> {
    match convert_code_to_vote(maybe_code, infavor) {
        Some(vote) => Some(vote),
        None => match convert_party_name_to_vote(maybe_code, infavor) {
            Some(vote) => Some(vote),
            None => {
                let party_code = all_party_name_to_party
                    .get(maybe_code)
                    .map(|party| party.code.clone());
                let Some(party_code) = party_code else {
                    log::error!(
                        "Could not name->code '{}' for vote translation. Available parties: {:?}",
                        maybe_code,
                        all_party_name_to_party
                    );
                    return None;
                };
                match convert_code_to_vote(&party_code, infavor) {
                    Some(vote) => Some(vote),
                    None => {
                        log::error!(
                            "Could not get party vote for {maybe_code}->{}  for vote translation. Available parties: {:?}",
                            party_code,
                            all_party_name_to_party
                        );
                        None
                    }
                }
            }
        },
    }
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

        let votes = translate_votes(&vec![], &parties, &pro_contra, "gp1").await;

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

        let votes = translate_votes(&vec![], &parties, &pro_contra, "gp1").await;

        assert_eq!(votes.len(), 2);
        assert!(votes.iter().any(|v| v.party == "Alpha" && v.infavor));
        assert!(votes.iter().any(|v| v.party == "Beta" && !v.infavor));
    }

    #[tokio::test]
    async fn test_translate_votes_with_invalid_party_name_at_date_thanks_parlament_gv_at() {
        let parties_at_date = vec![
            Party {
                code: "A".to_string(),
                name: "Alpha-Gamma".to_string(),
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

        let all_parties = vec![
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
            Party {
                code: "A".to_string(),
                name: "Alpha-Gamma".to_string(),
                fraction: 1,
                color: "#FF0000".to_string(),
            },
        ];

        let pro_contra = ProContra {
            pro: vec!["Alpha".to_string()],
            contra: vec!["Beta".to_string()],
        };

        let votes = translate_votes(&all_parties, &parties_at_date, &pro_contra, "gp1").await;

        assert_eq!(votes.len(), 2);
        assert!(votes.iter().any(|v| v.party == "Alpha-Gamma" && v.infavor));
        assert!(votes.iter().any(|v| v.party == "Beta" && !v.infavor));
    }
}
