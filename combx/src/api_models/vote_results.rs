mod enhancer;
mod hidden_vote_translation;
mod models;
pub use models::*;
use somes_macro::CompositeType;
mod named_votes;

use std::sync::Arc;

pub use enhancer::*;
use optional_struct::optional_struct;
use serde::{Deserialize, Serialize};
use somai_client::SomAiClient;
use somes_common_lib::Document;
use somes_scraper::{LegislativeInitiative, Opinion};
use sqlx::{FromRow, PgPool, prelude::Type};

use crate::{
    FromScrapedType, OptionalTopic, Topic,
    api_models::vote_results::hidden_vote_translation::translate_votes,
};
#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[optional_struct]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(
    Debug,
    Deserialize,
    Serialize,
    Clone,
    FromRow,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    CompositeType,
    Type,
)]
pub struct MeilisearchHelper {
    #[serde(default)]
    pub votes: Vec<String>,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(Debug, Deserialize, Default, Serialize, Clone, FromRow)]
pub struct VoteResult {
    pub id: Option<i32>,
    pub legislative_initiative: DbLegislativeInitiativeQuery,
    pub votes: Vec<DbVote>,
    pub speeches: Vec<DbSpeechWithLink>,
    pub named_votes: Option<DbNamedVotes>,
    pub topics: Vec<Topic>,
    pub eurovoc_topics: Vec<Topic>,
    pub other_keyword_topics: Vec<Topic>,
    pub documents: Vec<Document>,
    pub absences: Option<Vec<i32>>,
    pub issued_by_dels: Vec<DbRelatedDelegate>,
    pub referenced_by_others_ids: Option<Vec<i32>>,
    pub references: Option<Vec<DbReference>>,
    #[serde(default)]
    pub meilisearch_helper: MeilisearchHelper,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(Debug, Deserialize, Serialize, PartialEq, Clone, FromRow, Default)]
pub struct OptionalVoteResult {
    pub id: Option<i32>,
    pub legislative_initiative: Option<OptionalDbLegislativeInitiativeQuery>,
    pub votes: Option<Vec<OptionalDbVote>>,
    pub speeches: Option<Vec<OptionalDbSpeechWithLink>>,
    pub named_votes: Option<OptionalDbNamedVotes>,
    pub topics: Option<Vec<OptionalTopic>>,
    pub eurovoc_topics: Option<Vec<OptionalTopic>>,
    pub other_keyword_topics: Option<Vec<OptionalTopic>>,
    pub documents: Option<Vec<Document>>,
    pub absences: Option<Vec<i32>>,
    pub issued_by_dels: Option<Vec<OptionalDbRelatedDelegate>>,
    pub referenced_by_others_ids: Option<Vec<i32>>,
    pub references: Option<Vec<OptionalDbReference>>,
    #[serde(default)]
    pub meilisearch_helper: Option<OptionalMeilisearchHelper>,
}

impl<T: VoteResultEnhancer> FromScrapedType<T> for VoteResult {
    type ScrapedType = LegislativeInitiative;

    async fn from_scraped_type(db_enhancer: T, ty: Self::ScrapedType) -> Self {
        Self::from_legis_init(db_enhancer, ty, None).await
    }
}

impl VoteResult {
    // move to other crate
    pub async fn from_db(pool: &PgPool, legis_init_id: i32) -> Self {
        todo!()
    }
    pub async fn from_legis_init(
        db_enhancer: impl VoteResultEnhancer,
        legis_init: LegislativeInitiative,
        somai_client: Option<Arc<SomAiClient>>,
    ) -> Self {
        let plenary_session_id = db_enhancer.retrieve_plenary_session_id(&legis_init).await;

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
            is_voteable_on: legis_init.is_voteable_on,
            is_urgent: legis_init.is_urgent,
            is_emphasis_ai_generated: false,
            voting: legis_init.voting.map(|voting| voting.to_string()),
            // law_come_into_effect_date: legis_init.come_into_effect_info.map(|x| x.law_come_into_effect_date.clone()),
        };

        let mut votes = legis_init
            .votes
            .iter()
            .map(|vote| DbVote {
                party: vote.party.name.clone(),
                code: Some(vote.party.code.clone()),
                fraction: vote.party.fraction as i32,
                infavor: vote.infavor,
            })
            .collect::<Vec<_>>();

