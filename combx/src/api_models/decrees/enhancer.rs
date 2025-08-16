use somes_scraper::Thing;

use crate::{Enhancer, api_models::decrees::enhancer::postgres::GovOfficialAtDate};

mod postgres;

pub trait DecreeEnhancer: Enhancer {
    fn gov_officials_at_date(
        &self,
        decree: &Thing,
    ) -> impl Future<Output = Result<Vec<GovOfficialAtDate>, Self::Error>>;
}
