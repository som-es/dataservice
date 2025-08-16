use std::sync::Arc;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use somes_scraper::Thing;
use sqlx::FromRow;

use crate::{
    FromScrapedType, api_models::decrees::enhancer::DecreeEnhancer,
    with_data::gps::LegislativePeriod,
};

mod enhancer;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Document {
    pub title: String,
    pub document_url: String,
    pub document_type: String,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Decree {
    pub gov_official_id: Option<i32>,
    pub ris_id: String,
    pub ministrial_issuer: String,
    pub title: String,
    pub short_title: String,
    pub publication_date: NaiveDate,
    pub part: String,
    pub emphasis: Option<String>,
    pub gp: Option<String>,
    pub documents: Vec<Document>,
}

impl<T: DecreeEnhancer> FromScrapedType<T> for Decree {
    type ScrapedType = (Thing, Arc<[LegislativePeriod]>);

    async fn from_scraped_type(db_enhancer: T, (thing, all_gps): Self::ScrapedType) -> Self {
        Decree::from_thing(db_enhancer, thing, all_gps).await
    }
}

impl Decree {
    pub async fn from_thing(
        db_enhancer: impl DecreeEnhancer,
        decree: Thing,
        all_gps: Arc<[LegislativePeriod]>,
    ) -> Self {
        let possible_gp = all_gps.windows(2).find_map(|gps| {
            let end_date = gps[0].start_date;
            let start_date = gps[1].start_date;
            if decree.date <= end_date && decree.date >= start_date {
                Some(&gps[1].gp)
            } else {
                None
            }
        });

        let gov_officials_at_date = db_enhancer
            .gov_officials_at_date(&decree)
            .await
            .unwrap_or_default();

        let gov_official_id = gov_officials_at_date
            .iter()
            .find(|x| x.ministry.as_ref() == Some(&decree.issuer))
            .map(|x| x.delegate_id);

        Decree {
            gov_official_id,
            ris_id: decree.id,
            ministrial_issuer: decree.issuer,
            title: decree.title,
            short_title: decree.short_title,
            publication_date: decree.date,
            part: decree.part,
            emphasis: None,
            gp: possible_gp.cloned(),
            documents: decree
                .documents
                .into_iter()
                .map(|doc| Document {
                    title: doc.title,
                    document_url: doc.link,
                    document_type: doc.ftype.to_string(),
                })
                .collect(),
        }
    }
}
