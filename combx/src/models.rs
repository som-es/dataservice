use std::hash::Hash;

use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, Utc};
use optional_struct::optional_struct;
use serde_derive::{Deserialize, Serialize};
use somes_meilisearch_filter::{FilterArgument, FilterOp};

use somes_macro::{CompositeType, MeilisearchFilter};
#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

pub use crate::api_models::*;

#[derive(PartialEq, Eq, Hash, Debug, Clone, Serialize, Deserialize)]
pub struct DelegateParty {
    pub party: Option<String>,
    pub id: i32,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Type,
    CompositeType,
    MeilisearchFilter,
    Hash,
)]
pub struct DbRisAmendmentRef {
    pub bgbl: String,
    pub doc_number: String,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LegislativePeriod {
    pub gp: String,
    pub start_date: NaiveDate,
}

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
#[derive(PartialEq, Eq, Hash, Debug, Clone, Serialize, Deserialize)]
// #[diesel(table_name = delegates)]
// #[diesel(primary_key(id))]
pub struct DbDelegate {
    pub id: i32,
    pub name: String,
    pub prev_name: Option<String>,
    pub prev_name_till: Option<NaiveDate>,
    pub party: Option<String>,
    pub image_url: Option<String>,
    pub image_copyright: Option<String>,
    pub constituency: Option<String>,
    pub council: Option<String>,
    pub seat_row: Option<i32>,
    pub seat_col: Option<i32>,
    pub gender: Option<String>,
    pub is_active: Option<bool>,
    pub birthdate: Option<NaiveDate>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub hash: Option<i64>,
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
#[derive(
    PartialEq,
    Debug,
    Clone,
    Serialize,
    Deserialize,
    FromRow,
    Type,
    CompositeType,
    MeilisearchFilter,
    Default,
    Hash,
)]
// #[diesel(table_name = ministrial_proposals)]
pub struct DbMinistrialProposalQueryMeta {
    #[filter(make_optional)]
    pub id: i32,
    #[filter(make_optional)]
    pub ityp: String,
    #[filter(make_optional, make_vec)]
    pub gp: String,
    #[filter(make_optional)]
    pub inr: i32,
    pub emphasis: Option<String>,
    #[filter(make_optional)]
    pub title: String,
    #[filter(make_optional)]
    pub description: String,
    #[filter(make_optional)]
    pub raw_data_created_at: DateTime<Utc>,
    pub raw_data_updated_at: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    #[filter(make_optional)]
    pub due_to: NaiveDate,
    #[filter(make_optional, make_vec)]
    pub ressort: String,
    #[filter(make_optional, make_vec)]
    pub ressort_shortform: String,
    pub legis_init_gp: Option<String>,
    pub legis_init_inr: Option<i32>,
    pub legis_init_ityp: Option<String>,
    #[filter(make_optional)]
    pub has_vote_result: bool,
}

use sqlx::{FromRow, prelude::Type};

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
    pub id: i32,
    pub inr: i32,
    pub title: String,
    pub council: String,
    pub description: String,
    pub raw_data_created_at: DateTime<Utc>,
    pub raw_data_updated_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub legislative_period: String,
    pub absences_doc_url: Option<String>,
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

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct OrientationQuestion {
    pub id: i32,
    pub question: String,
    pub is_left: Option<bool>,
    pub is_liberal: Option<bool>,
    pub is_part_of: Option<Vec<String>>,
}

#[optional_struct]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(
    Debug,
    Deserialize,
    Serialize,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    CompositeType,
    Type,
    MeilisearchFilter,
    Hash,
)]
pub struct Topic {
    pub topic: String,
}

#[optional_struct]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(
    Debug,
    Deserialize,
    Serialize,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    CompositeType,
    Type,
    MeilisearchFilter,
    Hash,
    Default,
)]
pub struct TopicWithId {
    pub topic: String,
    pub id: String,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(
    Debug,
    Deserialize,
    Serialize,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    CompositeType,
    Type,
    MeilisearchFilter,
    Hash,
)]
#[sqlx(type_name = "interjection")]
pub struct Interjection {
    pub interjection_text: Option<String>,
    pub interjector_delegate_id: i32,
    pub date: DateTime<Utc>,
    pub speaker_delegate_id: i32,
    pub plenar_speech_id: i32,
    pub rel_start_idx: i32,
    pub rel_end_idx: i32,
    pub delegate_match: DelegateMatch,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(
    Debug,
    Deserialize,
    Serialize,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    CompositeType,
    Type,
    MeilisearchFilter,
    Hash,
)]
#[sqlx(type_name = "db_interjection")]
pub struct DbInterjection {
    pub interjection_text: Option<String>,
    pub interjector_delegate_id: i32,
    pub plenar_speech_id: i32,
    pub rel_start_idx: i32,
    pub rel_end_idx: i32,
    pub delegate_matching_id: i32,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(
    Debug,
    Deserialize,
    Serialize,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    CompositeType,
    Type,
    MeilisearchFilter,
    Hash,
)]
pub struct DelegateMatch {
    pub similiarity_score: i64,
    pub searched_with: String,
    pub matched_with: String,
    pub delegate_id: i32,
    pub manually_matched: bool,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(
    Debug,
    Deserialize,
    Serialize,
    Clone,
    PartialEq,
    PartialOrd,
    CompositeType,
    Type,
    MeilisearchFilter,
)]
#[sqlx(type_name = "article_link")]
pub struct ArticleLink {
    pub provider: String,
    pub url: String,
    pub score: f32,
    pub lastmod: DateTime<FixedOffset>,
}

impl Hash for ArticleLink {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.provider.hash(state);
        self.url.hash(state);
        self.lastmod.hash(state);
    }
}
