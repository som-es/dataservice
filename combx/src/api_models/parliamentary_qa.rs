use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use somes_macro::CompositeType;
use sqlx::prelude::{FromRow, Type};
#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(
    JsonSchema, Hash, Debug, Deserialize, PartialEq, Serialize, Clone, FromRow, CompositeType,
)]
pub struct QuestionEntry {
    short_title: String,
    once_sentence_short_info_summary: String,
    very_short_info_summary: String,
    short_info_summary: String,
    info_summary: String,
    info_sources_urls: Vec<String>,
    detailed_info_summary: String,
    question_summary: String,
    detailed_question_summary: String,
    questions: Vec<Question>,
    topics: Vec<String>,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(JsonSchema, Hash, Debug, Deserialize, PartialEq, Serialize, Clone)]
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
    question: String,
    nth_level: Vec<Count>,
    // sub_questions: Vec<Question>,
}
