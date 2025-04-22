use chrono::{NaiveDate, NaiveDateTime};
use serde_derive::{Deserialize, Serialize};

// use super::schema::{
//     call_to_order, contacts, debates, delegates, eurovoc_topics_legis_init,
//     eurovoc_topics_ministrial_proposals, eurovoc_topics_proposals, legislative_documents,
//     legislative_initiatives, mandates, ministrial_proposals, ministrial_proposals_documents,
//     other_keyword_topics_legis_init, other_keyword_topics_ministrial_proposals,
//     other_keyword_topics_proposals, parties, plenar_infos, plenar_speech_links, plenar_speeches,
//     proposal_delegates, proposal_documents, proposals, speeches, topics_legis_init,
//     topics_ministrial_proposals, topics_proposals, votes,
// };

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
// #[diesel(table_name = proposals)]
pub struct DbProposal {
    pub id: String,
    pub ityp: String,
    pub gp: String,
    pub title: String,
    pub description: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
}
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
// #[diesel(table_name = proposals)]
pub struct DbProposalQuery {
    pub id: String,
    pub ityp: String,
    pub gp: String,
    pub title: String,
    pub description: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
}
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
// #[diesel(table_name = delegates)]
// #[diesel(primary_key(id))]
pub struct DbDelegate {
    pub id: i32,
    pub name: String,
    pub party: Option<String>,
    pub image_url: Option<String>,
    pub constituency: Option<String>,
    pub council: Option<String>,
    pub seat_row: Option<i32>,
    pub seat_col: Option<i32>,
    pub gender: Option<String>,
    pub is_active: Option<bool>,
    pub birthdate: Option<NaiveDate>,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
// #[diesel(table_name = proposal_delegates)]
// #[diesel(primary_key(proposal_id, delegate_id))]
// #[diesel(belongs_to(DbDelegate, foreign_key = delegate_id))]
// #[diesel(belongs_to(DbProposal, foreign_key = proposal_id))]
pub struct ProposalDelegate {
    pub proposal_id: String,
    pub delegate_id: i32,
    pub is_receiver: bool,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
// #[diesel(table_name = legislative_initiatives)]
pub struct DbLegislativeInitiative {
    pub ityp: String,
    pub doktyp: String,
    pub gp: String,
    pub inr: i32,
    pub emphasis: Option<String>,
    pub title: String,
    pub description: String,
    pub accepted: Option<String>,
    pub created_at: NaiveDate,
    pub appeared_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
    pub requires_simple_majority: bool,
    pub pre_declined_type: Option<String>,
    pub voted_by_name: bool,
    pub plenary_session_id: Option<i32>,
    pub is_law: bool,
    pub law_accepted: Option<bool>,
    pub law_come_into_effect_date: Option<NaiveDate>,
    pub law_expires_on_date: Option<NaiveDate>,
    pub by_publication: Option<bool>,
    pub has_reference: Option<bool>,
    pub is_voteable_on: Option<bool>,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
// #[diesel(table_name = ministrial_proposals)]
pub struct DbMinistrialProposal {
    pub ityp: String,
    pub gp: String,
    pub inr: i32,
    pub emphasis: Option<String>,
    pub title: String,
    pub description: String,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
    pub due_to: NaiveDate,
    pub ressort: String,
    pub ressort_shortform: String,
    pub has_vote_result: bool,
    pub legis_init_gp: Option<String>,
    pub legis_init_inr: Option<i32>,
    pub legis_init_ityp: Option<String>,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize, FromRow)]
// #[diesel(table_name = ministrial_proposals)]
pub struct DbMinistrialProposalQuery {
    pub delegate_id: i32,
    pub id: i32,
    pub ityp: String,
    pub gp: String,
    pub inr: i32,
    pub emphasis: Option<String>,
    pub title: String,
    pub description: String,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
    pub due_to: NaiveDate,
    pub ressort: Option<String>,
    pub ressort_shortform: Option<String>,
    pub legis_init_gp: Option<String>,
    pub legis_init_inr: Option<i32>,
    pub legis_init_ityp: Option<String>,
    pub has_vote_result: Option<bool>,
}
use sqlx::FromRow;

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize, FromRow)]
// #[diesel(table_name = legislative_initiatives)]
pub struct DbLegislativeInitiativeQuery {
    pub id: i32,
    pub ityp: String,
    pub doktyp: Option<String>,
    pub gp: String,
    pub inr: i32,
    pub emphasis: Option<String>,
    pub title: String,
    pub description: String,
    pub accepted: Option<String>,
    pub created_at: NaiveDate,
    pub appeared_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
    pub requires_simple_majority: Option<bool>,
    pub pre_declined_type: Option<String>,
    pub voted_by_name: bool,
    pub plenary_session_id: Option<i32>,
    pub is_emphasis_ai_generated: Option<bool>,
    pub is_law: Option<bool>,
    pub law_accepted: Option<bool>,
    pub law_come_into_effect_date: Option<NaiveDate>,
    pub law_expires_on_date: Option<NaiveDate>,
    pub by_publication: Option<bool>,
    pub has_reference: Option<bool>,
    pub is_voteable_on: Option<bool>,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
// #[diesel(table_name = votes)]
// #[diesel(primary_key(party, legislative_initiatives_id))]
// #[diesel(belongs_to(DbLegislativeInitiative, foreign_key = legislative_initiatives_id))]
pub struct DbVote {
    pub party: String,
    pub fraction: i32,
    pub infavor: bool,
    pub legislative_initiatives_id: i32,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
// #[diesel(table_name = votes)]
// #[diesel(primary_key(party, legislative_initiatives_id))]
// #[diesel(belongs_to(DbLegislativeInitiative, foreign_key = legislative_initiatives_id))]
pub struct DbVoteIns {
    pub party: String,
    pub code: String,
    pub fraction: i32,
    pub infavor: bool,
    pub legislative_initiatives_id: i32,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
// #[diesel(table_name = speeches)]
// #[diesel(primary_key(delegate_id, legislative_initiatives_id))]
// #[diesel(belongs_to(DbLegislativeInitiative, foreign_key = legislative_initiatives_id))]
// #[diesel(belongs_to(DbDelegate, foreign_key = delegate_id))]
pub struct DbSpeech {
    pub delegate_id: i32,
    pub infavor: Option<bool>,
    pub opinion: Option<String>,
    pub legislative_initiatives_id: i32,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct DbSpeechWithLink {
    pub delegate_id: i32,
    pub infavor: Option<bool>,
    pub opinion: Option<String>,
    pub document_url: Option<String>,
    pub about: Option<String>,
    pub legislative_initiatives_id: Option<i32>,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
// #[diesel(table_name = topics_legis_init)]
// #[diesel(belongs_to(DbLegislativeInitiative, foreign_key = legislative_initiatives_id))]
pub struct DbTopicLegisInit {
    pub topic: String,
    pub legislative_initiatives_id: i32,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
// #[diesel(table_name = eurovoc_topics_legis_init)]
// #[diesel(belongs_to(DbLegislativeInitiative, foreign_key = legislative_initiatives_id))]
pub struct DbEurovocTopicLegisInit {
    pub topic: String,
    pub legislative_initiatives_id: i32,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
// #[diesel(table_name = other_keyword_topics_legis_init)]
// #[diesel(belongs_to(DbLegislativeInitiative, foreign_key = legislative_initiatives_id))]
pub struct DbOtherKeywordTopicLegisInit {
    pub topic: String,
    pub legislative_initiatives_id: i32,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
// #[diesel(table_name = topics_ministrial_proposals)]
// #[diesel(belongs_to(DbMinistrialProposal, foreign_key = ministrial_proposal_id))]
pub struct DbTopicMinistrialProposals {
    pub topic: String,
    pub ministrial_proposal_id: i32,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
// #[diesel(table_name = eurovoc_topics_ministrial_proposals)]
// #[diesel(belongs_to(DbMinistrialProposal, foreign_key = ministrial_proposal_id))]
pub struct DbEurovocTopicMinsistrialProposals {
    pub topic: String,
    pub ministrial_proposal_id: i32,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
// #[diesel(table_name = other_keyword_topics_ministriy = ministrial_proposal_id))]
pub struct DbOtherKeywordTopicMinistrialProposals {
    pub topic: String,
    pub ministrial_proposal_id: i32,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
// #[diesel(table_name = topics_proposals)]
// #[diesel(belongs_to(DbProposal, foreign_key = proposal_id))]
pub struct DbTopicProposals {
    pub topic: String,
    pub proposals_id: String,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
// #[diesel(table_name = eurovoc_topics_proposals)]
// #[diesel(belongs_to(DbProposal, foreign_key = proposal_id))]
pub struct DbEurovocTopicProposals {
    pub topic: String,
    pub proposals_id: String,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
// #[diesel(table_name = other_keyword_topics_proposals)]
// #[diesel(belongs_to(DbProposal, foreign_key = proposal_id))]
pub struct DbOtherKeywordTopicProposals {
    pub topic: String,
    pub proposals_id: String,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
// #[diesel(table_name = legislative_documents)]
// #[diesel(belongs_to(DbLegislativeInitiative, foreign_key = legislative_initiatives_id))]
pub struct DbLegisDocument {
    pub title: String,
    pub document_type: String,
    pub document_url: String,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
// #[diesel(table_name = legislative_documents)]
// #[diesel(belongs_to(DbLegislativeInitiative, foreign_key = legislative_initiatives_id))]
pub struct DbLegisDocumentOptional {
    pub title: Option<String>,
    pub document_type: Option<String>,
    pub document_url: Option<String>,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
// #[diesel(table_name = ministrial_proposals_documents)]
// #[diesel(belongs_to(DbMinistrialProposal, foreign_key = ministrial_proposal_id))]
pub struct DbMinistrialProposalDocument {
    pub title: String,
    pub document_type: String,
    pub document_url: String,
    pub ministrial_proposal_id: i32,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
// #[diesel(table_name = proposal_documents)]
// #[diesel(belongs_to(DbProposal, foreign_key = proposal_id))]
pub struct DbProposalDocument {
    pub title: String,
    pub document_type: String,
    pub document_url: String,
    pub proposal_id: String,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
// #[diesel(table_name = contacts)]
pub struct DbContactQuery {
    pub id: i32,
    pub mail: Option<String>,
    pub phone_number: Option<String>,
    pub twitter_url: Option<String>,
    pub facebook_url: Option<String>,
    pub website_url: Option<String>,
    pub tiktok_url: Option<String>,
    pub instagram_url: Option<String>,
    pub youtube_url: Option<String>,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
// #[diesel(table_name = contacts)]
// #[diesel(primary_key(id))]
pub struct DbContact {
    pub id: i32,
    pub mail: Option<String>,
    pub phone_number: Option<String>,
    pub twitter_url: Option<String>,
    pub facebook_url: Option<String>,
    pub website_url: Option<String>,
    pub tiktok_url: Option<String>,
    pub instagram_url: Option<String>,
    pub youtube_url: Option<String>,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(
    //
    PartialEq,
    Debug,
    Clone,
    Serialize,
    Deserialize,
)]
// #[diesel(primary_key(id))]
pub struct DbPlenar {
    pub inr: i32,
    pub title: String,
    pub description: String,
    pub add_date: NaiveDateTime,
    pub legislative_period: String,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
// #[diesel(table_name = call_to_order)]
// #[diesel(primary_key(id))]
pub struct DbCallToOrder {
    pub id: String,
    pub plenar_id: i32,
    pub inr: i32,
    pub receiver_id: i32,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
// #[diesel(table_name = debates)]
// #[diesel(primary_key(id))]
pub struct DbDebate {
    pub plenar_id: i32,
    pub about: String,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
// #[diesel(table_name = plenar_speeches)]
// #[diesel(primary_key(id, delegate_id, debate_id))]
pub struct DbPlenarSpeech {
    pub delegate_id: i32,
    pub debate_id: i32,
    pub opinion: Option<String>,
    pub spoken_text: Option<String>,
    pub duration_in_seconds: i32,
    pub ty: String,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
// #[diesel(table_name = plenar_speech_links)]
// #[diesel(primary_key(id, delegate_id, debate_id))]
pub struct DbPlenarSpeechLink {
    pub plenar_speech_id: i32,
    pub document_url: String,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
// #[diesel(table_name = mandates)]
// #[diesel(primary_key(id))]
pub struct DbMandate {
    pub delegate_id: i32,
    pub is_nr: bool,
    pub is_gov_official: bool,
    pub function: Option<String>,
    pub is_ministry: bool,
    pub is_chancellor: bool,
    pub ministry: Option<String>,
    pub name: String,
    pub party: Option<String>,
    pub start_date: NaiveDate,
    pub end_date: Option<NaiveDate>,
}

impl DbMandate {
    #[inline]
    pub fn is_active(&self) -> bool {
        self.end_date.is_none()
    }
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
// #[diesel(table_name = parties)]
// #[diesel(primary_key(id))]
pub struct DbParty {
    pub name: String,
    pub fraction: i32,
    pub color: String,
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct DbNamedVote {
    pub id: i32,
    pub infavor: Option<bool>,
    pub was_absent: Option<bool>,
    pub lev: i64,
    pub similiarity_score: i64,
    pub searched_with: Option<String>,
    pub matched_with: String,
    pub delegate_id: i32,
    pub named_vote_info_id: i32,
    pub manually_matched: Option<bool>,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct DbNamedVotes {
    pub named_vote_info: DbNamedVoteInfo,
    pub named_votes: Vec<DbNamedVote>,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct DbNamedVoteInfo {
    pub id: i32,
    pub legis_init_id: i32,
    pub pro_count: i32,
    pub contra_count: i32,
    pub given_vote_sum: i32,
    pub invalid_count: i32,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct DbPartyNamedVoteCount {
    pub party: Option<String>,
    pub infavor: Option<bool>,
    pub count: Option<i64>,
}