        let code_to_parties = db_enhancer
            .gen_code_to_parties_with_date(&db_legis_init.created_at, &db_legis_init.gp)
            .await;

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
                    Some(pro_contra) => db_enhancer
                        .create_db_named_votes(pro_contra, legis_init.date, &legis_init.gp)
                        .await
                        .ok(),
                    None => None,
                };
                Some(DbNamedVotes {
                    named_vote_info,
                    named_votes: named_votes
                        .unwrap_or_default()
                        .into_iter()
                        // TODO add From trait impl for optional_struct
                        // .map(Into::into)
                        // TODO add Vec support for optional_struct
                        .map(|named_vote| OptionalDbNamedVote {
                            id: Some(named_vote.id),
                            infavor: named_vote.infavor,
                            was_absent: named_vote.was_absent,
                            lev: Some(named_vote.lev),
                            similiarity_score: Some(named_vote.similiarity_score),
                            searched_with: named_vote.searched_with,
                            matched_with: Some(named_vote.matched_with),
                            delegate_id: Some(named_vote.delegate_id),
                            manually_matched: named_vote.manually_matched,
                        })
                        .collect(),
                })
            }
            _ => None,
        };

        let topics = legis_init
            .topics
            .into_iter()
            .map(|topic| Topic { topic })
            .collect::<Vec<_>>();
        let mut eurovoc_topics = legis_init
            .eurovoc_topics
            .into_iter()
            .map(|topic| Topic { topic })
            .collect::<Vec<_>>();
        let other_keyword_topics = legis_init
            .other_keyword_topics
            .into_iter()
            .map(|topic| Topic { topic })
            .collect::<Vec<_>>();
        let documents = legis_init
            .documents
            .into_iter()
            .map(|document| Document {
                title: Some(document.title),
                document_type: Some(document.ftype.to_string()),
                document_url: Some(document.link),
            })
            .collect::<Vec<_>>();

        if let Some(somai_client) = somai_client {
            if eurovoc_topics.is_empty() {
                // check if there are already stored in db to avoid expensive model call?
                fn extract_text_from_docs() -> String {
                    "text from docs".into()
                }
                eurovoc_topics = somai_client
                    .make_topics_predictions(extract_text_from_docs())
                    .await
                    .unwrap()
                    .topics
                    .into_iter()
                    .map(|topic| Topic { topic })
                    .collect();
                panic!()
            }
        }

        let mut speeches = legis_init
            .speeches
            .into_iter()
            .map(|speech| {
                let opinion = match &speech.opinion {
                    Opinion::Infavor => Some("Pro".to_string()),
                    Opinion::Against => Some("Contra".to_string()),
                    Opinion::Other(text) => Some(text.clone()),
                    Opinion::None => None,
                };

                DbSpeechWithLink {
                    delegate_id: speech.delegate_id as i32,
                    infavor: match speech.opinion {
                        Opinion::Infavor => Some(true),
                        Opinion::Against => Some(false),
                        _ => None,
                    },
                    duration_in_seconds: None,
                    opinion,
                    document_url: speech
                        .spoken_text_html_urls
                        .first()
                        .map(|html_url| html_url.document_url.clone()),
                    about: None,
                    legis_init_id: None,
                }
            })
            .collect::<Vec<_>>();

        // IMPORTANT: others must not rely on this id (anymore)! (e.g. Meilisearch ID/primary key)
        let maybe_legis_init_id = db_enhancer.maybe_get_legis_init_id(&db_legis_init).await;

        let (absences, referenced_by_others_ids) =
            if let Some(maybe_legis_init_id) = maybe_legis_init_id {
                if let Ok(plenar_speeches) = db_enhancer
                    .get_plenar_speeches(&db_legis_init, maybe_legis_init_id)
                    .await
                {
                    speeches = plenar_speeches;
                }

                let absences = db_enhancer
                    .get_absences_from_legis_init(maybe_legis_init_id)
                    .await
                    .ok();

                let referenced_by_other_ids = db_enhancer
                    .get_referenced_by_other_ids(&db_legis_init)
                    .await
                    .ok();

                (absences, referenced_by_other_ids)
            } else {
                (None, None)
            };

        VoteResult {
            id: maybe_legis_init_id,
            legislative_initiative: db_legis_init,
            votes,
            speeches,
            named_votes,
            topics,
            eurovoc_topics,
            other_keyword_topics,
            documents,
            absences,
            issued_by_dels: legis_init
                .related_delegates
                .into_iter()
                .map(|related_delegate| DbRelatedDelegate {
                    text: related_delegate.text,
                    delegate_id: related_delegate.delegate_id as i32,
                })
                .collect(),
            referenced_by_others_ids,
            references: legis_init.references.map(|references| {
                references
                    .into_iter()
                    .map(|reference| DbReference {
                        gp: reference.gp,
                        ityp: reference.ityp,
                        inr: reference.inr as i32,
                    })
                    .collect()
            }),
            meilisearch_helper: MeilisearchHelper::default(),
        }
    }
}
