
use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};
use somes_scraper::{AcceptedState, ComeIntoEffectInfo, Document, LegislativeInitiative, ProContra, Proposal, Reference, RelatedDelegate, Vote, VoteStage};
use sqlx::FromRow;

use crate::models::{DbLegisDocumentOptional, DbLegislativeInitiativeQuery, DbNamedVotes, DbSpeechWithLink, DbVote, Topic};

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(Debug, Deserialize, Serialize, Clone, FromRow, Default)]
pub struct MeiliesearchHelper {
    #[serde(default)]
    pub votes: Vec<String>,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(Debug, Deserialize, Serialize, Clone, FromRow, Default)]
pub struct VoteResult {
    pub id: i32,
    pub legislative_initiative: DbLegislativeInitiativeQuery,
    pub votes: Vec<DbVote>,
    pub speeches: Vec<DbSpeechWithLink>,
    pub named_votes: Option<DbNamedVotes>,
    pub topics: Vec<Topic>,
    pub eurovoc_topics: Vec<Topic>,
    pub other_keyword_topics: Vec<Topic>,
    pub documents: Vec<DbLegisDocumentOptional>,
    pub absences: Vec<i32>,
    pub issued_by_dels: Vec<RelatedDelegate>,
    pub referenced_by_others_ids: Vec<i32>,
    pub references: Vec<i32>,
    #[serde(default)]
    pub meilisearch_helper: MeiliesearchHelper,
}

impl VoteResult {
    pub async fn new(legis_init: LegislativeInitiative) -> Self {

        todo!()
    }
}

