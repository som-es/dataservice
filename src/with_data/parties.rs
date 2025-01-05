use chrono::NaiveDate;
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


pub async fn all_parties_at_date(pg: &PgPool, date: &NaiveDate, gp: &str) -> sqlx::Result<Vec<Party>> {
    let party_names = sqlx::query!("
       SELECT 
        party, COUNT(*)
    FROM 
        mandates
    WHERE 
        mandates.is_nr
        AND mandates.start_date <= $1::date 
        AND (CASE WHEN mandates.end_date IS NULL THEN $1::date ELSE mandates.end_date END) >= $1::date
        AND party is not null
    group by party
    ", date).fetch_all(pg).await?;

    let all_parties_at_gp = all_parties_at_gp(pg, gp).await?;

    Ok(party_names.into_iter().flat_map(|party| {
        let corresponding_party = all_parties_at_gp.iter().find(|x| &x.name == party.party.as_ref().unwrap())?;
        Some(Party {
            fraction: party.count.unwrap() as u32,
            name: party.party.unwrap(),
            code: corresponding_party.code.clone(),
            color: corresponding_party.color.clone(),
        })
    }).collect())
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
