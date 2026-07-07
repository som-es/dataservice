use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use somes_macro::{CompositeType, MeilisearchFilter};
use somes_meilisearch_filter::{
    FilterArgument, FilterOp, FilterOptions, build_prefix, to_meilisearch_filters,
};

use sqlx::types::Json;
use sqlx::{FromRow, Type};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::{CriticalAnalysis, Glossary};

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(
    JsonSchema, Hash, Debug, Deserialize, PartialEq, Serialize, Clone, FromRow, CompositeType,
)]
pub struct SpeechAiSummarizeOutput {
    pub summary: Option<SpeechAiSummary>,
    pub abort_reason: SpeechAbortReason,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(
    JsonSchema,
    Debug,
    PartialEq,
    Deserialize,
    Serialize,
    Clone,
    FromRow,
    CompositeType,
    MeilisearchFilter,
    Hash,
)]
pub struct SpeechAiSummary {
    #[filter(make_optional)]
    pub short_title: String,

    #[filter(make_optional)]
    pub one_sentence_short_summary: String,

    #[filter(make_optional)]
    pub very_short_summary: String,

    #[filter(make_optional)]
    pub short_summary: String,

    #[filter(make_optional)]
    pub summary: String,

    #[filter(make_optional)]
    pub detailed_summary: String,

    #[filter(make_optional)]
    pub very_detailed_summary: String,

    #[filter(skip)]
    pub glossary: Glossary,

    #[filter(skip)]
    pub critical_analysis: CriticalAnalysis,

    #[filter(skip)]
    pub key_points: Vec<SpeechKeypoint>,
}
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(
    JsonSchema, Hash, Debug, PartialEq, Deserialize, Serialize, Clone, FromRow, CompositeType,
)]
pub struct SpeechKeypoint {
    pub summarized_point: String,
    pub unmodified_reference_point: String,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(JsonSchema, Hash, Debug, Deserialize, PartialEq, Serialize, Clone, Type)]
pub enum SpeechAbortReason {
    NoSpeechProvidedInContext,
    None,
}

#[derive(Hash, Debug, Deserialize, PartialEq, Serialize, Type, Clone, FromRow, CompositeType)]
#[sqlx(type_name = "db_speech_ai_summary")]
pub struct DbSpeechAiSummary {
    pub id: i32,
    pub speech_id: i32,
    pub short_title: String,
    pub one_sentence_short_summary: String,
    pub very_short_summary: String,
    pub short_summary: String,
    pub summary: String,
    pub detailed_summary: String,
    pub very_detailed_summary: String,
    pub full_speech_summary: Json<SpeechAiSummary>,
    pub model_used: String,
    pub version: String,
    pub generated_at: DateTime<Utc>,
}

#[derive(Hash, Debug, Deserialize, PartialEq, Serialize, Type, Clone, FromRow, CompositeType)]
#[sqlx(type_name = "db_speech_relations")]
pub struct DbSpeechRelations {
    pub id: i32,
    pub speech_ai_summary_id: i32,
    pub legis_init_id: i32,
    pub full_speech_relations: Json<SpeechRelations>,
    pub model_used: String,
    pub version: String,
    pub generated_at: DateTime<Utc>,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(
    JsonSchema, Hash, Debug, Deserialize, PartialEq, Serialize, Clone, FromRow, CompositeType,
)]
pub struct SpeechRelationsToProposalOutput {
    pub speech_relations: Option<SpeechRelations>,
    pub abort_reason: SpeechAbortReason,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(
    JsonSchema,
    Debug,
    PartialEq,
    Deserialize,
    Serialize,
    Clone,
    FromRow,
    CompositeType,
    MeilisearchFilter,
    Hash,
)]
pub struct SpeechRelations {
    propsal_keypoint_relations: Option<Vec<SpeechRelation>>,
    speech_related_to_proposal_summary: bool,
    speech_related_to_detailed_proposal_summary: bool,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(
    JsonSchema,
    Debug,
    PartialEq,
    Deserialize,
    Serialize,
    Clone,
    FromRow,
    CompositeType,
    MeilisearchFilter,
    Hash,
)]
pub struct SpeechRelation {
    speech_key_point: i32,
    #[schemars(length(min = 1, max = 32))]
    referenced_proposal_key_point_ids: Vec<u32>,
}
