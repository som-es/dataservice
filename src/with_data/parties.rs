use somes_scraper::Party;
use sqlx::PgPool;

pub async fn all_parties_at_gp(pg: &PgPool, gp: &str) -> sqlx::Result<Vec<Party>> {
    let parties = sqlx::query!(
        "select code, fraction, name, color from parties where gp = $1",
        gp
    )
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

pub async fn all_parties(pg: &PgPool) -> sqlx::Result<Vec<Party>> {
    let parties = sqlx::query!(
        "
        WITH RankedParties AS (
            SELECT
                id,
                name,
                gp,
                code,
                fraction,
                color,
                ROW_NUMBER() OVER (PARTITION BY name ORDER BY gp DESC) AS rn
            FROM
                parties
        )
        SELECT
            id,
            name,
            gp,
            code,
            fraction,
            color
        FROM
            RankedParties
        WHERE
            rn = 1; 
    "
    )
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
