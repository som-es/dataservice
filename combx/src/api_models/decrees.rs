use chrono::{DateTime, NaiveDate, Utc};
use optional_struct::*;
use serde::{Deserialize, Serialize};
use somes_common_lib::Document;
use somes_macro::MeilisearchFilter;
use somes_meilisearch_filter::{FilterArgument, FilterOp};
use somes_scraper::Thing;
use sqlx::FromRow;
use std::sync::Arc;

use crate::{
    FromScrapedType, api_models::decrees::enhancer::DecreeEnhancer,
    with_data::gps::LegislativePeriod,
};

mod enhancer;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[optional_struct]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, MeilisearchFilter)]
pub struct Decree {
    #[filter(make_vec)]
    pub gov_official_id: Option<i32>,
    #[filter(make_optional, make_vec)]
    pub ris_id: String,
    #[filter(make_optional, make_vec)]
    pub ministrial_issuer: String,
    #[filter(make_optional)]
    pub title: String,
    #[filter(make_optional)]
    pub short_title: String,
    #[filter(make_optional)]
    pub publication_date: NaiveDate,
    #[filter(make_optional)]
    pub part: String,
    pub emphasis: Option<String>,
    #[filter(make_vec)]
    pub gp: Option<String>,
    #[filter(make_optional)]
    pub eli: String,
    #[filter(make_optional)]
    pub document_url: String,
    #[filter(skip)]
    pub documents: Vec<Document>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
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
            eli: decree.eli,
            document_url: decree.document_url,
            emphasis: None,
            gp: possible_gp.cloned(),
            documents: decree
                .documents
                .into_iter()
                .map(|doc| Document {
                    title: Some(doc.title),
                    document_url: Some(doc.link),
                    document_type: Some(doc.ftype.to_string()),
                })
                .collect(),
            created_at: None,
            updated_at: None,
        }
    }
}
