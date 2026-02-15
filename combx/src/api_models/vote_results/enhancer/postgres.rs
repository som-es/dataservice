use std::collections::HashMap;

use chrono::NaiveDate;
use somes_scraper::{LegislativeInitiative, Party, ProContra};
use sqlx::PgPool;

use crate::{
    DbLegislativeInitiativeQuery, DbNamedVote, DbSpeechWithLink, LegisInitInfo,
    api_models::vote_results::{
        enhancer::VoteResultEnhancer, hidden_vote_translation::pg_gen_code_to_parties_with_date,
        named_votes::pg_create_db_named_votes,
    },
    enhancer::{Enhancer, PostgresEnhancer},
    with_data::all_parties_by_most_recent_gp,
};

impl VoteResultEnhancer for PostgresEnhancer {
    fn retrieve_plenary_session_id(
        &self,
        legis_init: &LegislativeInitiative,
    ) -> impl Future<Output = Option<i32>> {
        pg_retrieve_plenary_session_id(self.source(), legis_init)
    }

    fn gen_code_to_parties_with_date(
        &self,
        date: &NaiveDate,
        gp: &str,
    ) -> impl Future<Output = (Vec<Party>, HashMap<String, Party>)> {
        async {
            (
                all_parties_by_most_recent_gp(self.source())
                    .await
                    .unwrap_or_default(),
                pg_gen_code_to_parties_with_date(self.source(), date, gp).await,
            )
        }
    }

    fn create_db_named_votes(
        &self,
        votes: &ProContra,
        voted_on: NaiveDate,
        gp: &str,
    ) -> impl Future<Output = Result<Vec<DbNamedVote>, Self::Error>> {
        pg_create_db_named_votes(self.source(), votes, voted_on, gp)
    }

    fn maybe_get_legis_init_info(
        &self,
        inr: i32,
        ityp: &str,
        gp: &str,
    ) -> impl Future<Output = LegisInitInfo> {
        pg_maybe_get_legis_init_id(self.source(), inr, ityp, gp)
    }

    fn get_plenar_speeches(
        &self,
        db_legis_init: &DbLegislativeInitiativeQuery,
        maybe_legis_init_id: i32,
    ) -> impl Future<Output = Result<Vec<DbSpeechWithLink>, Self::Error>> {
        pg_get_plenar_speeches(self.source(), db_legis_init, maybe_legis_init_id)
    }

    fn get_absences_from_legis_init(
        &self,
        maybe_legis_init_id: i32,
    ) -> impl Future<Output = Result<Vec<i32>, Self::Error>> {
        pg_get_absences_from_legis_init(self.source(), maybe_legis_init_id)
    }

    fn get_referenced_by_other_ids(
        &self,
        db_legis_init: &DbLegislativeInitiativeQuery,
    ) -> impl Future<Output = Result<Vec<i32>, sqlx::Error>> {
        pg_get_referenced_by_other_ids(self.source(), db_legis_init)
    }
}

pub async fn pg_get_plenar_speeches(
    pool: &PgPool,
    db_legis_init: &DbLegislativeInitiativeQuery,
    maybe_legis_init_id: i32,
) -> sqlx::Result<Vec<DbSpeechWithLink>> {
    sqlx::query_as!(
        DbSpeechWithLink,
        "
                    SELECT
                        legis_init_id,
                        delegate_id,
                        CASE
                            WHEN opinion = 'Pro' THEN ($2 NOT LIKE '%p%')
                            WHEN opinion = 'Contra' THEN ($2 LIKE '%p%')
                            ELSE NULL
                        END AS infavor,
                        duration_in_seconds,
                        opinion,
                        document_url,
                        about
                    FROM plenar_speeches
                    INNER JOIN plenar_speech_links psl ON plenar_speeches.id = psl.plenar_speech_id
                    INNER JOIN plenar_speech_legis_inits pl ON pl.speech_id = plenar_speeches.id
                    INNER JOIN debates ON debates.id = plenar_speeches.debate_id
                    WHERE legis_init_id = $1
                    ",
        maybe_legis_init_id,
        db_legis_init.pre_declined_type
    )
    .fetch_all(pool)
    .await
}

pub async fn pg_retrieve_plenary_session_id(
    pool: &sqlx::Pool<sqlx::Postgres>,
    legis_init: &LegislativeInitiative,
) -> Option<i32> {
    match legis_init.plenary_session {
        Some(session) => sqlx::query!(
            "select id from plenar_infos where inr = $1 and legislative_period = $2",
            session as i32,
            legis_init.gp
        )
        .fetch_one(pool)
        .await
        .ok()
        .map(|id| id.id),
        None => None,
    }
}

pub async fn pg_maybe_get_legis_init_id(
    pool: &PgPool,
    inr: i32,
    ityp: &str,
    gp: &str,
) -> LegisInitInfo {
    sqlx::query_as!(LegisInitInfo,
        "select id, updated_at, created_at, hash from legislative_initiatives where inr = $1 and ityp = $2 and gp = $3",
        inr,
        ityp,
        gp
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .unwrap_or_default()
}

pub async fn pg_get_absences_from_legis_init(
    pool: &PgPool,
    maybe_legis_init_id: i32,
) -> Result<Vec<i32>, sqlx::Error> {
    sqlx::query!(
        "
                   SELECT delegate_id FROM absences
        WHERE plenary_session_id = (
            SELECT plenary_session_id
            FROM legislative_initiatives
            WHERE id = $1
        ) 
            ",
        maybe_legis_init_id
    )
    .fetch_all(pool)
    .await
    .map(|x| x.into_iter().map(|x| x.delegate_id).collect::<Vec<_>>())
}

pub async fn pg_get_referenced_by_other_ids(
    pool: &PgPool,
    db_legis_init: &DbLegislativeInitiativeQuery,
) -> Result<Vec<i32>, sqlx::Error> {
    sqlx::query!("select origin_legis_init_id from legis_inits_refs where ref_gp = $1 and ref_ityp = $2 and ref_inr = $3", 
        db_legis_init.gp, db_legis_init.ityp, db_legis_init.inr).fetch_all(pool).await
        .map(|x| x.into_iter().map(|x| x.origin_legis_init_id).collect::<Vec<_>>())
}
