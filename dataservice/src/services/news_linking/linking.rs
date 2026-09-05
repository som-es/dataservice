use std::collections::HashMap;

use chrono::{DateTime, Days, FixedOffset, NaiveDate, NaiveTime};
use qdrant_client::{
    Qdrant,
    qdrant::{Condition, DatetimeRange, Filter, SearchPoints, Timestamp, value::Kind},
};
use sitemaps::Config;
use somai_client::SomAiClient;

use crate::{AppState, news_linking::ARTICLE_URLS_COLLECTION};

pub const TOP_ARTICLE_URLS: usize = 12;

#[derive(Debug, Clone)]
pub struct ArticleLink {
    pub provider: String,
    pub url: String,
    pub score: f32,
    pub lastmod: DateTime<FixedOffset>,
}

fn start_of_day(date: NaiveDate) -> Timestamp {
    Timestamp {
        seconds: date.and_time(NaiveTime::MIN).and_utc().timestamp(),
        nanos: 0,
    }
}

pub async fn top_article_urls(
    qdrant: &Qdrant,
    ai_client: &SomAiClient,
    provider: &str,
    query: &str,
    lastmod: Option<(NaiveDate, NaiveDate)>,
) -> crate::Result<Vec<ArticleLink>> {
    let embedding = ai_client
        .embedding_embed(query.to_string(), None)
        .await?
        .embedding;

    let mut must = vec![Condition::matches("provider", provider.to_string())];
    if let Some((from, to)) = lastmod {
        must.push(Condition::datetime_range(
            "lastmod",
            DatetimeRange {
                gte: Some(start_of_day(from)),
                lte: Some(start_of_day(to)),
                ..Default::default()
            },
        ));
    }

    let response = qdrant
        .search_points(SearchPoints {
            collection_name: ARTICLE_URLS_COLLECTION.to_string(),
            vector: embedding.into_iter().map(|value| value as f32).collect(),
            filter: Some(Filter::must(must)),
            limit: TOP_ARTICLE_URLS as u64,
            with_payload: Some(true.into()),
            ..Default::default()
        })
        .await?;

    Ok(response
        .result
        .iter()
        .filter_map(|point| {
            let Kind::StringValue(url) = point.payload.get("url")?.kind.as_ref()? else {
                return None;
            };
            let Kind::StringValue(lastmod) = point.payload.get("lastmod")?.kind.as_ref()? else {
                return None;
            };
            let lastmod = DateTime::<FixedOffset>::parse_from_rfc3339(lastmod).unwrap();

            Some(ArticleLink {
                url: url.clone(),
                score: point.score,
                lastmod,
                provider: provider.to_string(),
            })
        })
        .collect())
}

pub async fn top_articles_per_provider(
    qdrant: &Qdrant,
    ai_client: &SomAiClient,
    title: &str,
    date_range: Option<(NaiveDate, NaiveDate)>,
) -> crate::Result<HashMap<String, Vec<ArticleLink>>> {
    let config = Config::builtin();

    let mut provider_articles = HashMap::new();
    for provider in config.provider_ids() {
        provider_articles.insert(
            provider.to_string(),
            top_article_urls(qdrant, ai_client, provider, title, date_range).await?,
        );
    }
    Ok(provider_articles)
}

pub async fn run_news_linking(
    app_state: AppState,
    qdrant: &Qdrant,
    ai_client: &SomAiClient,
    only_recents: bool,
) -> crate::Result<()> {
    let legis_inits = sqlx::query!(
        r#"select
            li.id as legis_init_id, short_title, raw_data_created_at as "start", nr_plenary_activity_date
        from
            legislative_initiative_summaries lis
        inner join summaries on lis.summary_id = summaries.id
        inner join legislative_initiatives li on li.id = lis.legis_init_id
        where
            not $1
            or nr_plenary_activity_date >= current_date - interval '30 days'
        order by lis.legis_init_id"#,
        only_recents
    )
    .fetch_all(&app_state.pg_pool)
    .await?;

    for legis_init in legis_inits {
        let end = legis_init
            .nr_plenary_activity_date
            .checked_add_days(Days::new(10))
            .unwrap();
        let start = legis_init
            .start
            .date_naive()
            .checked_sub_days(Days::new(10))
            .unwrap();

        match top_articles_per_provider(
            qdrant,
            ai_client,
            &legis_init.short_title,
            Some((start, end)),
        )
        .await
        {
            Ok(top_articles) => {
                let mut tx = match app_state.pg_pool.begin().await {
                    Ok(tx) => tx,
                    Err(e) => {
                        log::error!("cannot start transaction: {e:?}");
                        continue;
                    }
                };

                if let Err(e) = sqlx::query!(
                    "delete from article_legis_init_links where legis_init_id = $1",
                    &legis_init.legis_init_id
                )
                .execute(&mut *tx)
                .await
                {
                    log::error!("cannot delete articles legis init links because of {e:?}");
                }

                for articles in top_articles.values() {
                    for article in articles {
                        if article.score < 0.56 {
                            continue;
                        }
                        if let Err(e) = sqlx::query!(
                            "insert into
                                article_legis_init_links (legis_init_id, provider, url, score, lastmod)
                            values ($1, $2, $3, $4, $5)", legis_init.legis_init_id, article.provider, article.url, article.score as f64, article.lastmod
                        ).execute(&mut *tx).await {
                            log::error!("cannot insert into article legis init links: {e:?}")
                        }
                    }
                }

                if let Err(e) = tx.commit().await {
                    log::error!("cannot commit transaction: {e:?}");
                }
            }
            Err(e) => {
                log::error!(
                    "could not retrieve top articles for legis init {:?} because of {e:?}",
                    legis_init.legis_init_id
                )
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::{Days, NaiveDate};
    use qdrant_client::Qdrant;
    use somai_client::SomAiClient;

    use crate::news_linking::linking::top_article_urls;

    #[tokio::test]
    async fn test_top_articles_for_query() {
        let titles_to_compare = vec![
            (
                "Kurzfristige Margenbegrenzung bei Kraftstoffpreisen",
                NaiveDate::from_ymd_opt(2026, 3, 23).unwrap(),
                NaiveDate::from_ymd_opt(2026, 3, 25).unwrap(),
            ),
            (
                "Günstiger-Strom-Gesetz zur Reform der Elektrizitätswirtschaft",
                NaiveDate::from_ymd_opt(2025, 11, 20).unwrap(),
                NaiveDate::from_ymd_opt(2025, 12, 12).unwrap(),
            ),
        ];

        let _ = simple_logger::init_with_level(log::Level::Info);
        let somai_client =
            SomAiClient::new("http://10.30.0.164:8000", "API_USER", "supersicher123!")
                .await
                .unwrap();

        let qdrant = Qdrant::from_url("http://10.30.0.164:6334")
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap();

        for (title, start, end) in titles_to_compare {
            let start = start.checked_sub_days(Days::new(5)).unwrap();
            let end = end.checked_add_days(Days::new(5)).unwrap();
            let articles = top_article_urls(
                &qdrant,
                &somai_client,
                "derstandard",
                title,
                Some((start, end)),
            )
            .await
            .unwrap();
            dbg!(&articles);
        }
    }
}
