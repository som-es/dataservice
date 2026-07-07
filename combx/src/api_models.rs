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

pub mod speech_ai_summary;
pub use speech_ai_summary::*;

pub trait FromScrapedType<DbEnhancer> {
    type ScrapedType;
    fn from_scraped_type(
        db_enhancer: DbEnhancer,
        ty: Self::ScrapedType,
    ) -> impl Future<Output = Self>;
}
