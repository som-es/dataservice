mod speech_ai_summary;
use crate::models::*;
use serde::{Deserialize, Serialize};
use somes_macro::{CompositeType, MeilisearchFilter};
use somes_meilisearch_filter::{FilterArgument, FilterOp};
pub use speech_ai_summary::*;
use sqlx::prelude::{FromRow, Type};

use crate::DbSpeechWithLink;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(
    Debug,
    Deserialize,
    Serialize,
    PartialEq,
    Clone,
    FromRow,
    Type,
    Hash,
    CompositeType,
    MeilisearchFilter,
)]
#[sqlx(type_name = "full_speech")]
pub struct FullSpeech {
    pub id: i32,
    pub debate_id: i32,
    pub delegate_id: i32,
    pub speech: DbSpeechWithLink,
    #[filter(skip)]
    pub ai_summary: Option<DbSpeechAiSummary>,
    #[filter(skip)]
    pub relations: Vec<DbSpeechRelations>,
}
