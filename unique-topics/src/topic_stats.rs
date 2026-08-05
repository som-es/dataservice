use std::collections::HashSet;

use combx::connect_pg;
use eurovoc::Eurovoc;
use unique_topics::{all_unique_eurovoc_topics, hallucinated_ai_topics_by_model};
mod unique_topics;

#[tokio::main]
async fn main() -> combx::Result<()> {
    let eurovoc = Eurovoc::new(None, None).unwrap();
    eurovoc.downloader().ensure_data().unwrap();
    let pg = connect_pg().await;

    let unique_eurovoc_topics = all_unique_eurovoc_topics(&pg).await?;

    let eurovoc_topics = eurovoc.get_topics(&["de"]).unwrap();

    let mut used_eurovoc_topics_by_id = HashSet::new();

    'outer: for unique_topic in unique_eurovoc_topics.iter() {
        for topic in &eurovoc_topics {
            if unique_topic == &topic.label {
                used_eurovoc_topics_by_id.insert(topic.clone());
                continue 'outer;
            }
        }
        panic!("could not find id for topic: {unique_topic}");
    }

    let text = serde_json::to_string_pretty(&used_eurovoc_topics_by_id).unwrap();
    tokio::fs::write("./unique_eurovoc_topics", &text).await;

    Ok(())
}
