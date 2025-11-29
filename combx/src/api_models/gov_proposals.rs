use std::{collections::HashMap, sync::Arc};

use serde::{Deserialize, Serialize};
use somes_common_lib::Document;
use somes_scraper::{GovOfficial, ministrial_proposals::MinistrialProposal};
#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::{
    DbMinistrialProposalQuery, FromScrapedType, OptionalVoteResult, Topic, VoteResult, today,
};

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GovProposal {
    pub ministrial_proposal: DbMinistrialProposalQuery,
    pub vote_result: Option<OptionalVoteResult>,
    pub topics: Vec<Topic>,
    pub eurovoc_topics: Vec<Topic>,
    pub other_keyword_topics: Vec<Topic>,
    pub ministerial_issuers: Vec<i32>,
    pub documents: Vec<Document>,
}

impl<T> FromScrapedType<T> for GovProposal {
    type ScrapedType = (MinistrialProposal, Arc<HashMap<u64, Vec<GovOfficial>>>);

    async fn from_scraped_type(
        _db_enhancer: T,
        (ministerial_proposal, gov_officials): Self::ScrapedType,
    ) -> Self {
        GovProposal::from_ministerial_proposal(ministerial_proposal, gov_officials)
    }
}

impl GovProposal {
    pub fn from_ministerial_proposal(
        ministerial_proposal: MinistrialProposal,
        gov_officials: Arc<HashMap<u64, Vec<GovOfficial>>>,
    ) -> Self {
        let ministerial_issuers: Vec<i32> = if ministerial_proposal.delegate_ids.is_empty() {
            gov_officials
                .iter()
                .find(|(_, roles)| {
                    roles
                        .iter()
                        .find(|role| {
                            role.ministry == ministerial_proposal.gegenstand_info.ressort
                                && ministerial_proposal.created_at.date() >= role.start_date
                                && role.end_date.unwrap_or(today())
                                    >= ministerial_proposal.created_at.date()
                        })
                        .is_some()
                })
                .map(|(id, _value)| vec![*id])
                .unwrap_or_default()
        } else {
            ministerial_proposal.delegate_ids
        }
        .into_iter()
        .map(|x| x as i32)
        .collect();
        let db_ministerial_proposal = DbMinistrialProposalQuery {
            id: ministerial_proposal.id as i32,
            ityp: ministerial_proposal.ityp,
            gp: ministerial_proposal.gp,
            title: ministerial_proposal.title,
            description: ministerial_proposal.description,
            delegate_id: ministerial_issuers[0],
            inr: ministerial_proposal.inr as i32,
            emphasis: ministerial_proposal.emphasis,
            created_at: ministerial_proposal.created_at,
            updated_at: ministerial_proposal.updated_at,
            due_to: ministerial_proposal.gegenstand_info.due_to,
            ressort: ministerial_proposal.gegenstand_info.ressort,
            ressort_shortform: ministerial_proposal.gegenstand_info.ressort_shortform,
            legis_init_gp: ministerial_proposal.legis_gp,
            legis_init_inr: ministerial_proposal.legis_init_inr.map(|x| x as i32),
            legis_init_ityp: ministerial_proposal.legis_init_ityp,
            has_vote_result: ministerial_proposal.legis_init_inr.is_some(),
        };
        GovProposal {
            ministrial_proposal: db_ministerial_proposal,
            vote_result: None,
            topics: ministerial_proposal
                .topics
                .into_iter()
                .map(|topic| Topic { topic })
                .collect(),
            eurovoc_topics: ministerial_proposal
                .eurovoc_topics
                .into_iter()
                .map(|topic| Topic { topic })
                .collect(),
            other_keyword_topics: ministerial_proposal
                .other_keyword_topics
                .into_iter()
                .map(|topic| Topic { topic })
                .collect(),
            ministerial_issuers,
            documents: ministerial_proposal
                .documents
                .into_iter()
                .map(|document| Document {
                    title: Some(document.title),
                    document_url: Some(document.link),
                    document_type: Some(document.ftype.to_string()),
                })
                .collect(),
        }
    }
}
