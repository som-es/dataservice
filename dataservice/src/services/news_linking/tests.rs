use somai_client::SomAiClient;

use crate::news_linking::{tests::urls::URLS, upload_and_embed::extract_relevant_parts_from_loc};

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

#[test]
fn test_url_relevant_part_extraction() {
    let loc = "https://www.heute.at/s/zecke-gefunden-darum-sollst-du-sie-einsenden-120213902";
    let res = extract_relevant_parts_from_loc(loc);
    assert_eq!(
        res,
        "zecke gefunden darum sollst du sie einsenden 120213902"
    );

    let loc = "https://www.profil.at/wirtschaft/oebag-vorstand-ausschreibung-hlawati-ofner-hattmannsdorfer/403186836";
    let res = extract_relevant_parts_from_loc(loc);
    assert_eq!(
        res,
        "oebag vorstand ausschreibung hlawati ofner hattmannsdorfer"
    );

    let loc = "https://www.derstandard.at/story/3000000338370/niederlage-fuer-trump-gericht-in-missouri-kippt-neuzuschnitt-der-wahlkreise";

    let res = extract_relevant_parts_from_loc(loc);
    assert_eq!(
        res,
        "niederlage fuer trump gericht in missouri kippt neuzuschnitt der wahlkreise"
    );

    let loc = "https://kurier.at/politik/ausland/kolumbien-erdbeben-trump-aussetzung-der-zoelle/403184220";

    let res = extract_relevant_parts_from_loc(loc);
    assert_eq!(res, "kolumbien erdbeben trump aussetzung der zoelle");
}
