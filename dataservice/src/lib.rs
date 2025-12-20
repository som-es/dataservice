#[cfg(feature = "models")]
pub mod db;

#[cfg(feature = "runner")]
pub mod services;
#[cfg(feature = "runner")]
pub mod shutdown;

use std::{cell::LazyCell, sync::Arc};

#[cfg(feature = "runner")]
pub use services::*;

#[cfg(feature = "runner")]
use chrono::NaiveDate;
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
pub(crate) use should_update::*;

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
pub const OCR_ENGINE: LazyCell<Option<tesseract_rs::TesseractAPI>> = LazyCell::new(|| {
    ::combx::init_ocr_engine_tesseract(&tessdata_dir())
        .ok()
});

use dotenvy_macro::dotenv;

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

pub fn tessdata_dir() -> String {
    std::env::var("TESSDATA_DIR").expect("TESSDATA_DIR env var must be set")
}

pub const DATABASE_URL: &str = dotenv!("DATABASE_URL");
pub const LATEST_GP: &str = dotenv!("LATEST_GP");
pub const GPS: [&str; 9] = [
    LATEST_GP, "XXVII", "XXVI", "XXV", "XXIV", "XXIII", "XXII", "XXI", "XX",
];

#[cfg(feature = "runner")]
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[cfg(feature = "runner")]
#[inline]
pub fn today() -> NaiveDate {
    chrono::Local::now().date_naive()
}

#[cfg(feature = "runner")]
#[inline]
pub fn today_and_time() -> chrono::NaiveDateTime {
    chrono::Local::now().naive_local()
}

pub async fn connect_pg() -> PgPool {
    PgPoolOptions::new()
        .max_connections(128)
        .min_connections(4)
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
