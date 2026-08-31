use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use somes_macro::CompositeType;
use sqlx::{FromRow, Type};
#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, FromRow, Type, CompositeType)]
#[sqlx(type_name = "db_mood")]
pub struct DbMood {
    pub id: i64,
    pub auto_mood: f64,
    pub pre_aggregated_user_mood: Option<f64>,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, FromRow, Type, CompositeType)]
#[sqlx(type_name = "db_user_mood")]
pub struct DbUserMood {
    pub id: i64,
    pub user_mood: f64,
    pub user_id: i32,
    pub mood_id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, FromRow, Type, CompositeType,
)]
#[sqlx(type_name = "db_gov_prop_mood")]
pub struct DbGovPropMood {
    pub id: i64,
    pub gov_prop_id: i32,
    pub mood_id: i64,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, FromRow, Type, CompositeType, Default,
)]
#[sqlx(type_name = "db_mood_barometer")]
pub struct MoodBarometer {
    pub gov_prop_id: i32,
    pub mood_id: i64,
    pub auto_mood: f64,
    pub pre_aggregated_user_mood: Option<f64>,
    pub user_mood: Option<f64>,
}
