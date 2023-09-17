use diesel::{Queryable, Selectable, Insertable, AsChangeset, prelude::QueryableByName};
use serde_derive::{Serialize, Deserialize};

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(
    Queryable, Selectable, PartialEq, Debug, Clone, Insertable, Serialize, Deserialize, AsChangeset,
)]
#[diesel(table_name = crate::db::schema::questions)]
#[diesel(primary_key(id))]
pub struct DbQuestion {
    pub issuer_id: i32,
    pub delegate_id: i32,
    pub title: String,
    pub body: String,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(
    Queryable, Selectable, PartialEq, Debug, Clone, Insertable, Serialize, Deserialize, AsChangeset, QueryableByName
)]
#[diesel(table_name = crate::db::schema::questions)]
#[diesel(primary_key(id))]
pub struct DbQuestionQuery {
    pub id: i32,
    pub issuer_id: i32,
    pub delegate_id: i32,
    pub title: String,
    pub body: String,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}
