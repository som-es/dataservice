use std::collections::HashMap;

use somes_scraper::{AustriaScraper, GovOfficial, gov_officials::GovOfficials};

pub async fn create_gov_officials_map() -> HashMap<u64, Vec<GovOfficial>> {
    let scraper = AustriaScraper::default();
    let gov_officials_listed = scraper.gov_officials().await;

    let gov_officials_listed = gov_officials_listed.unwrap_or_default();

    let mut gov_officials = HashMap::new();
    for gov_official in gov_officials_listed {
        gov_officials
            .entry(gov_official.delegate_id)
            .and_modify(|e: &mut Vec<GovOfficial>| e.push(gov_official.clone()))
            .or_insert(vec![gov_official]);
    }

    gov_officials
}
