pub mod vote_results;
pub use vote_results::*;

pub mod delegates;
pub use delegates::*;

pub mod decrees;
pub use decrees::*;

pub mod gov_proposals;
pub use gov_proposals::*;

pub mod ai_summary;
pub use ai_summary::*;

pub mod parliamentary_qa;
pub use parliamentary_qa::*;

pub mod speeches;
pub use speeches::*;

pub mod mood;
pub use mood::*;

pub trait FromScrapedType<DbEnhancer> {
    type ScrapedType;
    fn from_scraped_type(
        db_enhancer: DbEnhancer,
        ty: Self::ScrapedType,
    ) -> impl Future<Output = Self>;
}
