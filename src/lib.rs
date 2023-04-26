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
