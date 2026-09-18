use std::collections::HashMap;

use chrono::{DateTime, Days, FixedOffset, NaiveDate, NaiveTime};
use combx::ArticleLink;
use qdrant_client::{
    Qdrant,
    qdrant::{
        Condition, DatetimeRange, Filter, SearchParams, SearchPoints, Timestamp, value::Kind,
    },
};
use sitemaps::Config;
use somai_client::SomAiClient;

use crate::{
    AppState,
    news_linking::{ARTICLE_URLS_COLLECTION, upload_and_embed::extract_relevant_parts_from_loc},
};

pub const TOP_ARTICLE_URLS: usize = 20;
pub const SCORE_STORE_THRESHOLD: f32 = -1.5;

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
            params: Some(SearchParams {
                exact: Some(true),
                ..Default::default()
            }),
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

pub async fn top_articles_per_provider_reranked(
    qdrant: &Qdrant,
    ai_client: &SomAiClient,
    title: &str,
    summary: &str,
    date_range: Option<(NaiveDate, NaiveDate)>,
) -> crate::Result<HashMap<String, Vec<ArticleLink>>> {
    let config = Config::builtin();

    let mut provider_articles = HashMap::new();
    for provider in config.provider_ids() {
        let articles = top_article_urls(qdrant, ai_client, provider, title, date_range).await?;
        if articles.is_empty() {
            continue;
        }
        let to_rerank = articles
            .iter()
            .map(|article| extract_relevant_parts_from_loc(&article.url))
            .collect::<Vec<_>>();
        let reranked_docs = ai_client
            .embedding_rerank(summary.to_string(), to_rerank.clone(), None, None)
            .await
            .unwrap();

        let mut scores = vec![];
        for doc in reranked_docs.results {
            let mut article = articles[doc.index].clone();
            article.score = doc.score as f32;
            scores.push(article);
        }
        scores.sort_by(|a, b| {
            a.score
                .partial_cmp(&b.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        provider_articles.insert(provider.to_string(), scores);
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
            li.id as legis_init_id, short_title, detailed_summary, short_summary, raw_data_created_at as "start", nr_plenary_activity_date
        from
            legislative_initiative_summaries lis
        inner join summaries on lis.summary_id = summaries.id
        inner join legislative_initiatives li on li.id = lis.legis_init_id
        where
            not $1
            or nr_plenary_activity_date >= current_date - interval '30 days'
        order by nr_plenary_activity_date desc"#,
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

        match top_articles_per_provider_reranked(
            qdrant,
            ai_client,
            &legis_init.short_title,
            &legis_init.detailed_summary,
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
                        if article.score < SCORE_STORE_THRESHOLD {
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

    use crate::{
        AT_DATABASE_URL, connect_pg2, news_linking::linking::top_articles_per_provider_reranked,
    };

    #[tokio::test]
    async fn test_legis_init_by_path() {
        let _ = simple_logger::init_with_level(log::Level::Info);
        let somai_client =
            SomAiClient::new("http://10.30.0.164:8000", "API_USER", "supersicher123!")
                .await
                .unwrap();

        let qdrant = Qdrant::from_url("http://10.30.0.164:6334")
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap();

        let pg = connect_pg2(AT_DATABASE_URL).await;

        let legis_init = sqlx::query!(
            r#"select
                    li.id as legis_init_id, short_title, detailed_summary, short_summary, raw_data_created_at as "start", nr_plenary_activity_date
                from
                    legislative_initiative_summaries lis
                inner join summaries on lis.summary_id = summaries.id
                inner join legislative_initiatives li on li.id = lis.legis_init_id
                where gp = 'XXVIII' and ityp = 'A' and inr = 504"#
        ).fetch_one(&pg).await.unwrap();

        let end = legis_init
            .nr_plenary_activity_date
            .checked_add_days(Days::new(10))
            .unwrap();
        let start = legis_init
            .start
            .date_naive()
            .checked_sub_days(Days::new(10))
            .unwrap();

        let res = top_articles_per_provider_reranked(
            &qdrant,
            &somai_client,
            &legis_init.short_title,
            &legis_init.detailed_summary,
            Some((start, end)),
        )
        .await
        .unwrap();
        dbg!(res);
    }

    #[tokio::test]
    // #[ignore]
    async fn test_top_articles_for_query() {
        let titles_to_compare = vec![
            (
                "Kurzfristige Margenbegrenzung bei Kraftstoffpreisen",
                "Der Gesetzentwurf ergänzt das Preisgesetz 1992 um eine Verordnungsermächtigung, mit der die Bundesregierung bei außergewöhnlichen Preissteigerungen von Diesel oder Euro‑Super (mehr als 30 % im Vergleich zu den Preisen zwei Monate zuvor) vorübergehend Margen festlegen kann. Die festgelegten Margen dürfen die Versorgungssicherheit und den Wirtschaftsstandort nicht gefährden; sie gelten höchstens einen Monat und werden von der Regulierungsbehörde E‑Control überwacht. Der neue Artikel I verankert die Kompetenz der Bundesregierung, solche Maßnahmen zu ergreifen, während weitere formale Änderungen das Wort „ist“ im § 8 Abs. 1 entfernen und die Befristung der Regelung bis zum 30.12.2026 festlegen.",
                NaiveDate::from_ymd_opt(2026, 3, 23).unwrap(),
                NaiveDate::from_ymd_opt(2026, 3, 25).unwrap(),
            ),
            (
                "Günstiger-Strom-Gesetz zur Reform der Elektrizitätswirtschaft",
                "Dieses umfassende Gesetzpaket, bekannt als 'Günstiger-Strom-Gesetz', reformiert die gesamte österreichische Elektrizitätswirtschaft. Es führt das Elektrizitätswirtschaftsgesetz (ElWG) ein, das den Markt für Strom neu organisiert, um die Ziele der Klimaneutralität bis 2040 und den Ausbau erneuerbarer Energien zu unterstützen. Ein zentraler Pfeiler ist die Stärkung der Endkundinnen und Endkunden durch neue Rechte wie die Wahl zwischen dynamischen und festen Strompreisen, vereinfachte Kündigungsfristen und das Recht auf Ratenzahlung bei Nachzahlungen. Zudem wird ein System zur Bekämpfung von Energiearmut etabliert, das Haushalte mit geringem Einkommen durch gestützte Strompreise entlastet. Das Gesetz regelt zudem detailliert die Pflichten der Netzbetreiber, die Anforderungen an intelligente Messgeräte (Smart Meter) sowie die Organisation des Netzes durch neue Akteure wie Aggregatoren und Bürgerenergiegemeinschaften.",
                NaiveDate::from_ymd_opt(2025, 11, 20).unwrap(),
                NaiveDate::from_ymd_opt(2025, 12, 12).unwrap(),
            ),
            (
                "Novelle zum Parteiengesetz: Ausnahmen, Online‑Kennzeichnung & EP‑Fraktionsausgaben",
                "Der Gesetzentwurf ändert das Parteiengesetz 2012, indem er die Definition von „nahestehenden Organisationen“ präzisiert und bestimmte Gruppen – parlamentarische Klubs, Rechtsträger, Landtagsklubs, landesförderte Bildungseinrichtungen, Fraktionen im Europäischen Parlament sowie internationale und europäische Parteiverbände, deren Tätigkeit nicht primär auf Österreich ausgerichtet ist – ausdrücklich von dieser Definition ausschließt. Zusätzlich werden neue Unterpunkte j, k und l zu § 2 Z 5b eingefügt, die die Bereitstellung von Inhalten durch Regierungsmitarbeiter, Klubmitarbeiter und die Ausgaben für EP‑Fraktionen regeln und dabei Kennzeichnungspflichten sowie Impressum‑Hinweise festlegen. § 4 Abs 6 befreit die Ausgaben der EP‑Fraktionen von den üblichen Beschränkungen und Berichtspflichten, während § 16 Abs 12 das Inkrafttreten zum 1. Juli 2025 bestimmt und Übergangsregelungen für bereits realisierte Fälle enthält.",
                NaiveDate::from_ymd_opt(2025, 6, 18).unwrap(),
                NaiveDate::from_ymd_opt(2025, 7, 11).unwrap(),
            ),
            (
                "UWG-Novelle 2025: Schutz vor Greenwashing und Obsoleszenz",
                "Diese Gesetzesnovelle dient der Umsetzung von EU-Richtlinien zur Stärkung des Verbraucherschutzes im ökologischen Wandel und im digitalen Bereich. Ein zentraler Schwerpunkt liegt auf der Bekämpfung von irreführenden Umweltaussagen (Greenwashing). Künftig müssen allgemeine Aussagen wie 'umweltfreundlich' durch nachgewiesene, hervorragende Umweltleistungen belegt oder präzisiert werden. Zudem werden Nachhaltigkeitssiegel strengeren Regeln unterworfen, indem sie entweder von staatlichen Stellen stammen oder auf transparenten, unabhängigen Zertifizierungssystemen beruhen müssen. Ein weiterer wichtiger Aspekt ist der Schutz vor frühzeitiger Obsoleszenz: Es wird untersagt, Informationen über die begrenzte Haltbarkeit von Waren zurückzuhalten oder Produkte fälschlicherweise als reparierbar darzustellen. Schließlich werden manipulative Online-Methoden (Dark Patterns) im Finanzsektor sowie rechtsmissbräuchliche Abmahnpraktiken gesetzlich unterbunden.",
                NaiveDate::from_ymd_opt(2026, 6, 10).unwrap(),
                NaiveDate::from_ymd_opt(2026, 7, 8).unwrap(),
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

        for (title, desc, start, end) in titles_to_compare {
            dbg!(title, start, end);
            let start = start.checked_sub_days(Days::new(5)).unwrap();
            let end = end.checked_add_days(Days::new(5)).unwrap();

            let res = top_articles_per_provider_reranked(
                &qdrant,
                &somai_client,
                title,
                desc,
                Some((start, end)),
            )
            .await;
            dbg!(&res);

            // for (article, doc_score, score) in scores {
            //     println!(
            //         "\nscore: {}, embed score: {}, doc score: {} \nurl: {}",
            //         score, article.score, doc_score, article.url
            //     );
            // }
        }
    }
}
