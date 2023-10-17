use chrono::{NaiveDate, NaiveDateTime};
use diesel::prelude::*;
use serde_derive::{Deserialize, Serialize};

use crate::{db::schema::parties, db::schema::topics};

use super::schema::{
    call_to_order, contacts, debates, delegates, legislative_initiatives, mandates, plenar_infos,
    plenar_speeches, proposal_delegates, proposals, speeches, votes,
};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(
    Queryable, Selectable, Identifiable, PartialEq, Debug, Clone, Insertable, Serialize, Deserialize,
)]
#[diesel(table_name = proposals)]
pub struct DbProposal {
    pub id: String,
    pub ityp: String,
    pub gp: String,
    pub title: String,
    pub description: String,
}
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(Queryable, Selectable, Identifiable, PartialEq, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = proposals)]
pub struct DbProposalQuery {
    pub id: String,
    pub ityp: String,
    pub gp: String,
    pub title: String,
    pub description: String,
    pub created_at: NaiveDateTime,
}
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(
    Queryable,
    Selectable,
    Identifiable,
    PartialEq,
    Debug,
    Clone,
    Insertable,
    AsChangeset,
    Serialize,
    Deserialize,
)]
#[diesel(table_name = delegates)]
#[diesel(primary_key(id))]
pub struct DbDelegate {
    pub id: i32,
    pub name: String,
    pub party: Option<String>,
    pub image_url: Option<String>,
    pub constituency: Option<String>,
    pub council: Option<String>,
    pub seat_row: Option<i32>,
    pub seat_col: Option<i32>,
    pub contacts_id: Option<i32>,
    pub gender: Option<String>,
    pub is_active: Option<bool>,
    pub birthdate: Option<NaiveDate>,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(
    Identifiable,
    Selectable,
    Queryable,
    Associations,
    Debug,
    Clone,
    Insertable,
    Serialize,
    Deserialize,
)]
#[diesel(table_name = proposal_delegates)]
#[diesel(primary_key(proposal_id, delegate_id))]
#[diesel(belongs_to(DbDelegate, foreign_key = delegate_id))]
#[diesel(belongs_to(DbProposal, foreign_key = proposal_id))]
pub struct ProposalDelegate {
    pub proposal_id: String,
    pub delegate_id: i32,
    pub is_receiver: bool,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(
    Queryable, Selectable, PartialEq, Debug, Clone, Insertable, Serialize, Deserialize, AsChangeset,
)]
#[diesel(table_name = legislative_initiatives)]
pub struct DbLegislativeInitiative {
    pub ityp: String,
    pub gp: String,
    pub inr: i32,
    pub emphasis: Option<String>,
    pub title: String,
    pub description: String,
    pub accepted: Option<bool>,
    pub created_at: NaiveDate,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(
    Queryable, Selectable, PartialEq, Debug, Clone, Insertable, Serialize, Deserialize, AsChangeset,
)]
#[diesel(table_name = legislative_initiatives)]
pub struct DbLegislativeInitiativeQuery {
    pub id: i32,
    pub ityp: String,
    pub gp: String,
    pub inr: i32,
    pub emphasis: Option<String>,
    pub title: String,
    pub description: String,
    pub accepted: Option<bool>,
    pub created_at: NaiveDate,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(
    Queryable, Identifiable, Selectable, PartialEq, Debug, Clone, Insertable, Serialize, Deserialize,
)]
#[diesel(table_name = votes)]
#[diesel(primary_key(party, legislative_initiatives_id))]
#[diesel(belongs_to(DbLegislativeInitiative, foreign_key = legislative_initiatives_id))]
pub struct DbVote {
    pub party: String,
    pub fraction: i32,
    pub infavor: bool,
    pub legislative_initiatives_id: i32,
}
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(
    Queryable, Identifiable, Selectable, PartialEq, Debug, Clone, Insertable, Serialize, Deserialize,
)]
#[diesel(table_name = speeches)]
#[diesel(primary_key(delegate_id, legislative_initiatives_id))]
#[diesel(belongs_to(DbLegislativeInitiative, foreign_key = legislative_initiatives_id))]
#[diesel(belongs_to(DbDelegate, foreign_key = delegate_id))]
pub struct DbSpeech {
    pub delegate_id: i32,
    pub infavor: Option<bool>,
    pub legislative_initiatives_id: i32,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(Queryable, Selectable, PartialEq, Debug, Clone, Insertable, Serialize, Deserialize)]
