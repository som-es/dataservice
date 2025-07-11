#[cfg(feature = "models")]
pub mod db;

#[cfg(feature = "runner")]
pub mod services;
#[cfg(feature = "runner")]
pub mod shutdown;

#[cfg(feature = "runner")]
pub use services::*;

#[cfg(feature = "runner")]
use chrono::NaiveDate;
#[cfg(feature = "runner")]
pub use db::*;
#[cfg(feature = "runner")]
pub use shutdown::*;
use sqlx::{postgres::PgPoolOptions, PgPool};

pub mod email;
pub mod notify_registrars;
pub mod should_update;
pub use should_update::*;
pub use notify_registrars::*;

use dotenvy_macro::dotenv;

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
        .max_connections(150)
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
