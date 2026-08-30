mod enhancer;
mod filtering;
mod hidden_vote_translation;
mod models;
pub use models::*;
use somes_macro::{CompositeType, MeilisearchFilter};
use somes_meilisearch_filter::{FilterArgument, FilterOp};
mod named_votes;

pub use enhancer::*;
pub use filtering::*;
use optional_struct::optional_struct;
use serde::{Deserialize, Serialize};
use somai_client::SomAiClient;
use somes_common_lib::Document;
use somes_scraper::{LegislativeInitiative, Opinion};
use sqlx::{FromRow, PgPool, prelude::Type};
use std::collections::HashMap;
use std::sync::Arc;

use crate::with_data::DelegateParty;
use crate::{CombinedData, Parliament, models::*};
use crate::{
    FromScrapedType, Topic, api_models::vote_results::hidden_vote_translation::translate_votes,
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
    MeilisearchFilter,
    Hash,
)]
#[sqlx(type_name = "meilisearch_helper")]
pub struct MeilisearchHelper {
    #[serde(default)]
    #[filter(make_optional)]
    pub votes: Vec<String>,
    #[serde(default)]
    #[filter(make_optional)]
    pub issuer_parties: Vec<String>,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(
    Debug,
    Deserialize,
    Default,
    PartialEq,
    Serialize,
    Clone,
    FromRow,
    Type,
    CompositeType,
    MeilisearchFilter,
    Hash,
)]
pub struct VoteResult {
    #[filter(make_vec)]
    pub id: Option<i32>,

    pub legislative_initiative: DbLegislativeInitiativeQuery,
    pub votes: Vec<DbVote>,
    pub speeches: Vec<FullSpeech>,

    pub named_votes: Option<DbNamedVotes>,
    pub topics: Vec<Topic>,
    pub eurovoc_topics: Vec<Topic>,
    pub other_keyword_topics: Vec<Topic>,
    #[filter(skip)]
    pub documents: Vec<Document>,
    pub absences: Option<Vec<i32>>,
    pub issued_by_dels: Vec<DbRelatedDelegate>,
    pub referenced_by_others_ids: Option<Vec<i32>>,
    pub references_to: Option<Vec<DbReference>>,
    pub ai_summary: Option<DbAiSummary>,
    #[serde(default)]
    pub meilisearch_helper: Option<MeilisearchHelper>,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(
    Debug,
    Deserialize,
    Serialize,
    PartialEq,
    Clone,
    FromRow,
    Default,
    Type,
    Hash,
    CompositeType,
    MeilisearchFilter,
)]
pub struct OptionalVoteResult {
    #[filter(make_vec)]
    pub id: Option<i32>,
    pub legislative_initiative: Option<DbLegislativeInitiativeQuery>,
    pub votes: Option<Vec<DbVote>>,
    pub speeches: Option<Vec<FullSpeech>>,
    pub named_votes: Option<DbNamedVotes>,
    pub topics: Option<Vec<Topic>>,
    pub eurovoc_topics: Option<Vec<Topic>>,
    pub other_keyword_topics: Option<Vec<Topic>>,
    #[filter(skip)]
    pub documents: Option<Vec<Document>>,
    pub absences: Option<Vec<i32>>,
    pub issued_by_dels: Option<Vec<DbRelatedDelegate>>,
    pub referenced_by_others_ids: Option<Vec<i32>>,
    pub references: Option<Vec<DbReference>>,
    pub ai_summary: Option<DbAiSummary>,
    #[serde(default)]
    pub meilisearch_helper: Option<MeilisearchHelper>,
}

impl CombinedData for OptionalVoteResult {
    const INDEX: crate::Index = crate::Index::VoteResults;
    const PRIMARY_KEY: &str = "id";
    #[inline]
    fn id(&self) -> u64 {
        self.id.unwrap() as u64
    }
}

impl CombinedData for VoteResult {
    const INDEX: crate::Index = crate::Index::VoteResults;
    const PRIMARY_KEY: &str = "id";
    #[inline]
    fn id(&self) -> u64 {
        self.id.unwrap() as u64
    }
}