#[diesel(table_name = topics)]
#[diesel(belongs_to(DbLegislativeInitiative, foreign_key = legislative_initiatives_id))]
pub struct DbTopic {
    pub topic: String,
    pub legislative_initiatives_id: i32,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(Queryable, Selectable, Identifiable, PartialEq, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = contacts)]
pub struct DbContactQuery {
    pub id: i32,
    pub mail: Option<String>,
    pub phone_number: Option<String>,
    pub twitter_url: Option<String>,
    pub facebook_url: Option<String>,
    pub website_url: Option<String>,
    pub tiktok_url: Option<String>,
    pub instagram_url: Option<String>,
    pub youtube_url: Option<String>,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(Queryable, Selectable, PartialEq, Debug, Clone, Insertable, Serialize, Deserialize)]
#[diesel(table_name = contacts)]
#[diesel(primary_key(id))]
pub struct DbContact {
    pub mail: Option<String>,
    pub phone_number: Option<String>,
    pub twitter_url: Option<String>,
    pub facebook_url: Option<String>,
    pub website_url: Option<String>,
    pub tiktok_url: Option<String>,
    pub instagram_url: Option<String>,
    pub youtube_url: Option<String>,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(
    Queryable,
    Selectable,
    // Identifiable,
    PartialEq,
    Debug,
    Clone,
    Insertable,
    Serialize,
    Deserialize,
    AsChangeset,
)]
#[diesel(table_name = plenar_infos)]
// #[diesel(primary_key(id))]
pub struct DbPlenar {
    pub inr: i32,
    pub title: String,
    pub description: String,
    pub add_date: NaiveDateTime,
    pub legislative_period: String,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(
    Queryable,
    Identifiable,
    Selectable,
    PartialEq,
    Debug,
    Clone,
    Insertable,
    Serialize,
    Deserialize,
    AsChangeset,
)]
#[diesel(table_name = call_to_order)]
#[diesel(primary_key(id))]
pub struct DbCallToOrder {
    pub id: String,
    pub plenar_id: i32,
    pub inr: i32,
    pub receiver_id: i32,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(
    Queryable, Selectable, PartialEq, Debug, Clone, Insertable, Serialize, Deserialize, AsChangeset,
)]
#[diesel(table_name = debates)]
#[diesel(primary_key(id))]
pub struct DbDebate {
    pub plenar_id: i32,
    pub about: String,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(
    Queryable, Selectable, PartialEq, Debug, Clone, Insertable, Serialize, Deserialize, AsChangeset,
)]
#[diesel(table_name = plenar_speeches)]
#[diesel(primary_key(id, delegate_id, debate_id))]
pub struct DbPlenarSpeech {
    pub delegate_id: i32,
    pub debate_id: i32,
    pub opinion: Option<String>,
    pub spoken_text: Option<String>,
    pub duration_in_seconds: i32,
    pub ty: String,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(
    Queryable, Selectable, PartialEq, Debug, Clone, Insertable, Serialize, Deserialize, AsChangeset,
)]
#[diesel(table_name = mandates)]
#[diesel(primary_key(id))]
pub struct DbMandate {
    pub delegate_id: i32,
    pub name: String,
    pub party: Option<String>,
    pub start_date: NaiveDate,
    pub end_date: Option<NaiveDate>,
}

impl DbMandate {
    #[inline]
    pub fn is_active(&self) -> bool {
        self.end_date.is_none()
    }
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(
    Queryable, Selectable, PartialEq, Debug, Clone, Insertable, Serialize, Deserialize, AsChangeset,
)]
#[diesel(table_name = parties)]
#[diesel(primary_key(id))]
pub struct DbParty {
    pub name: String,
    pub fraction: i32,
    pub color: String,
}
