use somes_meilisearch_filter::*;

use crate::{OptionalGovProposalFilter, filtering::to_topics_filter, meilisearch_filters_vote_result};

pub fn meilisearch_filters_gov_props(gov_prop_filter: OptionalGovProposalFilter) -> Vec<String> {
    let mut filter_conditions = vec![];
    
    let top_level_filters = to_meilisearch_filters(
        &gov_prop_filter.filter_arguments(),
        &FilterOptions::default(),
    );
    filter_conditions.extend(top_level_filters);
    
    if let Some(vote_result_filter) = gov_prop_filter.vote_result {
        filter_conditions.extend(meilisearch_filters_vote_result(vote_result_filter));
    }

    filter_conditions.extend(to_topics_filter(
        gov_prop_filter
            .eurovoc_topics
            .unwrap_or_default()
            .as_slice(),
        "eurovoc_topics",
    ));

    filter_conditions.extend(to_topics_filter(
        gov_prop_filter
            .other_keyword_topics
            .unwrap_or_default()
            .as_slice(),
        "other_keyword_topics",
    ));

    filter_conditions.extend(to_topics_filter(
        gov_prop_filter.topics.unwrap_or_default().as_slice(),
        "topics",
    ));

    filter_conditions.extend(to_meilisearch_filters(
        &gov_prop_filter
            .ai_summary
            .as_ref()
            .map(|ai_summary| ai_summary.filter_arguments())
            .unwrap_or_default(),
        &FilterOptions {
            prefix: Some("ai_summary".into()),
            ..Default::default()
        },
    ));

    filter_conditions
}