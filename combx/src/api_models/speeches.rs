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

// #[cfg_attr(feature = "utoipa", derive(ToSchema))]
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
pub struct FullSpeech {
    id: i32,
    speech: DbSpeechWithLink,
    #[filter(skip)]
    ai_summary: DbSpeechAiSummary,
    #[filter(skip)]
    relations: DbSpeechRelations,
}
