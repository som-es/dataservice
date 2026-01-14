use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use optional_struct::optional_struct;
use serde_derive::{Deserialize, Serialize};
use somes_meilisearch_filter::{FilterArgument, FilterOp};

use somes_macro::{CompositeType, MeilisearchFilter};
#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

pub use crate::api_models::*;

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
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
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
    PartialEq, Debug, Clone, Serialize, Deserialize, FromRow, Type, CompositeType, MeilisearchFilter,
)]
// #[diesel(table_name = ministrial_proposals)]
pub struct DbMinistrialProposalQueryMeta {
    pub id: i32,
    pub ityp: String,
    pub gp: String,
    pub inr: i32,
    pub emphasis: Option<String>,
    pub title: String,
    pub description: String,
    pub raw_data_created_at: DateTime<Utc>,
    pub raw_data_updated_at: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub due_to: NaiveDate,
    pub ressort: String,
    pub ressort_shortform: String,
    pub legis_init_gp: Option<String>,
    pub legis_init_inr: Option<i32>,
    pub legis_init_ityp: Option<String>,
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
)]
pub struct Topic {
    pub topic: String,
}
