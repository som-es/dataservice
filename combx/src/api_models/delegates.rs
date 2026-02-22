use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use somes_macro::{CompositeType, MeilisearchFilter};
use somes_meilisearch_filter::{FilterArgument, FilterOp};
use sqlx::Type;
#[cfg(feature = "utoipa")]
use utoipa::{IntoParams, ToSchema};

use crate::CombinedData;

#[cfg_attr(feature = "utoipa", derive(ToSchema, IntoParams))]
#[derive(
    Debug, Deserialize, Serialize, Default, PartialEq, Clone, CompositeType, Type, MeilisearchFilter,
)]
pub struct FullMandate {
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub name: Option<String>,
    pub party: Option<String>,
    pub is_nr: Option<bool>,
    pub is_gov_official: Option<bool>,
    pub is_ministry: Option<bool>,
    pub is_chancellor: Option<bool>,
    pub function: Option<String>,
}

#[derive(ToSchema, PartialEq, Debug, Clone, Serialize, Deserialize, Default, MeilisearchFilter)]
pub struct Delegate {
    pub id: Option<i32>,
    pub name: Option<String>,
    pub party: Option<String>,
    pub current_party: Option<String>,
    pub image_url: Option<String>,
    pub constituency: Option<String>,
    pub council: Option<String>,
    pub seat_row: Option<i32>,
    pub seat_col: Option<i32>,
    pub gender: Option<String>,
    pub is_active: Option<bool>,
    pub birthdate: Option<NaiveDate>,
    pub divisions: Option<Vec<String>>,
    pub mandates_at_time: Option<Vec<FullMandate>>,
    pub active_mandates: Option<Vec<FullMandate>>,
    pub mandates: Option<Vec<FullMandate>>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl CombinedData for Delegate {
    const INDEX: crate::Index = crate::Index::Delegates;

    const PRIMARY_KEY: &str = "id";

    fn id(&self) -> u64 {
        self.id.unwrap() as u64
    }
}
