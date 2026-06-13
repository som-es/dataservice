use somes_scraper::{parties::Parties, AustriaScraper, Party};

use crate::{scrape_untill_success, ApiLimiter, Shutdown, GPS};

use sqlx::PgPool;

pub async fn update_parties_in_db2(pool: &PgPool, parties: &[Party], gp: &str) {
    for party in parties {
        if let Err(e) = sqlx::query!(
            r#"
            INSERT INTO parties (name, fraction, color, code, gp)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (code, gp, name)
            DO UPDATE SET
                fraction = EXCLUDED.fraction,
                code = EXCLUDED.code,
                color = EXCLUDED.color
            "#,
            party.name,
            party.fraction as i32,
            party.color,
            party.code,
            gp
        )
        .execute(pool)
        .await
        {
            log::error!("could not update parties({party:?}) in db: {e:?}")
        }
    }
}

pub async fn get_parties(scraper: &AustriaScraper) -> crate::Result<Vec<Party>> {
    Ok(scraper.get_parties().await?)
}

pub async fn parties_once(pg_pool: &PgPool, parliament_api_limiter: ApiLimiter) {
    let mut handles = vec![];
    for gp in GPS {
        let pg_pool = pg_pool.clone();
        let parliament_api_limiter = parliament_api_limiter.clone();
        let gp = gp.to_string();
        let handle = tokio::task::spawn(async move {
            let pg_pool = pg_pool;
            let mut scraper = AustriaScraper::default();
            scraper.scrape_info.gp = vec![gp.clone().into()];

            let parties = scrape_untill_success::<Vec<_>, _>(|| async {
                parliament_api_limiter
                    .run(scraper.get_parties())
                    .await
                    .map_err(|e| e.into())
            })
            .await;

            update_parties_in_db2(&pg_pool, &parties, &gp).await;
        });
        handles.push(handle);
    }

    for handle in handles {
        let e = handle.await;
        if let Err(e) = e {
            log::error!("could not await parties handle: {e:?}")
        }
    }
}

pub async fn run_shut(mut shutdown: Shutdown, pg_pool: PgPool, parliament_api_limiter: ApiLimiter) {
    let party_sleep = std::env::var("PARTY_SLEEP")
        .unwrap_or_else(|_| "10000".to_string())
        .parse()
        .unwrap_or_else(|e| {
            log::warn!(
                "Could not parse PARTY_SLEEP env var: {e}. Using default value of 10000 seconds"
            );
            10000
        });

    #[cfg(feature = "parties")]
    while !shutdown.is_shutdown() {
        tokio::select! {
            _ = shutdown.recv() => {
                println!("shutting down");
                // return ok?
                break;
            }
            _ = async {
                let pg_pool = pg_pool.clone();
                let parliament_api_limiter = parliament_api_limiter.clone();
                if let Err(e) = tokio::task::spawn(async move {
                    log::info!("Starting party scraper");
                    parties_once(&pg_pool, parliament_api_limiter).await;
                }).await {
                    log::error!("Error in party scraper: {:?}", e);
                }
                log::info!("Party scraper service sleeps for {party_sleep} seconds...");
                tokio::time::sleep(tokio::time::Duration::from_secs(party_sleep)).await;
            } => {}
        }
    }
}
