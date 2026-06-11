use std::collections::HashSet;

use somes_scraper::{
    plenar::{Plenar, PlenarInfo},
    AustriaScraper,
};

use crate::ApiLimiter;

pub async fn scrape_new_plenar_infos(
    scraper: &AustriaScraper,
    parliament_api_limiter: ApiLimiter,
) -> crate::Result<Vec<PlenarInfo>> {
    let raw_plenar = parliament_api_limiter
        .run(scraper.post_plenar_payload())
        .await?;
    let plenar_paths = scraper.plenar_paths(raw_plenar)?;
    let mut new_plenar_infos = HashSet::new();

    for plenar_path in &plenar_paths {
        let Ok(plenar_info) = parliament_api_limiter
            .run(scraper.get_plenar(&plenar_path))
            .await
        else {
            log::debug!("Cannot process plenar info: '{plenar_path}'");
            continue;
        };

        new_plenar_infos.insert(plenar_info);
    }

    Ok(new_plenar_infos.into_iter().collect())
}
