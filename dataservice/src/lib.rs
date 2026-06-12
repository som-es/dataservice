#[cfg(feature = "models")]
pub mod db;

#[cfg(feature = "runner")]
pub mod api_limiter;
#[cfg(feature = "runner")]
pub mod services;
#[cfg(feature = "runner")]
pub mod shutdown;

use std::{cell::LazyCell, env::VarError, sync::Arc};

use redis::aio::ConnectionManager;
#[cfg(feature = "runner")]
pub use services::*;

#[cfg(feature = "runner")]
pub use api_limiter::*;
#[cfg(feature = "runner")]
use chrono::{DateTime, Local, NaiveDate};
#[cfg(feature = "runner")]
pub use db::*;
#[cfg(feature = "runner")]
pub use shutdown::*;
use sqlx::{postgres::PgPoolOptions, PgPool};

pub mod location_hasher;

pub mod ai_task;
pub mod email;
pub mod notify_registrars;
pub mod should_update;
pub use should_update::*;

#[cfg(feature = "runner")]
pub mod ai_tasks;

pub mod combx {
    pub use combx::*;
}

// pub(self) const LEGIS_INIT_STILL_SEND_DAYS: LazyLock<i64> = LazyLock::new(|| {
//     dotenv!("LEGIS_INIT_STILL_SEND_DAYS")
//         .parse::<i64>()
//         .unwrap_or(10)
// });

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
pub const REDIS_URL: &str = dotenv!("REDIS_URL");
pub const LATEST_GP: &str = dotenv!("LATEST_GP");
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

pub async fn connect_pg() -> PgPool {
    PgPoolOptions::new()
        .max_connections(90)
        .min_connections(5)
        .connect(DATABASE_URL)
        .await
        .unwrap()
}

pub async fn connect_pg2() -> PgPool {
    PgPoolOptions::new()
        // pool sizes
        .max_connections(6)
        .connect(DATABASE_URL)
        .await
        .unwrap()
}

#[cfg(test)]
mod tests {
    use sqlx::query;

    use crate::connect_pg;

    #[tokio::test]
    async fn test_duplicate_spoken_text_check() {
        let pg = connect_pg().await;

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
