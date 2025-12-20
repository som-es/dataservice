use std::collections::HashSet;

use sqlx::PgPool;

pub async fn get_all_titles(pg: &PgPool) -> HashSet<String> {
    let names = sqlx::query!("select name from delegates where name like '%.%'")
        .fetch_all(pg)
        .await
        .unwrap();

    let mut unique_titles = HashSet::<String>::new();

    for name in names {
        unique_titles.extend(
            name.name
                .split(' ')
                .filter(|a| a.contains('.'))
                .map(|x| x.trim_matches(','))
                .map(ToString::to_string),
        );
    }
    unique_titles
}

#[cfg(test)]
mod tests {
    use crate::{connect_pg, with_data::get_all_titles};

    #[tokio::test]
    async fn test_get_all_titles() {
        let pg = connect_pg().await;
        let titles = get_all_titles(&pg).await;
        println!("unique titles: {titles:?}");
    }
}
