mod vote_results;
pub use vote_results::*;

mod decrees;
pub use decrees::*;

pub trait FromScrapedType<DbEnhancer> {
    type ScrapedType;
    async fn from_scraped_type(db_enhancer: DbEnhancer, ty: Self::ScrapedType) -> Self;
}
