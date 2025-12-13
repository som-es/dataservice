use chrono::{NaiveDate, NaiveDateTime};
use optional_struct::*;
use serde::{Deserialize, Serialize};
use somes_macro::CompositeType;
use sqlx::{FromRow, prelude::Type};
#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
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
    pub is_urgent: bool,
}

#[optional_struct]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(
    PartialEq, Debug, Clone, Serialize, Deserialize, FromRow, Default, CompositeType, Type,
)]
#[sqlx(type_name = "db_legislative_initiative_query")]
pub struct DbLegislativeInitiativeQuery {
    pub id: i32,
    pub ityp: String,
    pub doktyp: String,
    pub gp: String,
    pub inr: i32,
    pub emphasis: Option<String>,
    pub ai_emphasis: Option<String>,
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
    pub is_emphasis_ai_generated: bool,
    pub is_law: bool,
    pub law_accepted: Option<bool>,
    pub law_come_into_effect_date: Option<NaiveDate>,
    pub law_expires_on_date: Option<NaiveDate>,
    pub by_publication: Option<bool>,
    pub has_reference: Option<bool>,
    pub is_voteable_on: bool,
    pub is_urgent: bool,
    pub voting: Option<String>,
}

#[optional_struct]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize, CompositeType, Type)]
#[sqlx(type_name = "db_vote")]
pub struct DbVote {
    pub party: String,
    pub code: Option<String>,
    pub fraction: i32,
    pub infavor: bool,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct DbVoteIns {
    pub party: String,
    pub code: String,
    pub fraction: i32,
    pub infavor: bool,
    pub legislative_initiatives_id: i32,
}

#[optional_struct]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize, CompositeType, Type)]
#[sqlx(type_name = "db_speech_with_link")]
pub struct DbSpeechWithLink {
    pub delegate_id: i32,
    pub legis_init_id: Option<i32>,
    pub infavor: Option<bool>,
    pub duration_in_seconds: Option<i32>,
    pub opinion: Option<String>,
    pub document_url: Option<String>,
    pub about: Option<String>,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize, CompositeType, Type)]
#[sqlx(type_name = "db_named_vote")]
pub struct DbNamedVote {
    pub id: i32,
    pub infavor: Option<bool>,
    pub was_absent: Option<bool>,
    pub lev: i64,
    pub similiarity_score: i64,
    pub searched_with: Option<String>,
    pub matched_with: String,
    pub delegate_id: i32,
    // pub named_vote_info_id: i32,
    pub manually_matched: Option<bool>,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize, CompositeType, Type)]
#[sqlx(type_name = "optional_db_named_vote")]
pub struct OptionalDbNamedVote {
    pub id: Option<i32>,
    pub infavor: Option<bool>,
    pub was_absent: Option<bool>,
    pub lev: Option<i64>,
    pub similiarity_score: Option<i64>,
    pub searched_with: Option<String>,
    pub matched_with: Option<String>,
    pub delegate_id: Option<i32>,
    // pub named_vote_info_id: i32,
    pub manually_matched: Option<bool>,
}

// #[optional_struct]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize, CompositeType, Type)]
#[sqlx(type_name = "db_named_vote_info")]
pub struct DbNamedVoteInfo {
    pub pro_count: i32,
    pub contra_count: i32,
    pub given_vote_sum: i32,
    pub invalid_count: i32,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize, CompositeType, Type, Default)]
#[sqlx(type_name = "optional_db_named_vote_info")]
pub struct OptionalDbNamedVoteInfo {
    pub pro_count: Option<i32>,
    pub contra_count: Option<i32>,
    pub given_vote_sum: Option<i32>,
    pub invalid_count: Option<i32>,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize, CompositeType, Type)]
#[sqlx(type_name = "db_named_votes")]
pub struct DbNamedVotes {
    pub named_vote_info: DbNamedVoteInfo,
    pub named_votes: Option<Vec<DbNamedVote>>,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize, Default, CompositeType, Type)]
pub struct OptionalDbNamedVotes {
    pub named_vote_info: Option<OptionalDbNamedVoteInfo>,
    pub named_votes: Option<Vec<OptionalDbNamedVote>>,
}

#[optional_struct]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize, CompositeType, Type)]
#[sqlx(type_name = "db_named_vote_info_query")]
pub struct DbNamedVoteInfoQuery {
    pub id: i32,
    pub legis_init_id: i32,
    pub pro_count: i32,
    pub contra_count: i32,
    pub given_vote_sum: i32,
    pub invalid_count: i32,
}

#[optional_struct]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize, CompositeType, Type)]
#[sqlx(type_name = "db_party_named_vote_count")]
pub struct DbPartyNamedVoteCount {
    pub party: Option<String>,
    pub infavor: Option<bool>,
    pub count: Option<i64>,
}

#[optional_struct]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, CompositeType, Type)]
#[sqlx(type_name = "db_reference")]
pub struct DbReference {
    pub gp: String,
    pub ityp: String,
    pub inr: i32,
}

#[optional_struct]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, CompositeType, Type)]
#[sqlx(type_name = "db_related_delegate")]
pub struct DbRelatedDelegate {
    pub text: Option<String>,
    pub delegate_id: i32,
}
