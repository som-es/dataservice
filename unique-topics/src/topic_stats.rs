use std::collections::HashSet;

use combx::connect_pg;
use eurovoc::{Eurovoc, FileType};
use unique_topics::all_unique_eurovoc_topics;
mod unique_topics;

#[tokio::main]
async fn main() -> combx::Result<()> {
    let eurovoc = Eurovoc::new(None, None).unwrap();
    eurovoc.downloader().ensure_data().await.unwrap();
    let pg = connect_pg().await;

    let unique_eurovoc_topics = all_unique_eurovoc_topics(&pg).await?;

    let eurovoc_topics = eurovoc
        .get_topics(FileType::Thesaurus, &["de"])
        .await
        .unwrap()
        .into_iter()
        .map(|topic| (FileType::Thesaurus, topic))
        .chain(
            eurovoc
                .get_topics(FileType::Descriptors, &["de"])
                .await
                .unwrap()
                .into_iter()
                .map(|topic| (FileType::Descriptors, topic)),
        )
        .chain(
            eurovoc
                .get_topics(FileType::Domain, &["de"])
                .await
                .unwrap()
                .into_iter()
                .map(|topic| (FileType::Domain, topic)),
        )
        .collect::<Vec<_>>();

    let mut used_eurovoc_topics_by_id = HashSet::new();

    let mut unmatchable = HashSet::new();

    'outer: for unique_topic in unique_eurovoc_topics.iter() {
        for (ftype, topic) in &eurovoc_topics {
            if unique_topic.to_lowercase() == topic.label.to_lowercase() {
                used_eurovoc_topics_by_id.insert((ftype, topic.clone()));
                continue 'outer;
            }
        }
        unmatchable.insert(unique_topic);
    }

    let text = serde_json::to_string_pretty(&used_eurovoc_topics_by_id).unwrap();
    tokio::fs::write("./id_unique_eurovoc_topics.json", &text)
        .await
        .unwrap();

    dbg!(unmatchable);

    Ok(())
}
