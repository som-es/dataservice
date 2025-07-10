pub mod models;
use chrono::NaiveDate;
use dotenvy_macro::dotenv;
pub use models::*;
mod api_models;
pub use api_models::*;
use sqlx::{postgres::PgPoolOptions, PgPool};

mod with_data;


pub const DATABASE_URL: &str = dotenv!("DATABASE_URL");
pub const LATEST_GP: &str = dotenv!("LATEST_GP");
pub const GPS: [&str; 9] = [
    LATEST_GP, "XXVII", "XXVI", "XXV", "XXIV", "XXIII", "XXII", "XXI", "XX",
];

pub async fn connect_pg() -> PgPool {
    PgPoolOptions::new()
        .max_connections(150)
        .connect(DATABASE_URL)
        .await
        .unwrap()
}

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[inline]
pub fn today() -> NaiveDate {
    chrono::Local::now().date_naive()
}

#[inline]
pub fn today_and_time() -> chrono::NaiveDateTime {
    chrono::Local::now().naive_local()
}

