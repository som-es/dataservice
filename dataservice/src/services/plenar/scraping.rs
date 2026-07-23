use std::{collections::HashSet, time::Duration};

use somes_scraper::{
    error::ScraperError,
    plenar::{Plenar, PlenarInfo},
};

use crate::ApiLimiter;

const MAX_RETRIES: u32 = 3;

pub async fn scrape_new_plenar_infos<S: Plenar + Sync>(
    scraper: &S,
    parliament_api_limiter: ApiLimiter,
) -> crate::Result<Vec<PlenarInfo>> {
    let raw_plenar = parliament_api_limiter
        .run(scraper.post_plenar_payload())
        .await?;
    let plenar_paths = scraper.plenar_paths(raw_plenar)?;
    let mut new_plenar_infos = HashSet::new();

    for plenar_path in &plenar_paths {
        log::info!("plenar path: {plenar_path}");

        let mut attempt = 0;
        let plenar_info = loop {
            match parliament_api_limiter
                .run(scraper.get_plenar(plenar_path))
                .await
            {
                Ok(plenar_info) => break Some(plenar_info),

                Err(ScraperError::Forbidden) => return Err(Box::new(ScraperError::Forbidden)),

                Err(ScraperError::TooManyRequests(retry_after)) if attempt < MAX_RETRIES => {
                    attempt += 1;
                    let wait = retry_after.unwrap_or(Duration::from_secs(4 << attempt));
                    log::warn!(
                        "429 on {plenar_path:?}, waiting {wait:?} (attempt {attempt}/{MAX_RETRIES})",
                    );
                    tokio::time::sleep(wait).await;
                }

                Err(e) => {
                    log::error!("Could not scrape plenar_info {e:?}");
                    break None;
                }
            }
        };

        let Some(plenar_info) = plenar_info else {
            continue;
        };

        new_plenar_infos.insert(plenar_info);
    }

    Ok(new_plenar_infos.into_iter().collect())
}
