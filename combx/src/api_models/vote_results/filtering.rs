use somes_meilisearch_filter::*;

use crate::{
    OptionalVoteResultFilter, filtering::to_topics_filter, meilisearch_filters_ai_summary,
};

pub fn meilisearch_filters_vote_result(
    vote_result_filter: OptionalVoteResultFilter,
    prefix: Option<String>,
) -> Vec<String> {
    let mut filter_conditions = vec![];
    let legis_init_filters = to_meilisearch_filters(
        &vote_result_filter
            .legislative_initiative
            .as_ref()
            .map(|x| x.filter_arguments())
            .unwrap_or_default(),
        &FilterOptions {
            prefix: Some(build_prefix("legislative_initiative", prefix.as_deref())),
            ..Default::default()
        },
    );
    filter_conditions.extend(legis_init_filters);
    let top_level_filters = to_meilisearch_filters(
        &vote_result_filter.filter_arguments(),
        &FilterOptions {
            prefix: prefix.clone(),
            ..Default::default()
        },
    );
    filter_conditions.extend(top_level_filters);

    let topics_filter = to_topics_filter(
        vote_result_filter
            .eurovoc_topics
            .unwrap_or_default()
            .as_slice(),
        &build_prefix("eurovoc_topics", prefix.as_deref()),
    );
    filter_conditions.extend(topics_filter);

    filter_conditions.extend(to_topics_filter(
        vote_result_filter
            .other_keyword_topics
            .unwrap_or_default()
            .as_slice(),
        &build_prefix("other_keyword_topics", prefix.as_deref()),
    ));

    filter_conditions.extend(to_topics_filter(
        vote_result_filter.topics.unwrap_or_default().as_slice(),
        &build_prefix("topics", prefix.as_deref()),
    ));

    filter_conditions.extend(meilisearch_filters_ai_summary(
        vote_result_filter.ai_summary,
        Some("ai_summary".into()),
    ));

    filter_conditions.extend(to_meilisearch_filters(
        &vote_result_filter
            .issued_by_dels
            .unwrap_or_default()
            .iter()
            .map(|issued_by_del| issued_by_del.filter_arguments())
            .flatten()
            .collect::<Vec<_>>(),
        &FilterOptions {
            prefix: Some(build_prefix("issued_by_dels", prefix.as_deref())),
            combine_op: CombineOp::Or,
        },
    ));

    filter_conditions
}
