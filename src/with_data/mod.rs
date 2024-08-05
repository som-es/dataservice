use scraper::Party;
use sqlx::PgPool;

pub mod activity;
pub mod gps;

pub async fn all_parties_latest_gp(pg: &PgPool) -> sqlx::Result<Vec<Party>> {
    let parties = sqlx::query!("select code, fraction, name, color from parties where gp = $1", crate::LATEST_GP)
        .fetch_all(pg)
        .await?;
    Ok(parties
        .into_iter()
        .map(|party| Party {
            name: party.name,
            code: party.code,
            color: party.color,
            fraction: party.fraction as u32,
        })
        .collect())
}
