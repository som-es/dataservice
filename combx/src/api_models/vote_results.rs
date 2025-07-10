mod hidden_vote_translation;
mod named_votes;

use std::collections::HashMap;

use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};
use somes_scraper::{
    AcceptedState, ComeIntoEffectInfo, Document, LegislativeInitiative, Party, ProContra, Proposal,
    Reference, RelatedDelegate, Vote, VoteStage,
};
use sqlx::{FromRow, PgPool};

use crate::{
    api_models::vote_results::{hidden_vote_translation::{
        gen_code_to_parties_with_date, translate_votes,
    }, named_votes::create_db_named_votes},
    models::{
        DbLegisDocumentOptional, DbLegislativeInitiativeQuery, DbNamedVotes, DbSpeechWithLink,
        DbVote, Topic,
    },
    with_data::{all_parties_at_date, all_parties_at_gp}, DbNamedVoteInfo,
};

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(Debug, Deserialize, Serialize, Clone, FromRow, Default)]
pub struct MeiliesearchHelper {
    #[serde(default)]
    pub votes: Vec<String>,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(Debug, Deserialize, Serialize, Clone, FromRow, Default)]
pub struct VoteResult {
    pub id: i32,
    pub legislative_initiative: DbLegislativeInitiativeQuery,
    pub votes: Vec<DbVote>,
    pub speeches: Vec<DbSpeechWithLink>,
    pub named_votes: Option<DbNamedVotes>,
    pub topics: Vec<Topic>,
    pub eurovoc_topics: Vec<Topic>,
    pub other_keyword_topics: Vec<Topic>,
    pub documents: Vec<DbLegisDocumentOptional>,
    pub absences: Vec<i32>,
    pub issued_by_dels: Vec<RelatedDelegate>,
    pub referenced_by_others_ids: Vec<i32>,
    pub references: Vec<i32>,
    #[serde(default)]
    pub meilisearch_helper: MeiliesearchHelper,
}

impl VoteResult {
    pub async fn new(pool: &PgPool, legis_init: LegislativeInitiative) -> Self {
        let plenary_session_id = match legis_init.plenary_session {
            Some(session) => sqlx::query!(
                "select id from plenar_infos where inr = $1 and legislative_period = $2",
                session as i32,
                legis_init.gp
            )
            .fetch_one(pool)
            .await
            .ok()
            .map(|id| id.id),
            None => None,
        };

        let db_legis_init = DbLegislativeInitiativeQuery {
            id: 0,
            title: legis_init.title.clone(),
            inr: legis_init.inr as i32,
            emphasis: legis_init.emphasis.clone(),
            description: legis_init.description.clone(),
            ityp: legis_init.ityp.clone(),
            gp: legis_init.gp.clone(),
            accepted: legis_init.accepted.map(|a| a.to_string()),
            created_at: legis_init.date,
            appeared_at: Some(legis_init.created_at),
            updated_at: legis_init.updated_at,
            requires_simple_majority: legis_init.requires_simple_majority,
            pre_declined_type: legis_init
                .hidden_votes_info
                .vote_stage
                .map(|pre| pre.to_string()),
            voted_by_name: legis_init.hidden_votes_info.voted_by_name,
            plenary_session_id,
            doktyp: legis_init.doktyp.clone(),
            is_law: legis_init.is_law.clone(),
            law_accepted: legis_init.law_accepted.clone(),
            law_come_into_effect_date: legis_init
                .come_into_effect_info
                .as_ref()
                .map(|x| x.law_come_into_effect_date.clone())
                .flatten(),
            law_expires_on_date: legis_init
                .come_into_effect_info
                .as_ref()
                .map(|x| x.law_expires_on_date.clone())
                .flatten(),
            by_publication: legis_init
                .come_into_effect_info
                .as_ref()
                .map(|x| x.by_publication),
            has_reference: Some(
                legis_init
                    .references
                    .as_ref()
                    .map(|x| !x.is_empty())
                    .unwrap_or_default(),
            ),
            is_voteable_on: Some(legis_init.is_voteable_on),
            is_urgent: legis_init.is_urgent,
            is_emphasis_ai_generated: false,
            // law_come_into_effect_date: legis_init.come_into_effect_info.map(|x| x.law_come_into_effect_date.clone()),
        };

        let mut votes = legis_init
            .votes
            .iter()
            .map(|vote| DbVote {
                party: vote.party.name.clone(),
                code: vote.party.code.clone(),
                fraction: vote.party.fraction as i32,
                infavor: vote.infavor,
            })
            .collect::<Vec<_>>();

        let code_to_parties =
            gen_code_to_parties_with_date(pool, &db_legis_init.created_at, &db_legis_init.gp).await;

        if legis_init.votes.is_empty()
            // && legis_init.accepted == Some(somes_scraper::AcceptedState::PreDeclined)
            && legis_init.hidden_votes_info.pro_contra_parties.is_some()
        {
            match &legis_init.hidden_votes_info.pro_contra_parties {
                Some(pro_contra_parties) => {
                    votes =
                        translate_votes(&code_to_parties, pro_contra_parties, &legis_init.gp).await;
                }
                None => log::warn!(
                    "Pro Contra is empty despite hidden vote info for legis init: {}, {}, {}",
                    legis_init.gp,
                    legis_init.ityp,
                    legis_init.inr
                ),
            }
        }

        let named_votes = match &legis_init.hidden_votes_info.voted_by_name_data {
            Some(voted_by_name_data) => {
                
                let named_vote_info = DbNamedVoteInfo {
                    pro_count: voted_by_name_data.pro_count as i32,
                    contra_count: voted_by_name_data.contra_count as i32,
                    given_vote_sum: voted_by_name_data.vote_sum as i32,
                    invalid_count: voted_by_name_data.invalid_count as i32,
                };
                
                let named_votes = match &voted_by_name_data.delegates_pro_contra {
                    Some(pro_contra) => {
                        create_db_named_votes(pool, pro_contra, legis_init.date, &legis_init.gp).await.ok()
                    },
                    None => None,
                };
                Some(DbNamedVotes {
                    named_vote_info,
                    named_votes: named_votes.unwrap_or_default(),
                })
            },
            _ => None,
        };

        todo!()
    }
}
