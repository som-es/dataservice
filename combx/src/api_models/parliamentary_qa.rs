use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use somes_common_lib::Document;
use somes_macro::CompositeType;
use sqlx::{
    prelude::{FromRow, Type},
    types::Json,
};
#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::{AbortReason, DbReference};

// #[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(Debug, Deserialize, Serialize, PartialEq, Clone, FromRow, Type, Hash, CompositeType)]
#[sqlx(type_name = "parliament_raw_data")]
pub struct ParliamentRawData {
    pub gp: String,
    pub ityp: String,
    pub inr: i32,
    pub issuer_ids: Vec<i64>,
    pub receiver_ids: Vec<i64>,
    pub documents: Vec<Document>,
    pub topics: Vec<String>,
    pub eurovoc_topics: Vec<String>,
    pub other_keyword_topics: Vec<String>,
    pub description: Option<String>,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub raw_data_created_at: DateTime<Utc>,
    pub raw_data_updated_at: DateTime<Utc>,
}
// #[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(Debug, Deserialize, Serialize, PartialEq, Clone, FromRow, Type, Hash, CompositeType)]
#[sqlx(type_name = "parliament_inquiry")]
pub struct ParliamentInquiry {
    pub data: ParliamentRawData,
    pub ai_question: Option<DbAiInquiry>,
    pub refs: Vec<DbReference>,
}
// #[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(Debug, Deserialize, Serialize, PartialEq, Clone, FromRow, Type, Hash, CompositeType)]
#[sqlx(type_name = "parliament_answer")]
pub struct ParliamentAnswer {
    pub data: ParliamentRawData,
    pub ai_answer: Option<DbAnswerEntry>,
    pub refs: Vec<DbReference>,
}
// #[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(Debug, Deserialize, Serialize, PartialEq, Clone, FromRow, Type, Hash, CompositeType)]
#[sqlx(type_name = "parliament_inquiry_response")]
pub struct ParliamentInquiryResponse {
    pub question: Option<ParliamentInquiry>,
    pub answer: Option<Vec<ParliamentAnswer>>,
}

#[derive(Hash, Debug, Deserialize, PartialEq, Serialize, Type, Clone, FromRow, CompositeType)]
#[sqlx(type_name = "db_answer_entry")]
pub struct DbAnswerEntry {
    pub id: i64,
    pub pqa_meta_id: i64,
    pub short_title: String,
    pub full_answer_summary: String,
    pub detailed_full_answer_summary: String,
    pub full_answer_entry: Json<AnswerEntry>,
    pub model_used: String,
    pub version: String,
    pub generated_at: DateTime<Utc>,
}
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(
    JsonSchema, Hash, Debug, Deserialize, PartialEq, Serialize, Clone, FromRow, CompositeType,
)]
pub struct AnswerEntryOutput {
    pub answer_entry: Option<AnswerEntry>,
    pub abort_reason: AbortReason,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(
    JsonSchema, Hash, Debug, Deserialize, PartialEq, Serialize, Clone, FromRow, CompositeType,
)]
pub struct AnswerEntry {
    pub short_title: String,
    pub answers: Vec<SweepAnswer>,
    pub full_answer_summary: String,
    pub detailed_full_answer_summary: String,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(
    JsonSchema, Hash, Debug, Deserialize, PartialEq, Serialize, Clone, FromRow, CompositeType,
)]
pub struct SweepAnswer {
    pub answering_questions_references: Vec<QuestionRef>,
    pub raw_answer: String,
    pub once_sentence_short_answer_summary: String,
    pub very_short_answer_summary: String,
    pub short_answer_summary: String,
    pub answer_summary: String,
    pub other_written_answer_refs: Vec<AnswerRef>,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(
    JsonSchema, Hash, Debug, Deserialize, PartialEq, Serialize, Clone, FromRow, CompositeType,
)]
pub struct AnswerRef {
    inr: u64,
    gp: String,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(
    JsonSchema, Hash, Debug, Deserialize, PartialEq, Serialize, Clone, FromRow, CompositeType,
)]
pub struct QuestionRef {
    pub question_assignment_confidence: Confidence,
    #[schemars(length(min = 1, max = 10))]
    pub affected_question_absolute_path: Vec<Count>,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(JsonSchema, Hash, Debug, Deserialize, PartialEq, Serialize, Clone)]
pub enum Confidence {
    VeryConfident,
    Confident,
    Likely,
    NotConfident,
}

#[derive(Hash, Debug, Deserialize, PartialEq, Serialize, Type, Clone, FromRow, CompositeType)]
#[sqlx(type_name = "db_ai_inquiry")]
pub struct DbAiInquiry {
    pub id: i64,
    pub pqa_meta_id: i64,
    pub short_title: String,
    pub once_sentence_short_info_summary: String,
    pub very_short_info_summary: String,
    pub short_info_summary: String,
    pub info_summary: String,
    pub detailed_info_summary: String,
    pub question_summary: String,
    pub detailed_question_summary: String,
    pub full_question_entry: Json<QuestionEntry>,
    pub model_used: String,
    pub version: String,
    pub generated_at: DateTime<Utc>,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(
    JsonSchema, Hash, Debug, Deserialize, PartialEq, Serialize, Clone, FromRow, CompositeType,
)]
pub struct QuestionEntryOutput {
    pub complete_inquiry_extraction: Option<QuestionEntry>,
    pub abort_reason: AbortReason,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(
    JsonSchema, Hash, Debug, Deserialize, PartialEq, Serialize, Clone, FromRow, CompositeType,
)]
pub struct QuestionEntry {
    pub short_title: String,
    pub once_sentence_short_info_summary: String,
    pub very_short_info_summary: String,
    pub short_info_summary: String,
    pub info_summary: String,
    pub info_sources_urls: Vec<String>,
    pub detailed_info_summary: String,
    pub question_summary: String,
    pub detailed_question_summary: String,
    pub questions: Vec<Question>,
    pub topics: Vec<String>,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(Eq, JsonSchema, Hash, Debug, Deserialize, PartialEq, Serialize, Clone)]
#[serde(tag = "kind", content = "value")]
pub enum Count {
    Number(u32),
    String(String),
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(
    JsonSchema, Hash, Debug, Deserialize, PartialEq, Serialize, Clone, FromRow, CompositeType,
)]
pub struct Question {
    pub raw_question: String,
    #[schemars(length(min = 1, max = 10))]
    pub nth_level: Vec<Count>,
    // sub_questions: Vec<Question>,
}
