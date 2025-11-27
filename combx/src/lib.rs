mod delegate_matching;
pub mod models;
use chrono::NaiveDate;
pub use delegate_matching::*;
use dotenvy_macro::dotenv;
pub use models::*;
pub mod api_models;
pub use api_models::*;
mod gov_officials;
pub use gov_officials::*;

use sqlx::{PgPool, postgres::PgPoolOptions};

mod enhancer;
pub use enhancer::*;
pub mod with_data;

pub const DATABASE_URL: &str = dotenv!("DATABASE_URL");
pub const LATEST_GP: &str = dotenv!("LATEST_GP");
pub const SOMAI_API_SECRET: &str = dotenv!("SOMAI_API_SECRET");
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
