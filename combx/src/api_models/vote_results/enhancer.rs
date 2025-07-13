mod postgres;
pub use postgres::*;

use std::collections::HashMap;

use crate::{DbLegislativeInitiativeQuery, DbNamedVote, DbSpeechWithLink, enhancer::Enhancer};
use chrono::NaiveDate;
use somes_scraper::{LegislativeInitiative, Party, ProContra};

pub trait VoteResultEnhancer: Enhancer {
    fn retrieve_plenary_session_id(
        &self,
        legis_init: &LegislativeInitiative,
    ) -> impl Future<Output = Option<i32>>;
    fn gen_code_to_parties_with_date(
        &self,
        date: &NaiveDate,
        gp: &str,
    ) -> impl Future<Output = HashMap<String, Party>>;
    fn create_db_named_votes(
        &self,
        votes: &ProContra,
        voted_on: NaiveDate,
        gp: &str,
    ) -> impl Future<Output = Result<Vec<DbNamedVote>, Self::Error>>;
    fn maybe_get_legis_init_id(
        &self,
        legis_init: &DbLegislativeInitiativeQuery,
    ) -> impl Future<Output = Option<i32>>;

    fn get_plenar_speeches(
        &self,
        db_legis_init: &DbLegislativeInitiativeQuery,
        maybe_legis_init_id: i32,
    ) -> impl Future<Output = Result<Vec<DbSpeechWithLink>, Self::Error>>;

    fn get_absences_from_legis_init(
        &self,
        maybe_legis_init_id: i32,
    ) -> impl Future<Output = Result<Vec<i32>, Self::Error>>;

    fn get_referenced_by_other_ids(
        &self,
        db_legis_init: &DbLegislativeInitiativeQuery,
    ) -> impl Future<Output = Result<Vec<i32>, sqlx::Error>>;
}