impl<T: VoteResultEnhancer> FromScrapedType<T> for VoteResult {
    type ScrapedType = LegislativeInitiative;

    async fn from_scraped_type(db_enhancer: T, ty: Self::ScrapedType) -> Self {
        Self::from_legis_init(db_enhancer, ty, Parliament::At, None)
            .await
            .unwrap()
    }
}

impl VoteResult {
    // move to other crate
    pub async fn from_db(_pool: &PgPool, _legis_init_id: i32) -> Self {
        todo!()
    }
    pub async fn from_legis_init(
        db_enhancer: impl VoteResultEnhancer,
        legis_init: LegislativeInitiative,
        parliament: Parliament,
        _somai_client: Option<Arc<SomAiClient>>,
    ) -> crate::Result<Self> {
        let plenary_session_id = db_enhancer.retrieve_plenary_session_id(&legis_init).await;

        let LegisInitInfo {
            id,
            created_at: _,
            updated_at: _,
            hash: _,
        } = db_enhancer
            .maybe_get_legis_init_info(legis_init.inr as i32, &legis_init.ityp, &legis_init.gp)
            .await;

        let db_legis_init = DbLegislativeInitiativeQuery {
            id: 0,
            title: legis_init.title.clone(),
            inr: legis_init.inr as i32,
            emphasis: legis_init.emphasis.clone(),
            ai_emphasis: None,
            description: legis_init.description.clone(),
            ityp: legis_init.ityp.clone(),
            gp: legis_init.gp.clone(),
            accepted: legis_init.accepted.map(|a| a.to_string()),
            nr_plenary_activity_date: legis_init.nr_plenary_activity_date,
            vote_date: legis_init.vote_date,
            raw_data_created_at: legis_init.created_at,
            raw_data_updated_at: legis_init.updated_at,
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
            created_at: None,
            updated_at: None,
            hash: None,
            // law_come_into_effect_date: legis_init.come_into_effect_info.map(|x| x.law_come_into_effect_date.clone()),
        };

        let mut votes = legis_init
            .votes
            .iter()
            .map(|vote| {
                let (infavor_count, against_count) = if vote.infavor {
                    (vote.party.fraction as i32, 0)
                } else {
                    (0, vote.party.fraction as i32)
                };
                DbVote {
                    party: vote.party.name.clone(),
                    code: Some(vote.party.code.clone()),
                    infavor_count,
                    against_count,
                    absence_count: 0,
                    abstention_count: 0,
                }
            })
            .collect::<Vec<_>>();

        if legis_init.votes.is_empty()
            // && legis_init.accepted == Some(somes_scraper::AcceptedState::PreDeclined)
            && legis_init.hidden_votes_info.pro_contra_parties.is_some()
        {
            let (all_parties, code_to_parties) = db_enhancer
                .gen_code_to_parties_with_date(
                    &legis_init
                        .vote_date
                        .unwrap_or(legis_init.nr_plenary_activity_date),
                    &db_legis_init.gp,
                )
                .await;

            match &legis_init.hidden_votes_info.pro_contra_parties {
                Some(pro_contra_parties) => {
                    votes = translate_votes(
                        &all_parties,
                        &code_to_parties,
                        pro_contra_parties,
                        &legis_init.gp,
                    )
                    .await;
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
                        let date = legis_init
                            .vote_date
                            .unwrap_or(legis_init.nr_plenary_activity_date);
                        let function = match parliament {
                            Parliament::At => "NR",
                            Parliament::Eu => "MEP",
                        };
                        let delegate_parties = db_enhancer
                            .active_delegates_party_at_date(date, function)
                            .await?;
                        let named_votes = db_enhancer
                            .create_db_named_votes(pro_contra, date, &legis_init.gp, function)
                            .await?;

                        votes = build_db_votes(&named_votes, &delegate_parties);

                        Some(named_votes)
                    }
                    None => None,
                };
                Some(DbNamedVotes {
                    named_vote_info,
                    named_votes,
                })
            }
            _ => None,
        };

