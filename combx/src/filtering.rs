use crate::TopicFilter;
use somes_meilisearch_filter::*;

pub fn to_topics_filter(topics: &[TopicFilter], prefix: &str) -> Vec<String> {
    let topics = topics
        .iter()
        .map(|topic| topic.filter_arguments())
        .flatten()
        .collect::<Vec<_>>();
    to_meilisearch_filters(
        &topics,
        &FilterOptions {
            combine_op: CombineOp::Or,
            prefix: Some(prefix.into()),
        },
    )
}
