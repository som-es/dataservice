use chrono::NaiveDateTime;
use std::sync::LazyLock;

pub(super) struct Url {
    pub url: String,
    pub updated_at: NaiveDateTime,
}

pub(super) static URLS: LazyLock<Vec<Url>> = LazyLock::new(|| {
    vec![
    Url {
        url: "https://www.derstandard.at/story/3000000314420/brandanschlag-auf-russisches-kulturzentrum-in-prag".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-27T09:16:15.780", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/jetzt/livebericht/3000000314193/woeginger-prozess-heute-mit-zeugen-zu-flurfunk-ueber-postenschacher".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-27T09:12:31.503", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000314415/orf-turbulenzen-thurnher-richtet-transparenzbeirat-ein".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-27T09:07:28.750", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000314410/nahost-konflikt-schiebt-laut-umfrage-interesse-an-e-autos-an-umfrage".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-27T09:04:57.947", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000314110/drei-gute-minuten-corgi-als-hunderudelfuehrer-und-restituierte-benin-bronzen".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-27T09:00:00.000", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000314346/was-brauchen-gute-fuehrungskraefte-eurer-meinung-nach".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-27T09:00:00.000", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000314405/zehn-jahre-ohne-kollektivvertrag-italienische-journalisten-streiken".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-27T08:57:09.032", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000314385/ki-zusammenfassungen".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-27T08:56:55.110", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000314403/formel-1-mercedes-duo-im-suzuka-training-geschlagen-piastri-erster".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-27T08:54:53.696", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000314384/nach-rechtsstreit-mit-dem-vater-seit-suizidversuch-gelaehmte-spanierin-durfte-sterben".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-27T08:52:32.036", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000314398/ukraine-russland-besch228digte-sehensw252rdigkeiten-der-kk-monarchie".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-27T08:48:09.679", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000314404/gericht-in-amsterdam-verurteilt-grok-entwickler-wegen-ki-nacktbildern".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-27T08:47:08.957", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000314290/investor-danny-k-ist-unzufrieden-mit-seinem-prozess-niemand-kennt-sich-aus-hier".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-27T08:47:00.000", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000314394/bolivien-und-jamaika-in-wm-play-offs-eine-runde-weiter".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-27T08:38:48.058", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000314387/was-das-spritpreis-dilemma-mit-der-siedlungspolitik-zu-tun-hat".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-27T08:29:07.518", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000314221/die-spritpreisbremse-ist-gut-gemeint-aber-nicht-gut-gemacht".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-27T08:22:10.120", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000314189/risiko-besser-managen-wie-das-mit-captives-geht".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-27T08:00:00.000", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000309737/wissen-wo-sich-wasser-bei-starkregen-seinen-weg-bahnt".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-27T08:00:00.000", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000314344/freunde-mit-problematischen-ansichten-wie-geht-ihr-damit-um".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-27T08:00:00.000", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000312063/oesterreich-das-land-der-allzu-vielen-unis".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-27T08:00:00.000", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000314197/us-army-bekam-ersten-autonom-fliegenden-black-hawk-geliefert".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-27T08:00:00.000", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000314383/khm-causa-betriebsrat-fordert-transparente-und-l252ckenlose-aufkl228rung".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-27T07:52:30.630", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000314382/an-invisible-bottleneck-a-helium-shortage-threatens-the-chip-industry".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-27T07:51:06.752", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000314379/oscar-verleihung-zieht-von-hollywood-nach-la-downtown".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-27T07:43:59.422", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000314376/tatort-abschied-fuer-muenchner-kommissare-franz-leitmayr-und-ivo-batic".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-27T07:29:44.719", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000314375/disney-serie-vienna-game-ueber-wiener-kongress-startet-im-herbst".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-27T07:21:35.956", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000314368/verbund-chef-strugl-ueber-oekostrom-ausbau-wir-brauchen-immer-krisen".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-27T07:07:12.558", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000314026/ausstellung-in-moskau-zeigt-krieg-als-eine-art-abenteuer-urlaub-fuer-ganze-kerle".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-27T07:05:13.852", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000314183/wie-josef-winkler-im-neuen-buch-den-schatten-der-schwester-beschwoert".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-27T07:00:00.000", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000313479/das-psycho-rezept-die-entstehung-psychischer-erkrankungen".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-27T07:00:00.000", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000311221/relax-magazin-ein-wellnessheft-zwischen-spa-und-rechter-ideologie".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-27T07:00:00.000", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000314373/olympia-trump-gratuliert-ioc-zu-olympia-bann-f252r-transfrauen".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-27T06:46:54.577", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000314372/gericht-stoppt-vorerst-pentagon-sperre-gegen-ki-firma-anthropic".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-27T06:33:27.752", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000314361/wie-die-naechste-oelkrise-oesterreich-kaltlaesst".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-27T06:23:00.000", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000314364/preise-f252r-haushaltsenergie-im-februar-im-jahresvergleich-gesunken".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-27T06:05:51.006", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000314317/tanktouristen-aus-oesterreich-slowenien-rationiert-treibstoffe".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-27T06:05:14.284", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000314214/sprint-statt-flaute-wie-segeln-um-seine-olympische-zukunft-kaempft".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-27T06:02:09.016", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000314367/meta-investiert-zehn-milliarden-dollar-in-ki-rechenzentrum-in-texas".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-27T05:59:04.741", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000314366/chinesischer-chipbauer-smic-beliefert-irans-militaer".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-27T05:55:50.766", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000314365/musks-x-scheitert-mit-klage-gegen-abtruennige-werbekunden".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-27T05:47:34.757", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000314363/segelboote-mit-hilfsg252tern-f252r-kuba-vermisst".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-27T05:43:52.036", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000314362/eu-f252hrt-bearbeitungsgeb252hr-f252r-kleinpakete-aus-drittstaaten-ein".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-27T05:23:03.272", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000313934/arbeitsklima-kaputt-durch-einen-kollegen-was-wuerdet-ihr-tun".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-27T05:00:00.000", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000313671/der-irankrieg-zeigt-ein-fuer-alle-mal-dass-trump-nicht-agent-krasnow-ist".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-27T05:00:00.000", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000312822/beatles-girl-on-the-train-weltweit-tv-tipps-fuer-freitag".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-27T05:00:00.000", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000313881/arbeitsmediziner-der-wecker-ist-ein-gesundheitsrisiko".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-27T04:00:00.000", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000314236/insidertrading-polymarket-user-schloss-verdaechtig-genaue-wetten-auf-iran-krieg-ab".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-27T04:00:00.000", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000314160/usa-wollen-anscheinend-drei-weitere-gebiete-in-groenland".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-27T04:00:00.000", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000314022/rangnicks-reisegruppe-teil-drei-wer-steht-im-tor-wer-schiesst-die-tore".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-27T04:00:00.000", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000314172/so-versucht-die-nasa-ihr-mondlandeprogramm-zu-retten".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-27T04:00:00.000", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000314106/neue-regeln-fuer-e-scooter-e-bikes-und-e-mopes-wissen-sie-was-kuenftig-gilt".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-27T04:00:00.000", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000312667/standardraetsel-i-11251".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-26T23:01:00.000", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000314359/mit-20-ueber-nordirland-nimmt-italien-die-erste-playoff-huerde-die-naechste-ist-hoeher".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-26T22:50:29.177", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/jetzt/livebericht/3000000314127/israsechs-tote-nach-israelischen-angriffen-im-libanon-gemeldet".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-26T20:29:30.037", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000314188/buckelwal-steckt-an-deutscher-ostseekueste-fest-rettung-geht-weiter".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-26T20:23:26.703", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000314281/digitale-schmeichler-chatgpt-claude-gemini-und-co-sind-gefaehrlich-unterwuerfig".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-26T19:42:00.000", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000314355/pakistan-nimmt-milit228roperationen-gegen-afghanistan-wieder-auf".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-26T19:25:15.881", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000314204/melanie-trump-wirbt-fuer-homeschooling-mit-humanoidem-roboter".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-26T19:07:35.145", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/jetzt/livebericht/3000000314126/ukraine-vier-tote-nach-russischen-angriffen".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-26T18:54:36.182", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000309878/zeitgeistig-im-gegenstromland".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-26T18:51:00.000", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000314331/fu223ball-214fb-team-startet-mit-test-gegen-ghana-ins-wm-jahr".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-26T18:30:00.000", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000314352/70-song-contest-zus228tzliche-sicherheitsherausforderungen".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-26T18:28:04.421", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000314138/maduro-nach-gefangennahme-zum-zweiten-mal-vor-us-gericht".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-26T18:02:22.511", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
    Url {
        url: "https://www.derstandard.at/story/3000000314259/sind-die-louvre-juwelen-noch-in-paris-polizei-vermutet-sie-in-einem-vorort".into(),
        updated_at: NaiveDateTime::parse_from_str("2026-03-26T17:43:00.000", "%Y-%m-%dT%H:%M:%S%.f").unwrap(),
    },
]
});
