use somes_scraper::language::Language;
use sqlx::PgPool;

pub async fn unique_topics_for_lang(pg: &PgPool, language: Language) -> sqlx::Result<Vec<String>> {
    sqlx::query_scalar!(
        "select topic_name as topic from unique_eurovoc_topics where language = $1 order by topic", language.as_str()
    )
    .fetch_all(pg)
    .await
}