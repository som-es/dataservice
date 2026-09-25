use chrono::NaiveDate;
use somes_scraper::Thing;
use sqlx::PgPool;

use crate::{PostgresEnhancer, api_models::decrees::enhancer::DecreeEnhancer};

impl DecreeEnhancer for PostgresEnhancer {
    fn gov_officials_at_date(
        &self,
        decree: &Thing,
    ) -> impl Future<Output = Result<Vec<GovOfficialAtDate>, Self::Error>> {
        pg_gov_officials_at_date(&self.0, decree.date)
    }
}

#[derive(Debug)]
pub struct GovOfficialAtDate {
    pub delegate_id: i32,
    pub ministry: Option<String>,
}

pub async fn pg_gov_officials_at_date(
    pg_pool: &PgPool,
    decree_date: NaiveDate,
) -> sqlx::Result<Vec<GovOfficialAtDate>> {
    sqlx::query_as!(GovOfficialAtDate,
        "
        SELECT
            mandates.delegate_id,
            ministry
        FROM
            mandates
        WHERE
            mandates.is_gov_official
            AND ministry is not null
            AND mandates.start_date <= $1::date
            AND name not like '%sekretär%'
            AND (CASE WHEN mandates.end_date IS NULL THEN $1::date ELSE mandates.end_date END) >= $1::date;

        ",
        decree_date
    )
    .fetch_all(pg_pool)
    .await
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::NaiveDate;
    use somes_common_lib::ALL_GPS;
    use somes_scraper::decree::RisScraper;

    use crate::{
        Decree, LegislativePeriod, PostgresEnhancer, connect_pg,
        decrees::enhancer::postgres::pg_gov_officials_at_date, today, with_data::gps::gps,
    };

    #[tokio::test]
    async fn test_gov_officials_at_date() {
        let pg = connect_pg().await;
        let gov_officials =
            pg_gov_officials_at_date(&pg, NaiveDate::from_ymd_opt(2021, 8, 18).unwrap())
                .await
                .unwrap();
        dbg!(gov_officials);
        let scraper = somes_scraper::AustriaScraper::default();
        let thing = scraper
            .fetch_single_ris_entry("BGBLA_2021_II_363")
            .await
            .unwrap();
        dbg!(&thing);

        let mut all_gps = gps(&pg).await.unwrap();
        all_gps.insert(
            0,
            LegislativePeriod {
                gp: "".into(),
                start_date: today(),
            },
        );

        let all_gps: Arc<[LegislativePeriod]> = Arc::from(all_gps);

        let decree = Decree::from_thing(PostgresEnhancer(pg), thing, all_gps).await;
        dbg!(decree);
    }
}
