use diesel::prelude::*;

use super::schema::{proposals, delegates, proposal_delegates};

#[derive(Queryable, Selectable, Identifiable, PartialEq, Debug, Clone, Insertable)]
#[diesel(table_name = proposals)]
pub struct DbProposal {
    pub id: String,
    pub ityp: String,
    pub gp: String,
    pub title: String,
    pub description: String,
}

#[derive(Queryable, Selectable, Identifiable, PartialEq, Debug, Clone)]
#[diesel(table_name = delegates)]
pub struct DbDelegate {
    pub id: i32,
    pub name: String,
    pub party: String,
    pub constituency: String,
    pub council: String,
    pub seat_row: i32,
    pub seat_col: i32,
}

#[derive(Identifiable, Selectable, Queryable, Associations, Debug, Clone, Insertable)]
#[diesel(table_name = proposal_delegates)]
#[diesel(primary_key(proposal_id, delegate_id))]
#[diesel(belongs_to(DbDelegate, foreign_key = delegate_id))]
#[diesel(belongs_to(DbProposal, foreign_key = proposal_id))]
pub struct ProposalDelegate {
    pub proposal_id: String,
    pub delegate_id: i32,

}