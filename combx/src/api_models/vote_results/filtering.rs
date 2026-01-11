use somes_meilisearch_filter::*;

use crate::{OptionalVoteResultFilter, filtering::to_topics_filter};

pub fn meilisearch_filters_vote_result(vote_result_filter: OptionalVoteResultFilter) -> Vec<String> {
    let mut filter_conditions = vec![];
    let legis_init_filters = to_meilisearch_filters(
        &vote_result_filter
            .legislative_initiative
            .as_ref()
            .map(|x| x.filter_arguments())
            .unwrap_or_default(),
        &FilterOptions {
            prefix: Some("legislative_initiative".to_string()),
            ..Default::default()
        },
    );
    filter_conditions.extend(legis_init_filters);
    let top_level_filters = to_meilisearch_filters(
        &vote_result_filter.filter_arguments(),
        &FilterOptions::default(),
    );
    filter_conditions.extend(top_level_filters);

    filter_conditions.extend(to_topics_filter(
        vote_result_filter
            .eurovoc_topics
            .unwrap_or_default()
            .as_slice(),
        "eurovoc_topics",
    ));

    filter_conditions.extend(to_topics_filter(
        vote_result_filter
            .other_keyword_topics
            .unwrap_or_default()
            .as_slice(),
        "other_keyword_topics",
    ));

    filter_conditions.extend(to_topics_filter(
        vote_result_filter.topics.unwrap_or_default().as_slice(),
        "topics",
    ));

    filter_conditions.extend(to_meilisearch_filters(
        &vote_result_filter
            .ai_summary
            .as_ref()
            .map(|ai_summary| ai_summary.filter_arguments())
            .unwrap_or_default(),
        &FilterOptions {
            prefix: Some("ai_summary".into()),
            ..Default::default()
        },
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
            prefix: Some("issued_by_dels".into()),
            combine_op: CombineOp::Or,
        },
    ));

    filter_conditions
}