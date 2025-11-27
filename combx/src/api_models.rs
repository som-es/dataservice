pub mod vote_results;
pub use vote_results::*;

pub mod decrees;
pub use decrees::*;

pub mod gov_proposals;
pub use gov_proposals::*;

pub trait FromScrapedType<DbEnhancer> {
    type ScrapedType;
    fn from_scraped_type(
        db_enhancer: DbEnhancer,
        ty: Self::ScrapedType,
    ) -> impl Future<Output = Self>;
}
