use sqlx::{PgPool, postgres::PgPoolOptions};
use upload_political_questions::upload_questions::{
    GENERATED_QUESTIONS, LONG_QUESTIONS, QUESTIONS, SHORT_QUESTIONS, upload_questions,
};

pub async fn connect_pg(url: &str) -> PgPool {
    PgPoolOptions::new()
        .max_connections(1)
        .connect(url)
        .await
        .unwrap()
}

#[tokio::main]
async fn main() -> combx::Result<()> {
    dotenvy::dotenv()?;
    let pool = connect_pg(&std::env::var("AT_DATABASE_URL")?).await;

    upload_questions(&pool, QUESTIONS).await?;
    upload_questions(&pool, GENERATED_QUESTIONS).await?;

    upload_questions(&pool, SHORT_QUESTIONS).await?;
    upload_questions(&pool, LONG_QUESTIONS).await?;
    Ok(())
}
