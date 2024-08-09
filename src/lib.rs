#[cfg(feature = "models")]
pub mod db;

#[cfg(feature = "runner")]
pub mod services;
#[cfg(feature = "runner")]
pub mod shutdown;

use dotenv_codegen::dotenv;
#[cfg(feature = "runner")]
pub use services::*;

#[cfg(feature = "runner")]
use chrono::NaiveDate;
#[cfg(feature = "runner")]
pub use db::*;
#[cfg(feature = "runner")]
pub use shutdown::*;
use sqlx::{postgres::PgPoolOptions, PgPool};

pub mod with_data;

pub const DATABASE_URL: &str = dotenv!("DATABASE_URL");
pub const LATEST_GP: &str = dotenv!("LATEST_GP");

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
        // pool sizes
        .max_connections(8)
        .connect(DATABASE_URL)
        .await
        .unwrap()
}
