mod filtering;
pub use filtering::*;

use std::{collections::HashMap, sync::Arc};

use optional_struct::optional_struct;
use serde::{Deserialize, Serialize};
use somes_common_lib::Document;
use somes_macro::{CompositeType, MeilisearchFilter};
use somes_meilisearch_filter::{FilterArgument, FilterOp};
use somes_scraper::{GovOfficial, ministrial_proposals::MinistrialProposal};
use sqlx::Type;
#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::{
    CombinedData, DbAiSummary, DbMinistrialProposalQueryMeta, FromScrapedType, Topic, today,
};

use crate::models::*;

#[optional_struct]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(Debug, Deserialize, Serialize, Clone, Hash, Type, CompositeType, MeilisearchFilter)]
pub struct GovProposal {
    #[filter(make_vec)]
    pub id: Option<i32>,

    #[filter(make_optional)]
    pub ministrial_proposal: DbMinistrialProposalQueryMeta,
    pub vote_result: Option<OptionalVoteResult>,

    #[filter(make_optional)]
    pub topics: Vec<Topic>,
    #[filter(make_optional)]
    pub eurovoc_topics: Vec<Topic>,
    #[filter(make_optional)]
    pub other_keyword_topics: Vec<Topic>,
    pub ministerial_issuers: Vec<i32>,
    #[filter(skip)]
    pub documents: Vec<Document>,
    pub ai_summary: Option<DbAiSummary>,
}

impl CombinedData for GovProposal {
    #[inline]
    fn id(&self) -> u64 {
        self.id.unwrap() as u64
    }
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
                                && ministerial_proposal.created_at.date_naive() >= role.start_date
                                && role.end_date.unwrap_or(today())
                                    >= ministerial_proposal.created_at.date_naive()
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
        let db_ministerial_proposal = DbMinistrialProposalQueryMeta {
            id: ministerial_proposal.id as i32,
            ityp: ministerial_proposal.ityp,
            gp: ministerial_proposal.gp,
            title: ministerial_proposal.title,
            description: ministerial_proposal.description,
            inr: ministerial_proposal.inr as i32,
            emphasis: ministerial_proposal.emphasis,
            raw_data_created_at: ministerial_proposal.created_at,
            raw_data_updated_at: ministerial_proposal.updated_at,
            due_to: ministerial_proposal.gegenstand_info.due_to,
            ressort: ministerial_proposal.gegenstand_info.ressort,
            ressort_shortform: ministerial_proposal.gegenstand_info.ressort_shortform,
            legis_init_gp: ministerial_proposal.legis_gp,
            legis_init_inr: ministerial_proposal.legis_init_inr.map(|x| x as i32),
            legis_init_ityp: ministerial_proposal.legis_init_ityp,
            has_vote_result: ministerial_proposal.legis_init_inr.is_some(),
            created_at: None,
            updated_at: None,
        };
        GovProposal {
            id: Some(ministerial_proposal.id as i32),
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
            ai_summary: None,
        }
    }
}
