mod eu;
use somes_scraper::{
    AustriaScraper, Party,
    eu_parliament::EuParliamentScraper,
    parties::{DelegateMandate, Parties},
};

use crate::{
    ApiLimiter, GPS, Gps, Shutdown, parties::eu::get_delegate_mandates, scrape_untill_success,
};

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
    Ok(scraper.get_parties(&[]).await?)
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
            scraper.scrape_info.gp = vec![gp.clone()];

            scrape_and_update(&parliament_api_limiter, gp, pg_pool, &scraper, &[]).await;
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

#[async_trait::async_trait]
pub trait PartyService: Parties + Send + Sync + Default + Gps {
    async fn run(pg_pool: &PgPool, parliament_api_limiter: ApiLimiter) -> crate::Result<()>;
}

#[async_trait::async_trait]
impl PartyService for EuParliamentScraper {
    async fn run(pg_pool: &PgPool, parliament_api_limiter: ApiLimiter) -> crate::Result<()> {
        let mut scraper = EuParliamentScraper::default();
        let delegates = get_delegate_mandates(pg_pool).await?;

        for gp in ["7", "8", "9", "10"] {
            scraper.scrape_info.gp = vec![gp.into()];
            scrape_and_update(
                &parliament_api_limiter,
                gp.into(),
                pg_pool.clone(),
                &scraper,
                &delegates,
            )
            .await;
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl PartyService for AustriaScraper {
    async fn run(pg_pool: &PgPool, parliament_api_limiter: ApiLimiter) -> crate::Result<()> {
        let _: () = parties_once(pg_pool, parliament_api_limiter).await;
        Ok(())
    }
}

async fn scrape_and_update<S: Parties>(
    parliament_api_limiter: &ApiLimiter,
    gp: String,
    pg_pool: sqlx::Pool<sqlx::Postgres>,
    scraper: &S,
    delegates: &[DelegateMandate],
) {
    let parties = scrape_untill_success::<Vec<_>, _>(|| async {
        parliament_api_limiter
            .run(scraper.get_parties(delegates))
            .await
            .map_err(|e| e.into())
    })
    .await;

    update_parties_in_db2(&pg_pool, &parties, &gp).await;
}

pub async fn run_shut<S: PartyService>(
    _shutdown: Shutdown,
    _pg_pool: PgPool,
    _parliament_api_limiter: ApiLimiter,
) {
    let _party_sleep = std::env::var("PARTY_SLEEP")
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

                    S::run(&pg_pool, parliament_api_limiter).await.unwrap()
                }).await {
                    log::error!("Error in party scraper: {:?}", e);
                }
                log::info!("Party scraper service sleeps for {party_sleep} seconds...");
                tokio::time::sleep(tokio::time::Duration::from_secs(party_sleep)).await;
            } => {}
        }
    }
}