        let topics = legis_init
            .topics
            .into_iter()
            .map(|topic| Topic { topic })
            .collect::<Vec<_>>();
        let eurovoc_topics = legis_init
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
                document_url: Some(document.link), // store complete URL
            })
            .collect::<Vec<_>>();

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

                FullSpeech {
                    id: 0,
                    debate_id: 0,
                    delegate_id: speech.delegate_id as i32,
                    ai_summary: None,
                    relations: vec![],
                    speech: DbSpeechWithLink {
                        delegate_id: speech.delegate_id as i32,
                        infavor: match speech.opinion {
                            Opinion::Infavor => Some(true),
                            Opinion::Against => Some(false),
                            _ => None,
                        },
                        duration_in_seconds: None,
                        opinion,
                        document_urls: Some(
                            speech
                                .spoken_text_html_urls
                                .iter()
                                .map(|html_url| html_url.document_url.clone())
                                .collect(),
                        ),

                        about: None,
                        vote_result_ids: None,
                        start: None,
                    },
                }
            })
            .collect::<Vec<_>>();

        let (absences, referenced_by_others_ids) = if let Some(maybe_legis_init_id) = id {
            if let Ok(plenar_speeches) = db_enhancer
                .get_plenar_speeches(&db_legis_init, maybe_legis_init_id)
                .await
            {
                speeches = plenar_speeches
                    .into_iter()
                    .map(|speech| FullSpeech {
                        id: 0,
                        debate_id: 0,
                        delegate_id: speech.delegate_id,
                        speech,
                        ai_summary: None,
                        relations: vec![],
                    })
                    .collect();
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

        Ok(VoteResult {
            id,
            legislative_initiative: db_legis_init,
            ai_summary: None,
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
            references_to: legis_init.references.map(|references| {
                references
                    .into_iter()
                    .map(|reference| DbReference {
                        gp: reference.gp,
                        ityp: reference.ityp,
                        inr: reference.inr as i32,
                    })
                    .collect()
            }),
            meilisearch_helper: Some(MeilisearchHelper::default()),
        })
    }
}

pub fn build_db_votes(
    named_votes: &[DbNamedVote],
    delegate_parties: &[DelegateParty],
) -> Vec<DbVote> {
    let party_by_delegate: HashMap<i32, String> = delegate_parties
        .into_iter()
        .map(|dp| (dp.id, dp.party.clone().unwrap_or_else(|| "OK".to_string())))
        .collect();

    let mut grouped: HashMap<String, DbVote> = HashMap::new();

    for vote in named_votes {
        let party = party_by_delegate
            .get(&vote.delegate_id)
            .cloned()
            .unwrap_or_else(|| "OK".to_string());

        let entry = grouped.entry(party.clone()).or_insert_with(|| DbVote {
            party: party.clone(),
            code: None,
            infavor_count: 0,
            against_count: 0,
            abstention_count: 0,
            absence_count: 0,
        });

        if vote.was_abstention {
            entry.abstention_count += 1;
        } else if vote.was_absent.unwrap_or(false) {
            entry.absence_count += 1;
        } else {
            match vote.infavor {
                Some(true) => entry.infavor_count += 1,
                Some(false) => entry.against_count += 1,
                None => {}
            }
        }
    }

    grouped.into_values().collect()
}
#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use somai_client::SomAiClient;
    use somes_common_lib::Document;

    use crate::document_text::{combine_texts, extract_text_from_documents_html_first_stripped};

    #[tokio::test]
    async fn test_ai_summary_from_vote_result() {
        let url = "https://www.parlament.gv.at/dokument/XXVIII/A/468/fname_1715462.pdf";
        let documents = vec![Document {
            title: Some("Test Document".to_string()),
            document_url: Some(url.to_string()),
            document_type: Some("DigitalPDF".to_string()),
        }];

        let somai_client = Arc::new(
            SomAiClient::new_local("http://127.0.0.1:8000")
                .await
                .unwrap(),
        );

        let document_texts =
            combine_texts(&extract_text_from_documents_html_first_stripped(&documents, None).await);

        println!(
            "document_texts: {}",
            &document_texts[..300.min(document_texts.len())]
        );
        let out = somai_client
            .summarize_input(document_texts, None)
            .await
            .unwrap();
        println!("{:?}", out);
    }
}
