use chrono::Months;
use somes_scraper::{dates::{DateFilter, DateRange, Dates, ParliamentDate}, AustriaScraper};
use sqlx::PgPool;

use crate::{today_and_time, Shutdown};

pub async fn save_dates(pg_pool: &PgPool, only_latest: bool) -> crate::Result<()> {
    let scraper = AustriaScraper::default();
    let mut date_filter = DateFilter::default();
    if only_latest {
        date_filter.daterange = Some(DateRange {
            from: Some(today_and_time().checked_sub_months(Months::new(3)).unwrap()),
            to: None,
        });
    }
    let dates = scraper.post_dates_payload(&date_filter).await?;
    for date in scraper.raw_dates_into_parliament_dates(dates).await.unwrap_or_default() {
        save_date(date, pg_pool).await?
    }
    Ok(())
}

pub async fn save_date(date: ParliamentDate, pg_pool: &PgPool) -> sqlx::Result<()> {
    let rec = sqlx::query!(
        r#"
        INSERT INTO dates (
            date, date_sort, time, title, link, appointment_type,
            appointment_type_display, info_link, location, committee,
            year_month, repeated_date, month_year, media_relevant,
            start, "end", past, day_with_date, tour_format
        )
        VALUES (
            $1, $2, $3, $4, $5, $6,
            $7, $8, $9, $10,
            $11, $12, $13, $14,
            $15, $16, $17, $18, $19
        )
        RETURNING id
        "#,
        date.date,
        date.date_sort,
        date.time,
        date.title,
        date.link,
        date.appointment_type,
        date.appointment_type_display,
        date.info_link,
        date.location,
        date.committee,
        date.year_month,
        date.repeated_date,
        date.month_year,
        date.media_relevant,
        date.start,
        date.end,
        date.past,
        date.day_with_date,
        date.tour_format
    )
    .fetch_one(pg_pool)
    .await?;

    let date_id = rec.id;

    for topic in date.topics {
        sqlx::query!(
            r#"
            INSERT INTO dates_topics (date_id, topic)
            VALUES ($1, $2)
            "#,
            date_id,
            topic
        )
        .execute(pg_pool)
        .await?;
    }

    Ok(())
}


pub async fn run_shut_dates(mut shutdown: Shutdown, pg_pool: PgPool) {
    let dates_sleep = std::env::var("DATES_SLEEP")
        .unwrap_or_else(|_| "20000".to_string())
        .parse()
        .unwrap_or_else(|e| {
            log::warn!(
                "Could not parse DATES_SLEEP env var: {e}. Using default value of 20000 seconds"
            );
            20000
        });

    #[cfg(feature = "dates")]
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
                    log::info!("Starting dates Scraper");
                    save_dates(&pg_pool, false).await
                }).await {
                    log::error!("Error in dates Scraper: {:?}", e);
                }
                log::info!("dates Scraper service sleeps for {dates_sleep} seconds...");
                tokio::time::sleep(tokio::time::Duration::from_secs(dates_sleep)).await;
            } => {}
        }
    }
}

pub async fn run_shut_dates_latest(mut shutdown: Shutdown, pg_pool: PgPool) {
    let dates_sleep = std::env::var("DATES_SLEEP_LATEST")
        .unwrap_or_else(|_| "60".to_string())
        .parse()
        .unwrap_or_else(|e| {
            log::warn!(
                "Could not parse DATES_SLEEP_LATEST env var: {e}. Using default value of 60 seconds"
            );
            60
        });

    #[cfg(feature = "dates")]
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
                    log::info!("Starting (latest) dates Scraper");
                    save_dates(&pg_pool, true).await
                }).await {
                    log::error!("Error in (latest) dates Scraper: {:?}", e);
                }
                log::info!("(latest) dates Scraper service sleeps for {dates_sleep} seconds...");
                tokio::time::sleep(tokio::time::Duration::from_secs(dates_sleep)).await;
            } => {}
        }
    }
}