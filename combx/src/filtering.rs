use crate::TopicFilter;
use somes_meilisearch_filter::*;

pub fn to_topics_filter(topics: &[TopicFilter], prefix: &str) -> Option<String> {
    if topics.is_empty() {
        return None
    }
    let topics = topics
        .iter()
        .map(|topic| topic.filter_arguments())
        .flatten()
        .collect::<Vec<_>>();
    Some(to_meilisearch_filter_with_ops(
        &topics,
        &FilterOptions {
            combine_op: CombineOp::Or,
            prefix: Some(prefix.into()),
        },
    ))
}
