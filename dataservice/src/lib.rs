#[cfg(feature = "models")]
pub mod db;

#[cfg(feature = "runner")]
pub mod api_limiter;
#[cfg(feature = "runner")]
pub mod services;
#[cfg(feature = "runner")]
pub mod shutdown;

use std::{cell::LazyCell, env::VarError};

use ::combx::Parliament;
use chrono::Datelike;
use redis::aio::ConnectionManager;
#[cfg(feature = "runner")]
pub use services::*;

#[cfg(feature = "runner")]
pub use api_limiter::*;
#[cfg(feature = "runner")]
use chrono::{DateTime, Local};
#[cfg(feature = "runner")]
pub use db::*;
#[cfg(feature = "runner")]
pub use shutdown::*;
use somes_scraper::{AustriaScraper, eu_parliament::EuParliamentScraper};
use sqlx::{PgPool, postgres::PgPoolOptions};

pub mod location_hasher;

pub mod ai_task;
pub mod email;
pub mod notify_registrars;

pub use update_notify::*;

pub mod should_update;
pub use should_update::*;

#[cfg(feature = "runner")]
pub mod ai_tasks;

pub mod combx {
    pub use combx::*;
}

#[cfg(not(feature = "tesseract"))]
use std::sync::Arc;

#[cfg(not(feature = "tesseract"))]
pub const OCR_ENGINE: LazyCell<Option<Arc<ocrs::OcrEngine>>> = LazyCell::new(|| {
    ::combx::init_ocr_engine(&model_dir())
        .ok()
        .map(|engine| Arc::new(engine))
});

#[cfg(feature = "tesseract")]
fn init_engine() -> crate::Result<tesseract_rs::TesseractAPI> {
    Ok(::combx::init_ocr_engine_tesseract(&tessdata_dir()?)?)
}

#[cfg(feature = "tesseract")]
pub const OCR_ENGINE: LazyCell<Option<tesseract_rs::TesseractAPI>> =
    LazyCell::new(|| match init_engine() {
        Ok(engine) => Some(engine),
        Err(e) => {
            log::error!("Could not init tesseract engine => OCR actions disabled: {e:?}");
            None
        }
    });

use dotenvy_macro::dotenv;

#[cfg(feature = "runner")]
use tokio::sync::mpsc::Sender;

use crate::ai_task::AiTask;

#[cfg(feature = "runner")]
pub async fn setup_app_data() -> tokio::io::Result<()> {
    let home = std::env::var("HOME")
        .map_err(|_| tokio::io::Error::from(tokio::io::ErrorKind::NotFound))?;
    let path = format!("{}/.dataservice", home);

    if tokio::fs::try_exists(&path).await? {
        return Ok(());
    }
    tokio::fs::create_dir(&path).await
}

pub fn model_dir() -> String {
    let home = std::env::var("HOME").expect("HOME env var must be set");
    format!("{}/.dataservice", home)
}

pub fn tessdata_dir() -> std::result::Result<String, VarError> {
    std::env::var("TESSDATA_DIR")
}

pub const DATABASE_URL: &str = dotenv!("DATABASE_URL");
pub const AT_DATABASE_URL: &str = dotenv!("AT_DATABASE_URL");
pub const EU_DATABASE_URL: &str = dotenv!("EU_DATABASE_URL");
pub const REDIS_URL: &str = dotenv!("REDIS_URL");
pub const LATEST_GP: &str = dotenv!("LATEST_GP");
pub const EU_LATEST_GP: &str = dotenv!("EU_LATEST_GP");
pub const GPS: [&str; 9] = [
    LATEST_GP, "XXVII", "XXVI", "XXV", "XXIV", "XXIII", "XXII", "XXI", "XX",
];

#[derive(Clone)]
pub struct AppState {
    pub pg_pool: PgPool,
    pub redis_client: Option<ConnectionManager>,
    #[cfg(feature = "runner")]
    pub parliament_api_limiter: ApiLimiter,
    #[cfg(feature = "runner")]
    pub cpu_task_tx: Option<Sender<AiTask>>,
    pub parliament: Parliament,
}

#[cfg(feature = "runner")]
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[cfg(feature = "runner")]
#[inline]
pub fn today() -> DateTime<Local> {
    chrono::Local::now()
}

#[cfg(feature = "runner")]
#[inline]
pub fn today_and_time() -> chrono::DateTime<Local> {
    chrono::Local::now()
}

pub async fn connect_pg(db_url: &str) -> PgPool {
    PgPoolOptions::new()
        .max_connections(90)
        .min_connections(5)
        .connect(db_url)
        .await
        .unwrap()
}

pub async fn connect_pg2(db_url: &str) -> PgPool {
    PgPoolOptions::new()
        // pool sizes
        .max_connections(6)
        .connect(db_url)
        .await
        .unwrap()
}

pub trait Gps {
    const PARLIAMENT: Parliament;
    const LATEST_GP: &'static str;
    fn gps() -> Vec<String>;
}

impl Gps for AustriaScraper {
    const PARLIAMENT: Parliament = Parliament::At;

    fn gps() -> Vec<String> {
        GPS.iter().map(|gp| gp.to_string()).collect()
    }

    const LATEST_GP: &'static str = LATEST_GP;
}

impl Gps for EuParliamentScraper {
    const PARLIAMENT: Parliament = Parliament::Eu;
    fn gps() -> Vec<String> {
        let start = 2014;
        let end = today().year();
        (start..=end)
            .into_iter()
            .map(|year| year.to_string())
            .rev()
            .collect()
    }

    const LATEST_GP: &'static str = EU_LATEST_GP;
}
#[cfg(test)]
mod tests {
    use sqlx::query;

    use crate::{AT_DATABASE_URL, connect_pg2};

    #[tokio::test]
    async fn test_duplicate_spoken_text_check() {
        let pg = connect_pg2(AT_DATABASE_URL).await;

        let res = query!(
            "SELECT spoken_text, COUNT(*) as c
FROM plenar_speeches
WHERE spoken_text IS NOT NULL
GROUP BY spoken_text
HAVING COUNT(*) > 1 order by c desc;"
        )
        .fetch_all(&pg)
        .await
        .unwrap();

        for (idx, ex) in res.iter().enumerate() {
            println!("ex: {ex:?}");
            println!();

            if idx == 100 {
                break;
            }
        }
    }
}
