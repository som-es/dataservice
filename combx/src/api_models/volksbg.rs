use chrono::{DateTime, NaiveDate, Utc};
use optional_struct::optional_struct;
use serde::{Deserialize, Serialize};
use somes_common_lib::Document;
use somes_macro::{CompositeType, MeilisearchFilter};
use somes_meilisearch_filter::{FilterArgument, FilterOp};
use sqlx::prelude::{FromRow, Type};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[optional_struct]
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
#[sqlx(type_name = "db_volksbg_eintragungswoche")]
pub struct VolksbgEintragungswoche {
    pub id: i64,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub cut_off_date: Option<NaiveDate>,
    pub online_deadline_utc: Option<DateTime<Utc>>,
    pub polling_stations_url: Option<String>,
    pub volksbgs: Vec<DbVolksbg>,
}

#[optional_struct]
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
#[sqlx(type_name = "db_volksbg")]
pub struct DbVolksbg {
    pub id: i64,
    pub slug: String,
    pub title: String,
    pub description: Option<String>,
    pub overview_url: String,
    pub state: Option<String>,
    pub ruling_date: NaiveDate,
    pub cut_off_date: NaiveDate,
    pub eintragungswoche: Option<i64>,
    #[filter(skip)]
    pub documents: Vec<Document>,
}
