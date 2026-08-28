use somes_meilisearch_filter::*;

use crate::{
    OptionalGovProposalFilter, filtering::to_topics_filter, meilisearch_filters_ai_summary,
    meilisearch_filters_vote_result,
};

pub fn meilisearch_filters_gov_props(
    gov_prop_filter: OptionalGovProposalFilter,
    prefix: Option<String>,
) -> Vec<String> {
    let mut filter_conditions = vec![];
    let prefix = prefix.map(|prefix| prefix.to_string());

    let top_level_filters = to_meilisearch_filters(
        &gov_prop_filter.filter_arguments(),
        &FilterOptions {
            prefix: prefix.clone(),
            ..Default::default()
        },
    );
    filter_conditions.extend(top_level_filters);

    if let Some(ministrial_proposal) = gov_prop_filter.ministrial_proposal {
        filter_conditions.extend(to_meilisearch_filters(
            &ministrial_proposal.filter_arguments(),
            &FilterOptions {
                prefix: Some(build_prefix("ministrial_proposal", prefix.as_deref())),
                ..Default::default()
            },
        ));
    }

    if let Some(vote_result_filter) = gov_prop_filter.vote_result {
        filter_conditions.extend(meilisearch_filters_vote_result(
            vote_result_filter,
            Some(build_prefix("vote_result", prefix.as_deref())),
        ));
    }

    filter_conditions.extend(to_topics_filter(
        gov_prop_filter
            .eurovoc_topics
            .unwrap_or_default()
            .as_slice(),
        &build_prefix("eurovoc_topics", prefix.as_deref()),
    ));

    filter_conditions.extend(to_topics_filter(
        gov_prop_filter
            .other_keyword_topics
            .unwrap_or_default()
            .as_slice(),
        &build_prefix("other_keyword_topics", prefix.as_deref()),
    ));

    filter_conditions.extend(to_topics_filter(
        gov_prop_filter.topics.unwrap_or_default().as_slice(),
        &build_prefix("topics", prefix.as_deref()),
    ));

    filter_conditions.extend(meilisearch_filters_ai_summary(
        gov_prop_filter.ai_summary,
        Some("gov_proposal.ai_summary".into()),
    ));

    filter_conditions
}
