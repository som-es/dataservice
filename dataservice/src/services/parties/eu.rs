use combx::DbMandate;
use somes_scraper::{Mandate, parties::DelegateMandate};
use sqlx::PgPool;
use sqlx::types::Json;

fn to_mandate(db: DbMandate, constituency: Option<String>) -> Mandate {
    Mandate {
        name: db.name,
        party: db.party,
        constituency,
        start: db.start_date,
        end: db.end_date,
        function: db.function,
        is_nr: db.is_nr,
        is_gov: db.is_gov_official,
        is_ministry: db.is_ministry,
        is_chancellor: db.is_chancellor,
    }
}

pub async fn get_delegate_mandates(pool: &PgPool) -> Result<Vec<DelegateMandate>, sqlx::Error> {
    let rows = sqlx::query!(
        r#"
        SELECT
            d.id AS "delegate_id!",
            d.constituency,
            COALESCE(latest.party, d.party) AS "party",
            COALESCE(
                json_agg(
                    json_build_object(
                        'delegate_id', m.delegate_id,
                        'is_nr', m.is_nr,
                        'is_gov_official', m.is_gov_official,
                        'function', m.function,
                        'is_ministry', m.is_ministry,
                        'is_chancellor', m.is_chancellor,
                        'ministry', m.ministry,
                        'name', m.name,
                        'party', m.party,
                        'start_date', m.start_date,
                        'end_date', m.end_date
                    )
                    ORDER BY m.start_date DESC
                ) FILTER (WHERE m.id IS NOT NULL),
                '[]'::json
            ) AS "mandates!: Json<Vec<DbMandate>>"
        FROM delegates d
        LEFT JOIN mandates m
            ON m.delegate_id = d.id
        LEFT JOIN LATERAL (
            SELECT m2.party
            FROM mandates m2
            WHERE m2.delegate_id = d.id
              AND m2.party IS NOT NULL
            ORDER BY m2.start_date DESC
            LIMIT 1
        ) latest ON true
        GROUP BY d.id, d.constituency, d.party, latest.party
        ORDER BY d.id
        "#
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| DelegateMandate {
            delegate_id: r.delegate_id as u32,
            party: r.party,
            mandates: r
                .mandates
                .0
                .into_iter()
                .map(|m| to_mandate(m, r.constituency.clone()))
                .collect(),
        })
        .collect())
}
