use diesel::{Queryable, Selectable, Insertable, AsChangeset};
use serde_derive::{Serialize, Deserialize};

use crate::schema::questions;


#[derive(
    Queryable, Selectable, PartialEq, Debug, Clone, Insertable, Serialize, Deserialize, AsChangeset,
)]
#[diesel(table_name = questions)]
#[diesel(primary_key(id))]
pub struct DbQuestion {
    pub issuer_id: i32,
    pub delegate_id: i32,
    pub title: String,
    pub body: String,
}
