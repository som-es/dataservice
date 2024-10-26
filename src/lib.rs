#[cfg(feature = "models")]
pub mod db;

#[cfg(feature = "runner")]
pub mod services;
#[cfg(feature = "runner")]
pub mod shutdown;

use std::{
    cell::{Cell, LazyCell, OnceCell},
    future,
};

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

// could use variable in main -> pass to services
static mut PG_POOL: Option<PgPool> = None;

pub async fn connect_pg() -> PgPool {
    unsafe {
        match PG_POOL.clone() {
            Some(pg_pool) => pg_pool,
            None => {
                PG_POOL = PgPoolOptions::new()
                    // pool sizes
                    .max_connections(150)
                    .connect(DATABASE_URL)
                    .await
                    .ok();
                PG_POOL.clone().unwrap()
            }
        }
    }
}

pub async fn connect_pg2() -> PgPool {
    PgPoolOptions::new()
        // pool sizes
        .max_connections(6)
        .connect(DATABASE_URL)
        .await
        .unwrap()
}
