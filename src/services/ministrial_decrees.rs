use somes_scraper::{decree::RisScraper, AustriaScraper, Thing};
use sqlx::PgPool;

use crate::Shutdown;

pub async fn save_ministrial_decrees(pages: u32, pg_pool: &PgPool) {
    let scraper = AustriaScraper::default();

    for page in 1..=pages {
        let Ok(decrees) = scraper.fetch_and_parse_ministrial_decrees(page).await else {
            log::error!("Cannot scrape ministrial decrees on page {page}");
            continue;
        };

        for decree in decrees {
            if let Err(e) = insert_ministrial_decree(&decree, pg_pool).await {
                log::error!("Cannot insert decree: {e:?}");
            }
        }
    }
}

pub async fn insert_ministrial_decree(decree: &Thing, pg_pool: &PgPool) -> crate::Result<()> {
    let gov_officials_at_date = sqlx::query!(
        "
        SELECT 
            mandates.delegate_id, 
            ministry
        FROM 
            mandates 
        WHERE 
            mandates.is_gov_official
            AND ministry is not null
            AND mandates.start_date <= $1::date 
            AND (CASE WHEN mandates.end_date IS NULL THEN $1::date ELSE mandates.end_date END) >= $1::date;

        ",
        decree.date
    )
    .fetch_all(pg_pool)
    .await?;

    if let Some(fitting_gov_official) = gov_officials_at_date
        .iter()
        .find(|x| x.ministry.as_ref() == Some(&decree.issuer))
    {
        let decree_id = sqlx::query!(
            "
            INSERT INTO ministrial_decrees (
                decree_id, 
                date, 
                title, 
                text, 
                issuer, 
                ministry, 
                delegate_id
            ) 
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (decree_id) DO UPDATE
            SET 
                date = EXCLUDED.date,
                title = EXCLUDED.title,
                text = EXCLUDED.text,
                issuer = EXCLUDED.issuer,
                ministry = EXCLUDED.ministry,
                delegate_id = EXCLUDED.delegate_id
            RETURNING decree_id
            ",
            decree.decree_id,
            decree.date,
            decree.title,
            decree.text,
            decree.issuer,
            decree.ministry,
            fitting_gov_official.delegate_id
        )
        .execute(pg_pool)
        .await?;
    }

    Ok(())
}

pub async fn run_shut_pages(mut shutdown: Shutdown, pg_pool: PgPool, pages: u32) {
    let decree_sleep = std::env::var("DECREE_SLEEP")
        .unwrap_or_else(|_| "1000".to_string())
        .parse()
        .unwrap_or_else(|e| {
            log::warn!(
                "Could not parse DECREE SLEEP env var: {e}. Using default value of 1000 seconds"
            );
            1000
        });

    #[cfg(feature = "decrees")]
    while !shutdown.is_shutdown() {
        tokio::select! {
            _ = shutdown.recv() => {
                println!("shutting down");
                // return ok?
                break;
            }
            _ = async {
                let pg_pool = pg_pool.clone();
                if let Err(e) = tokio::task::spawn(async move {
                    log::info!("Starting Decree Scraper");
                    save_ministrial_decrees(pages, &pg_pool).await;
                }).await {
                    log::error!("Error in Decree Scraper: {:?}", e);
                }
                log::info!("Decree Scraper service sleeps for {decree_sleep} seconds...");
                tokio::time::sleep(tokio::time::Duration::from_secs(decree_sleep)).await;
            } => {}
        }
    }
}
