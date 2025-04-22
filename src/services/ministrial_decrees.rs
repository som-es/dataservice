use somes_scraper::{decree::RisScraper, AustriaScraper, Thing};
use sqlx::PgPool;

use crate::Shutdown;

pub async fn save_ministrial_decrees(pages: u32, pg_pool: &PgPool) {
    let scraper = AustriaScraper::default();

    for page in 1..=pages {
        log::info!("Currently on page {page}..");
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
            AND name not like '%sekretär%'
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
                gov_official_id, 
                ris_id, 
                ministrial_issuer, 
                title, 
                short_title, 
                publication_date, 
                part
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (ris_id) DO UPDATE
            SET 
                gov_official_id = EXCLUDED.gov_official_id,
                title = EXCLUDED.title,
                short_title = EXCLUDED.short_title,
                publication_date = EXCLUDED.publication_date,
                part = EXCLUDED.part
            RETURNING id;
            ",
            fitting_gov_official.delegate_id,
            decree.id,
            decree.issuer,
            decree.title,
            decree.short_title,
            decree.date,
            decree.part
        )
        .fetch_one(pg_pool)
        .await?;
        
        for document in &decree.documents {
            sqlx::query!(
                "
                INSERT INTO ministrial_decrees_documents (
                    ministrial_decree_id, 
                    title, 
                    document_type, 
                    document_url
                ) VALUES ($1, $2, $3, $4)
                ON CONFLICT (ministrial_decree_id, document_url) DO UPDATE
                SET 
                    title = EXCLUDED.title,
                    document_type = EXCLUDED.document_type,
                    document_url = EXCLUDED.document_url;
                ",
                decree_id.id,
                document.title,
                document.ftype.to_string(),
                document.link
            )
            .execute(pg_pool)
            .await?;
        }
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
