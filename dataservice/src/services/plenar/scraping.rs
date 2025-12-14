use somes_scraper::{AustriaScraper, plenar::{Plenar, PlenarInfo}};

pub async fn scrape_new_plenar_infos(scraper: &AustriaScraper) -> crate::Result<Vec<PlenarInfo>> {
    let raw_plenar = scraper.post_plenar_payload().await?;
    let plenar_paths = scraper.plenar_paths(raw_plenar)?;

    let mut new_plenar_infos = Vec::new();

    for plenar_path in &plenar_paths {
        let Ok(plenar_info) = scraper.get_plenar(&plenar_path).await else {
            log::debug!("Cannot process plenar info: '{plenar_path}'");
            continue;
        };

        new_plenar_infos.push(plenar_info);
    }

    Ok(new_plenar_infos)
}