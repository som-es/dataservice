use std::time::SystemTime;

use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde_derive::{Deserialize, Serialize};

use super::schema::{delegates, legislative_initiatives, proposal_delegates, proposals, votes};

#[derive(Queryable, Selectable, Identifiable, PartialEq, Debug, Clone, Insertable, Serialize, Deserialize)]
#[diesel(table_name = proposals)]
pub struct DbProposal {
    pub id: String,
    pub ityp: String,
    pub gp: String,
    pub title: String,
    pub description: String,
}

#[derive(Queryable, Selectable, Identifiable, PartialEq, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = proposals)]
pub struct DbProposalQuery {
    pub id: String,
    pub ityp: String,
    pub gp: String,
    pub title: String,
    pub description: String,
    pub created_at: NaiveDateTime
}

#[derive(Queryable, Selectable, Identifiable, PartialEq, Debug, Clone, Insertable, AsChangeset, Serialize, Deserialize)]
#[diesel(table_name = delegates)]
pub struct DbDelegate {
    pub id: i32,
    pub name: String,
    pub party: Option<String>,
    pub image_url: Option<String>,
    pub constituency: Option<String>,
    pub council: Option<String>,
    pub seat_row: Option<i32>,
    pub seat_col: Option<i32>,
}

#[derive(Identifiable, Selectable, Queryable, Associations, Debug, Clone, Insertable, Serialize, Deserialize)]
#[diesel(table_name = proposal_delegates)]
#[diesel(primary_key(proposal_id, delegate_id))]
#[diesel(belongs_to(DbDelegate, foreign_key = delegate_id))]
#[diesel(belongs_to(DbProposal, foreign_key = proposal_id))]
pub struct ProposalDelegate {
    pub proposal_id: String,
    pub delegate_id: i32,
    pub is_receiver: bool,
}

#[derive(Queryable, Selectable, Identifiable, PartialEq, Debug, Clone, Insertable, Serialize, Deserialize)]
#[diesel(table_name = legislative_initiatives)]
pub struct DbLegislativeInitiative {
    pub id: String,
    pub ityp: String,
    pub gp: String,
    pub title: String,
    pub description: String,
    pub accepted: bool,
    pub created_at: NaiveDateTime,
}

#[derive(Queryable, Identifiable, Selectable, PartialEq, Debug, Clone, Insertable, Serialize, Deserialize)]
#[diesel(table_name = votes)]
#[diesel(primary_key(party, legislative_initiatives_id))]
#[diesel(belongs_to(DbLegislativeInitiative, foreign_key = legislative_initiatives_id))]
pub struct DbVote {
    pub party: String,
    pub fraction: i32,
    pub infavor: bool,
    pub legislative_initiatives_id: String,
}
