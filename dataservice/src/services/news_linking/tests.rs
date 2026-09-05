use somai_client::SomAiClient;

use crate::news_linking::tests::urls::URLS;

mod urls;

#[tokio::test]
async fn test_embedding_comparison() {
    let titles_to_compare = vec![
        "Kurzfristige Margenbegrenzung bei Kraftstoffpreisen",
        "Günstiger-Strom-Gesetz zur Reform der Elektrizitätswirtschaft",
    ];

    let _ = simple_logger::init_with_level(log::Level::Info);
    let somai_client = SomAiClient::new("http://10.30.0.164:8000", "API_USER", "supersicher123!")
        .await
        .unwrap();

    for (idx, title) in titles_to_compare.iter().enumerate() {
        let mut sims = vec![];
        for url in &*URLS {
            let sim = somai_client
                .embedding_embed_and_cosine(title.to_string(), url.url.clone())
                .await
                .unwrap();
            sims.push((url.url.clone(), sim.cosine_similarity));
        }
        sims.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        if idx == 0 {
            assert_eq!(
                sims.last().unwrap().0,
                "https://www.derstandard.at/story/3000000314221/die-spritpreisbremse-ist-gut-gemeint-aber-nicht-gut-gemacht"
            );
        }

        if idx == 1 {
            assert_eq!(
                sims.last().unwrap().0,
                "https://www.derstandard.at/story/3000000314364/preise-f252r-haushaltsenergie-im-februar-im-jahresvergleich-gesunken"
            );
        }
        dbg!(title, sims);
    }
}
