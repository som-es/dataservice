use std::collections::HashSet;

use clap::Parser;
use combx::connect_pg;
use eurovoc::{Eurovoc, FileType};
use unique_topics::{TopicSource, all_used_topics, all_used_topics_combined};
mod unique_topics;

#[derive(Parser, Debug)]
#[command(about = "Match database topics against Eurovoc file types")]
struct Args {
    /// Which database topic table to match against the Eurovoc file types
    #[arg(long, value_enum, default_value_t = TopicSource::Eurovoc)]
    topic_source: TopicSource,
    /// Collect topics from all sources into one combined output file (ignores --topic-source)
    #[arg(long)]
    combine: bool,
}

#[tokio::main]
async fn main() -> combx::Result<()> {
    let args = Args::parse();
    let eurovoc = Eurovoc::new(None, None).unwrap();
    eurovoc.downloader().ensure_data().await.unwrap();
    let pg = connect_pg().await;

    let unique_eurovoc_topics = if args.combine {
        all_used_topics_combined(&pg).await?
    } else {
        all_used_topics(&pg, args.topic_source).await?
    };

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
        .chain(
            eurovoc
                .get_topics(FileType::Custom, &["de"])
                .await
                .unwrap()
                .into_iter()
                .map(|topic| (FileType::Custom, topic)),
        )
        .collect::<Vec<_>>();

    let mut used_eurovoc_topics_by_id = HashSet::new();

    let mut unmatchable = HashSet::new();

    'outer: for unique_topic in unique_eurovoc_topics.iter() {
        for (ftype, topic) in &eurovoc_topics {
            if unique_topic.to_lowercase() == topic.label.to_lowercase() {
                let mut topic = topic.clone();
                topic.label = unique_topic.clone();
                used_eurovoc_topics_by_id.insert((ftype, topic));
                continue 'outer;
            }
        }
        unmatchable.insert(unique_topic);
    }

    let text = serde_json::to_string_pretty(&used_eurovoc_topics_by_id).unwrap();
    let file_name = if args.combine {
        "./id_combined_topics.json"
    } else {
        match args.topic_source {
            TopicSource::Eurovoc => "./id_unique_eurovoc_topics.json",
            TopicSource::LegisInit => "./id_unique_legis_init_topics.json",
        }
    };
    tokio::fs::write(file_name, &text).await.unwrap();

    println!(
        "Matched {matched}/{total} {source} topics to Eurovoc file types",
        matched = used_eurovoc_topics_by_id.len(),
        total = unique_eurovoc_topics.len(),
        source = if args.combine {
            "combined"
        } else {
            args.topic_source.label()
        },
    );
    dbg!(unmatchable);

    Ok(())
}
