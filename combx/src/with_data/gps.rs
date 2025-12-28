use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LegislativePeriod {
    pub gp: String,
    pub start_date: NaiveDate,
}

pub async fn gps(pg: &PgPool) -> sqlx::Result<Vec<LegislativePeriod>> {
    Ok(sqlx::query!(
        "SELECT legislative_period as gp, MIN(add_date) as start_date
            FROM plenar_infos
            GROUP BY gp 
            HAVING COUNT(*) > 1
            ORDER BY start_date desc
        "
    )
    .fetch_all(pg)
    .await?
    .into_iter()
    .map(|gp| LegislativePeriod {
        gp: gp.gp,
        start_date: gp.start_date.unwrap().date(),
    })
    .collect())
}

#[cfg(test)]
mod tests {
    use sqlx::postgres::PgPoolOptions;

    use crate::{DATABASE_URL, with_data::gps::gps};

    #[tokio::test]
    async fn test_gps() {
        let pg = PgPoolOptions::new()
            .max_connections(1)
            .connect(DATABASE_URL)
            .await
            .unwrap();
        let res = gps(&pg).await.unwrap();
        println!("res: {res:?}");
        let res = res.iter().find(|x| x.gp == "XXVII").unwrap();
        assert_eq!(res.start_date.to_string(), "2019-10-23");
    }
}
