use scraper::{parties::Parties, AustriaScraper, Party};

use crate::{connect_pg, scrape_untill_success, Shutdown};

use sqlx::PgPool;

pub async fn update_parties_in_db2(pool: &PgPool, parties: &[Party]) -> Result<(), sqlx::Error> {
    for party in parties {
        sqlx::query!(
            r#"
            INSERT INTO parties (name, fraction, color)
            VALUES ($1, $2, $3)
            ON CONFLICT (name) DO NOTHING
            "#,
            party.name,
            party.fraction as i32,
            party.color
        )
        .execute(pool)
        .await?;
    }

    Ok(())
}

pub async fn parties_once(pg_pool: &PgPool) {
    let mut scraper = AustriaScraper::default();
    for gp in ["XXVII", "XXVI", "XXV", "XXIV", "XXIII", "XXII", "XXI", "XX"] {
        scraper.scrape_info.gp = vec![gp.into()];

        let parties = scrape_untill_success::<Vec<Party>, _>(|| async {
            scraper.get_parties().await.map_err(|e| e.into())
        })
        .await;

        update_parties_in_db2(pg_pool, &parties).await.unwrap();
    }
}

pub async fn run_shut(mut shutdown: Shutdown) {
    let party_sleep = std::env::var("PARTY_SLEEP")
        .unwrap_or_else(|_| "10000".to_string())
        .parse()
        .unwrap_or_else(|e| {
            log::warn!(
                "Could not parse PARTY_SLEEP env var: {e}. Using default value of 10000 seconds"
            );
            10000
        });

    while !shutdown.is_shutdown() {
        tokio::select! {
            _ = shutdown.recv() => {
                println!("shutting down");
                // return ok?
                break;
            }
            _ = async {
                if let Err(e) = tokio::task::spawn(async move {
                    log::info!("Starting party scraper");
                }).await {
                    log::error!("Error in party scraper: {:?}", e);
                }
                let pg_pool = connect_pg().await;
                parties_once(&pg_pool).await;
                log::info!("Party scraper service sleeps for {party_sleep} seconds...");
                tokio::time::sleep(tokio::time::Duration::from_secs(party_sleep)).await;
            } => {}
        }
    }
}
