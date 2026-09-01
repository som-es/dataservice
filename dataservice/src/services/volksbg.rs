use somes_scraper::{
    AustriaScraper, VolksbgEintragungswoche,
    error::ScraperError,
    volksbg::{BMI_VOLKSBEGEHREN_URL, VolksbgScraper},
};
use sqlx::PgPool;

use crate::{AppState, Shutdown};

pub async fn scrape_volksbg_of(url: &str) -> Result<Option<VolksbgEintragungswoche>, ScraperError> {
    let scraper = AustriaScraper::default();
    let Some(eintragungswoche) = scraper.eintragungswoche_of(url).await? else {
        return Ok(None);
    };

    Ok(Some(
        scraper
            .eintragungswoche_with_details(&eintragungswoche)
            .await?,
    ))
}

pub async fn insert_eintragungswoche_with_volksbegehren(
    pool: &PgPool,
    week: &VolksbgEintragungswoche,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    let week_row = sqlx::query!(
        r#"
        INSERT INTO volksbg_eintragungswoche (
            start_date,
            end_date,
            cut_off_date,
            online_deadline_utc,
            polling_stations_url
        )
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (start_date, end_date)
        DO UPDATE SET
            cut_off_date = EXCLUDED.cut_off_date,
            online_deadline_utc = EXCLUDED.online_deadline_utc,
            polling_stations_url = EXCLUDED.polling_stations_url
        RETURNING id
        "#,
        week.start,
        week.end,
        week.cut_off_date,
        week.online_deadline_utc,
        week.polling_stations_url,
    )
    .fetch_one(&mut *tx)
    .await?;

    for volksbg in &week.volksbegehren {
        let volksbg_row = sqlx::query!(
            r#"
            INSERT INTO volksbg (
                slug,
                title,
                description,
                overview_url,
                state,
                ruling_date,
                cut_off_date,
                eintragungswoche
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (slug)
            DO UPDATE SET
                title = EXCLUDED.title,
                description = EXCLUDED.description,
                overview_url = EXCLUDED.overview_url,
                state = EXCLUDED.state,
                ruling_date = EXCLUDED.ruling_date,
                cut_off_date = EXCLUDED.cut_off_date,
                eintragungswoche = EXCLUDED.eintragungswoche
            RETURNING id
            "#,
            volksbg.id,
            volksbg.title,
            volksbg.description,
            volksbg.overview_url,
            volksbg.state.as_ref().map(|s| s.to_string()),
            volksbg.ruling_date,
            volksbg.cut_off_date,
            week_row.id,
        )
        .fetch_one(&mut *tx)
        .await?;

        for document in &volksbg.documents {
            sqlx::query!(
                r#"
                INSERT INTO volksbg_documents (
                    volksbg_id,
                    title,
                    document_type,
                    document_url
                )
                VALUES ($1, $2, $3, $4)
                ON CONFLICT (document_url, volksbg_id)
                DO UPDATE SET
                    title = EXCLUDED.title,
                    document_type = EXCLUDED.document_type,
                    document_url = EXCLUDED.document_url
                "#,
                volksbg_row.id,
                if document.title.is_empty() {
                    None
                } else {
                    Some(document.title.as_str())
                },
                document.ftype.to_string(),
                document.link,
            )
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;

    Ok(())
}

pub async fn run_volksbg(app_state: AppState, url: &str) -> crate::Result<()> {
    let Some(eintragungswoche) = scrape_volksbg_of(url).await? else {
        log::info!("There currently is not an eintragungswoche");
        return Ok(());
    };

    insert_eintragungswoche_with_volksbegehren(&app_state.pg_pool, &eintragungswoche).await?;

    Ok(())
}

const JUNE_2026_VOLKSBG: &str =
    "https://web.archive.org/web/20260614003610/https://www.bmi.gv.at/411/start.html";

#[cfg(not(feature = "volksbg"))]
pub async fn run_shut(_shutdown: Shutdown, _app_state: AppState) {}

#[cfg(feature = "volksbg")]
pub async fn run_shut(mut shutdown: Shutdown, app_state: AppState) {
    let volksbg_sleep = std::env::var("VOLKSBG_SLEEP")
        .unwrap_or_else(|_| "10900".to_string())
        .parse()
        .unwrap_or_else(|e| {
            log::warn!(
                "Could not parse VOLKSBG_SLEEP env var: {e}. Using default value of 10900 seconds"
            );
            10
        });
    if let Err(e) = run_volksbg(app_state.clone(), JUNE_2026_VOLKSBG).await {
        log::error!("could not run: {e:?}");
    }

    while !shutdown.is_shutdown() {
        tokio::select! {
            _ = shutdown.recv() => {
                println!("shutting down");
                break;
            }
            _ = async {
                let app_state = app_state.clone();
                log::info!("Starting volksbg service");
                if let Err(e) =
                    run_volksbg(app_state, BMI_VOLKSBEGEHREN_URL)
                        .await
                {
                    log::error!("could not complete volksbg run: {e:?}")
                }
                log::info!("volksbg service sleeps for {volksbg_sleep} seconds...");
                tokio::time::sleep(tokio::time::Duration::from_secs(volksbg_sleep)).await;
            } => {}
        }
    }
}
