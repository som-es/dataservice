use std::sync::Arc;

use combx::{
    with_data::gps::{gps, LegislativePeriod},
    FromScrapedType, PostgresEnhancer,
};
use somes_scraper::{decree::RisScraper, AustriaScraper, Thing};
use sqlx::PgPool;

use crate::{today, Shutdown};

pub async fn save_ministrial_decrees(pages: u32, pg_pool: &PgPool) {
    let scraper = AustriaScraper::default();

    let mut all_gps = gps(pg_pool).await.unwrap();
    all_gps.insert(
        0,
        LegislativePeriod {
            gp: "".into(),
            start_date: today(),
        },
    );

    let all_gps: Arc<[LegislativePeriod]> = Arc::from(all_gps);

    for page in 1..=pages {
        log::info!("Currently on page {page}..");
        let Ok(decrees) = scraper.fetch_and_parse_ministrial_decrees(page).await else {
            log::error!("Cannot scrape ministrial decrees on page {page}");
            continue;
        };

        for decree in decrees {
            if let Err(e) = insert_ministrial_decree(decree, pg_pool, all_gps.clone()).await {
                log::error!("Cannot insert decree: {e:?}");
            }
        }
    }
}

pub async fn insert_ministrial_decree(
    decree: Thing,
    pg_pool: &PgPool,
    all_gps: Arc<[LegislativePeriod]>,
) -> crate::Result<()> {
    let decree =
        combx::Decree::from_scraped_type(PostgresEnhancer(pg_pool.clone()), (decree, all_gps))
            .await;

    if let Some(gov_official_id) = decree.gov_official_id {
        let decree_id = sqlx::query!(
            "
            INSERT INTO ministrial_decrees (
                gov_official_id, 
                ris_id, 
                ministrial_issuer, 
                title, 
                short_title, 
                publication_date, 
                part,
                gp
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (ris_id) DO UPDATE
            SET 
                gov_official_id = EXCLUDED.gov_official_id,
                title = EXCLUDED.title,
                short_title = EXCLUDED.short_title,
                publication_date = EXCLUDED.publication_date,
                part = EXCLUDED.part,
                gp = EXCLUDED.gp
            RETURNING id;
            ",
            gov_official_id,
            decree.ris_id,
            decree.ministrial_issuer,
            decree.title,
            decree.short_title,
            decree.publication_date,
            decree.part,
            decree.gp,
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
                document.document_type,
                document.document_url
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
