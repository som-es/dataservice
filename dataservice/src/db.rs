pub mod api_models;
pub mod models;

pub mod topics;

pub fn at_database_url() -> String {
    std::env::var("AT_DATABASE_URL").unwrap()
}

pub fn eu_database_url() -> String {
    std::env::var("EU_DATABASE_URL").unwrap()
}
