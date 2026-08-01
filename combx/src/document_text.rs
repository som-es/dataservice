use std::collections::{HashMap, HashSet};

#[cfg(not(feature = "tesseract"))]
use ocrs::OcrEngine;
use somes_common_lib::Document;
use somes_scraper::{TextExtractor, http_client};

use crate::{extract_images_from_pdf, pdf};

#[cfg(not(feature = "tesseract"))]
use crate::image_to_text;
#[cfg(not(feature = "tesseract"))]
use std::sync::Arc;

#[derive(Debug)]
pub struct TextDocument {
    pub document: Document,
    pub text: String,
}

pub fn combine_texts(documents: &[TextDocument]) -> String {
    documents
        .iter()
        .map(|doc| {
            let important_marker = if doc.document.title == Some("Gesetzestext".to_string()) {
                "analysiere diesen Gesetzestext besonders sorgfältig und ausführlich."
            } else {
                ""
            };

            format!(
                "{} - {important_marker}\n {}",
                doc.document.title.as_deref().unwrap_or(""),
                doc.text
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

const TOTAL_PROMPT_LENGTH: usize = 393000;

pub async fn extract_text_from_documents_html_first_stripped(
    documents: &[Document],
    #[cfg(feature = "tesseract")] ocr_engine: Option<tesseract_rs::TesseractAPI>,
    #[cfg(not(feature = "tesseract"))] ocr_engine: Option<Arc<OcrEngine>>,
) -> Vec<TextDocument> {
    let mut results = extract_text_from_documents_html_first(documents.iter(), ocr_engine).await;
    // move document called "Gesetzestext" to the front if exists
    if let Some(pos) = results.iter().position(|doc| {
        doc.document
            .title
            .as_deref()
            .map(|x| x.contains("Gesetzestext"))
            .unwrap_or_default()
    }) {
        let doc = results.remove(pos);
        results.insert(0, doc);
    }

    let mut acc_text_length = 0;
    results
        .into_iter()
        .filter(|doc| {
            let is_prompt_full = acc_text_length <= TOTAL_PROMPT_LENGTH;
            acc_text_length += doc.text.len();
            is_prompt_full
        })
        .collect()
}

pub async fn extract_text_from_documents_html_first<
    'a,
    I: std::iter::Iterator<Item = &'a Document>,
>(
    documents: I,
    #[cfg(feature = "tesseract")] ocr_engine: Option<tesseract_rs::TesseractAPI>,
    #[cfg(not(feature = "tesseract"))] ocr_engine: Option<Arc<OcrEngine>>,
) -> Vec<TextDocument> {
    let mut results = vec![];
    let mut grouped_documents = HashMap::<String, Vec<Document>>::new();
    for document in documents {
        grouped_documents
            .entry(document.title.clone().unwrap_or_default())
            .and_modify(|e| e.push(document.clone()))
            .or_insert(vec![document.clone()]);
    }
    for (_title, docs) in grouped_documents {
        let maybe_html = docs
            .iter()
            .find(|x| x.document_type.as_deref() == Some("HTML"));
        let mut try_extract_digital_pdf = false;
        match maybe_html {
            Some(html_doc) => {
                if let Ok(text) = extract_text_from_document(html_doc, ocr_engine.clone()).await {
                    results.push(TextDocument {
                        document: html_doc.clone(),
                        text,
                    });
                } else {
                    try_extract_digital_pdf = true;
                }
            }
            None => {
                try_extract_digital_pdf = true;
            }
        }

        let mut try_extract_original_pdf = false;
        if try_extract_digital_pdf {
            let maybe_pdf = docs
                .iter()
                .find(|x| x.document_type.as_deref() == Some("DigitalPDF"));
            if let Some(pdf_doc) = maybe_pdf {
                if let Ok(text) = extract_text_from_document(pdf_doc, ocr_engine.clone()).await {
                    results.push(TextDocument {
                        document: pdf_doc.clone(),
                        text,
                    });
                } else {
                    try_extract_original_pdf = true;
                }
            } else {
                try_extract_original_pdf = true;
            }
        }

        if try_extract_original_pdf {
            let maybe_orig_pdf = docs
                .iter()
                .find(|x| x.document_type.as_deref() == Some("OriginalPDF"));
            if let Some(orig_pdf_doc) = maybe_orig_pdf {
                if let Ok(text) = extract_text_from_document(orig_pdf_doc, ocr_engine.clone()).await
                {
                    // println!("Extracted text from OriginalPDF document: {}", &text[..800.min(text.len())]);
                    results.push(TextDocument {
                        document: orig_pdf_doc.clone(),
                        text,
                    });
                }
            }
        }
    }

    results.sort_by(|a, b| b.text.len().cmp(&a.text.len()));
    results
}

pub async fn extract_text_from_documents(
    documents: &[Document],
    #[cfg(feature = "tesseract")] ocr_engine: Option<tesseract_rs::TesseractAPI>,
    #[cfg(not(feature = "tesseract"))] ocr_engine: Option<Arc<OcrEngine>>,
) -> Vec<TextDocument> {
    let mut results = Vec::with_capacity(documents.len());
    for document in documents {
        let Ok(text) = extract_text_from_document(document, ocr_engine.clone()).await else {
            continue;
        };
        results.push(TextDocument {
            document: document.clone(),
            text,
        });
    }
    results
}

pub async fn extract_text_from_document(
    document: &Document,
    #[cfg(feature = "tesseract")] ocr_engine: Option<tesseract_rs::TesseractAPI>,
    #[cfg(not(feature = "tesseract"))] ocr_engine: Option<Arc<OcrEngine>>,
) -> crate::Result<String> {
    let document_url = document
        .document_url
        .as_ref()
        .ok_or_else(|| Box::new(crate::Error::Custom("no doc url".into())))?;
    let document_type = document
        .document_type
        .as_ref()
        .ok_or_else(|| Box::new(crate::Error::Custom("no doc type".into())))?;
    match document_type.as_str() {
        "HTML" => Ok(extract_text_from_html_document(document_url.as_str())
            .await
            .ok_or_else(|| Box::new(crate::Error::Custom("Could not extract HTML text".into())))?),
        "DigitalPDF" => match extract_text_from_pdf_document(document_url.as_str()).await {
            Ok(text) => Ok(text),
            Err(e) => {
                log::error!("Cannot process document {document:?} because of {e:?}");
                Err(e)
            }
        },
        "OriginalPDF" => {
            #[cfg(feature = "tesseract")]
            match extract_text_from_original_pdf_document_tesseract(
                document_url.as_str(),
                ocr_engine.ok_or_else(|| {
                    Box::new(crate::Error::Custom("no ocr engine available".into()))
                })?,
            )
            .await
            {
                Ok(text) => Ok(text),
                Err(e) => {
                    log::error!("Cannot process original document {document:?} because of {e:?}");
                    return Err(e);
                }
            }
            #[cfg(not(feature = "tesseract"))]
            match extract_text_from_original_pdf_document(
                document_url.as_str(),
                ocr_engine.ok_or_else(|| {
                    Box::new(crate::Error::Custom("no ocr engine available".into()))
                })?,
            )
            .await
            {
                Ok(text) => Ok(text),
                Err(e) => {
                    log::error!("Cannot process original document {document:?} because of {e:?}");
                    return Err(e);
                }
            }
        }
        _ => Err(Box::new(crate::Error::Custom(
            "unsupported doc type".into(),
        ))),
    }
}

pub async fn extract_text_from_html_document(url: &str) -> Option<String> {
    let mut text_extractor = TextExtractor::default();
    text_extractor.extract_text_from_url(&url, None, true).await
}

pub async fn extract_text_from_pdf_document(url: &str) -> crate::Result<String> {
    let client = http_client()?;
    let response = client.get(url).send().await?;
    let contents = response.bytes().await?;

    let text = tokio::task::spawn_blocking::<_, crate::Result<String>>(move || {
        let doc = poppler::Document::from_data(&contents, None)?;

        let num_pages = doc.n_pages();
        let mut full_text = String::new();

        for i in 0..num_pages {
            if let Some(page) = doc.page(i) {
                if let Some(page_text) = page.text() {
                    full_text.push_str(page_text.as_str());
                    full_text.push('\n');
                }
            }
        }
        Ok(full_text)
    })
    .await??;
    log::info!("pdf read ok, got {} chars", text.len());

    Ok(text)
}

#[cfg(feature = "tesseract")]
pub async fn extract_text_from_original_pdf_document_tesseract(
    url: &str,
    ocr_engine: tesseract_rs::TesseractAPI,
) -> crate::Result<String> {
    let contents = reqwest::get(url).await?.bytes().await?;
    let start = tokio::time::Instant::now();
    let res = tokio::task::spawn_blocking(move || {
        let images = extract_images_from_pdf(&contents)?;
        images
            .into_iter()
            .rev()
            .map(|(_, img)| pdf::image_to_text_tesseract(&ocr_engine, img))
            .collect::<crate::Result<String>>()
    })
    .await?;
    log::info!("tesseract ok: {url}, image to text: {:?}", start.elapsed());
    res
}

#[cfg(not(feature = "tesseract"))]
pub async fn extract_text_from_original_pdf_document(
    url: &str,
    ocr_engine: Arc<OcrEngine>,
) -> crate::Result<String> {
    let contents = reqwest::get(url).await?.bytes().await?;

    let start = tokio::time::Instant::now();
    let res = tokio::task::spawn_blocking(move || {
        let images = extract_images_from_pdf(&contents)?;
        images
            .into_iter()
            .rev()
            .map(|(_, img)| image_to_text(img, &ocr_engine))
            .collect::<crate::Result<String>>()
    })
    .await?;
    log::info!("image to text: {:?}", start.elapsed());
    res
}

pub fn clean_eu_documents(docs: Vec<Document>) -> Vec<Document> {
    let mut grouped_by_title: HashMap<String, Vec<Document>> = HashMap::new();
    for doc in docs {
        let Some(title) = doc.title.clone() else {
            continue;
        };
        if doc.document_type.as_deref() != Some("DigitalPDF") {
            continue;
        }
        if doc.title.is_some() {
            grouped_by_title.entry(title).or_default().push(doc);
        }
    }

    let recent_docs = grouped_by_title
        .into_iter()
        .flat_map(|(title, mut docs)| {
            docs.sort_by(|a, b| a.document_url.cmp(&b.document_url));
            docs.pop()
        })
        .collect::<Vec<_>>();

    let mut undrafted_docs = HashSet::new();
    for doc in &recent_docs {
        let mut updated_doc = doc;
        for other_doc in &recent_docs {
            if &doc.document_url == &other_doc.document_url {
                continue;
            }
            let lhs_doc_url = doc.title.as_ref().unwrap();
            let rhs_doc_url = other_doc.title.as_ref().unwrap();
            dbg!(
                &doc.title,
                lhs_doc_url,
                rhs_doc_url,
                lhs_doc_url.starts_with("DRAFT"),
                !rhs_doc_url.starts_with("DRAFT")
            );
            if lhs_doc_url.starts_with("DRAFT")
                && !rhs_doc_url.starts_with("DRAFT")
                && lhs_doc_url.contains(rhs_doc_url)
            {
                updated_doc = other_doc;
                break;
            }
        }
        undrafted_docs.insert(updated_doc.clone());
    }
    undrafted_docs.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use poppler::Document as Poppler;

    use super::*;

    #[tokio::test]
    async fn test_extract_html_from_eu_pdf() {
        let url = "https://data.europarl.europa.eu/distribution/reds_iCmOp-Dft/LIBE-PA-754988/LIBE-PA-754988_en.pdf";
        let text = extract_text_from_pdf_document(url).await.unwrap();
        dbg!(text);
    }
    #[tokio::test]
    async fn test_eu_doc_filtering() {
        let docs = vec![
            Document { title: Some("OPINION on the proposal for a regulation of the European Parliament and of the Council on the establishment of the digital euro".to_string()), document_url: Some("https://data.europarl.europa.eu/distribution/reds_iCmOp/LIBE-AD-754988/LIBE-AD-754988_en.docx".to_string()), document_type: Some("Unknown".to_string()) },
            Document { title: Some("OPINION on the proposal for a regulation of the European Parliament and of the Council on the establishment of the digital euro".to_string()), document_url: Some("https://data.europarl.europa.eu/distribution/reds_iCmOp/LIBE-AD-754988/LIBE-AD-754988_en.pdf".to_string()), document_type: Some("DigitalPDF".to_string()) },
            Document { title: Some("REPORT on the proposal for a regulation of the European Parliament and of the Council on the establishment of the digital euro".to_string()), document_url: Some("https://data.europarl.europa.eu/distribution/reds_iPlRp/A-10-2026-0185/A-10-2026-0185_en.docx".to_string()), document_type: Some("Unknown".to_string()) },
            Document { title: Some("REPORT on the proposal for a regulation of the European Parliament and of the Council on the establishment of the digital euro".to_string()), document_url: Some("https://data.europarl.europa.eu/distribution/reds_iPlRp/A-10-2026-0185/A-10-2026-0185_en.pdf".to_string()), document_type: Some("DigitalPDF".to_string()) },
            Document { title: Some("DRAFT REPORT on the proposal for a regulation of the European Parliament and of the Council on the establishment of the digital euro".to_string()), document_url: Some("https://data.europarl.europa.eu/distribution/reds_iCmRp-Dft/ECON-PR-778136/ECON-PR-778136_en.pdf".to_string()), document_type: Some("DigitalPDF".to_string()) },
            Document { title: Some("DRAFT REPORT on the proposal for a regulation of the European Parliament and of the Council on the establishment of the digital euro".to_string()), document_url: Some("https://data.europarl.europa.eu/distribution/reds_iCmRp-Dft/ECON-PR-778136/ECON-PR-778136_en.docx".to_string()), document_type: Some("Unknown".to_string()) },
            Document { title: Some("OPINION on the proposal for a regulation of the European Parliament and of the Council on the establishment of the digital euro".to_string()), document_url: Some("https://data.europarl.europa.eu/distribution/reds_iCmOp/LIBE-AD-775581/LIBE-AD-775581_en.pdf".to_string()), document_type: Some("DigitalPDF".to_string()) },
            Document { title: Some("OPINION on the proposal for a regulation of the European Parliament and of the Council on the establishment of the digital euro".to_string()), document_url: Some("https://data.europarl.europa.eu/distribution/reds_iCmOp/LIBE-AD-775581/LIBE-AD-775581_en.docx".to_string()), document_type: Some("Unknown".to_string()) },
            Document { title: Some("DRAFT REPORT on the proposal for a regulation of the European Parliament and of the Council on the establishment of the digital euro".to_string()), document_url: Some("https://data.europarl.europa.eu/distribution/reds_iCmRp-Dft/ECON-PR-758954/ECON-PR-758954_en.pdf".to_string()), document_type: Some("DigitalPDF".to_string()) },
            Document { title: Some("DRAFT REPORT on the proposal for a regulation of the European Parliament and of the Council on the establishment of the digital euro".to_string()), document_url: Some("https://data.europarl.europa.eu/distribution/reds_iCmRp-Dft/ECON-PR-758954/ECON-PR-758954_en.docx".to_string()), document_type: Some("Unknown".to_string()) },
            Document { title: Some("DRAFT OPINION on the proposal for a regulation of the European Parliament and of the Council on the establishment of the digital euro".to_string()), document_url: Some("https://data.europarl.europa.eu/distribution/reds_iCmOp-Dft/LIBE-PA-754988/LIBE-PA-754988_en.pdf".to_string()), document_type: Some("DigitalPDF".to_string()) },
            Document { title: Some("DRAFT OPINION on the proposal for a regulation of the European Parliament and of the Council on the establishment of the digital euro".to_string()), document_url: Some("https://data.europarl.europa.eu/distribution/reds_iCmOp-Dft/LIBE-PA-754988/LIBE-PA-754988_en.docx".to_string()), document_type: Some("Unknown".to_string()) },
        ];
        let cleaned = clean_eu_documents(docs);
        dbg!(cleaned);
    }
    #[tokio::test]
    async fn test_eu_doc_filtering2() {
        let docs = vec![
                Document { title: Some("REPORT on the proposal for a regulation of the European Parliament and of the Council on the establishment of the digital euro".to_string()), document_url: Some("https://data.europarl.europa.eu/distribution/reds_iPlRp/A-10-2026-0185/A-10-2026-0185_en.pdf".to_string()), document_type: Some("DigitalPDF".to_string()) },
                Document { title: Some("DRAFT REPORT on the proposal for a regulation of the European Parliament and of the Council on the establishment of the digital euro".to_string()), document_url: Some("https://data.europarl.europa.eu/distribution/reds_iCmRp-Dft/ECON-PR-778136/ECON-PR-778136_en.pdf".to_string()), document_type: Some("DigitalPDF".to_string()) },
                Document { title: Some("DRAFT REPORT on the proposal for a regulation of the European Parliament and of the Council on the establishment of the digital euro".to_string()), document_url: Some("https://data.europarl.europa.eu/distribution/reds_iCmRp-Dft/ECON-PR-758954/ECON-PR-758954_en.pdf".to_string()), document_type: Some("DigitalPDF".to_string()) },
            ];
        let cleaned = clean_eu_documents(docs);
        dbg!(cleaned);
    }

    #[tokio::test]
    async fn test_extract_text_from_html_document() {
        let url = "https://parlament.gv.at/dokument/XXVIII/A/468/fname_1715462.pdf";
        let text = extract_text_from_pdf_document(url).await.unwrap();
        println!("Extracted text: {}", &text[..]);
        let should = "
468/A(E) XXVIII. GP
Eingebracht am 24.09.2025
Dieser Text wurde elektronisch übermittelt. Abweichungen vom Original sind möglich.
Parlamentarische Materialien
ENTSCHLIESSUNGSANTRAG
der Abgeordnete";
        assert!(text.contains(should));
    }

    #[tokio::test]
    async fn test_extract_text_from_pdf_document() {
        let url = "https://www.parlament.gv.at/dokument/XXVIII/A/468/fnameorig_1715462.html";
        let text = extract_text_from_html_document(url).await;
        assert!(text.is_some());
        let text = text.unwrap();
        println!("Extracted text: {}", &text[..300.min(text.len())]);
        let should = "Mit videofähigen Drohnen";
        assert!(text.contains(should));
        assert!(text.contains("sich gegen Eingriffe,"))
    }

    #[tokio::test]
    async fn test_extract_text_from_test_docs() {
        let documents = vec![
            Document {
                title: Some("Test HTML Document".to_string()),
                document_type: Some("HTML".to_string()),
                document_url: Some(
                    "https://www.parlament.gv.at/dokument/XXVIII/A/468/fnameorig_1715462.html"
                        .to_string(),
                ),
            },
            Document {
                title: Some("Test HTML Document".to_string()),
                document_type: Some("DigitalPDF".to_string()),
                document_url: Some(
                    "https://www.parlament.gv.at/dokument/XXVIII/A/468/fnameorig_1715462.pdf"
                        .to_string(),
                ),
            },
            Document {
                title: Some("Test PDF Document".to_string()),
                document_type: Some("DigitalPDF".to_string()),
                document_url: Some(
                    "https://www.parlament.gv.at/dokument/XXVIII/A/468/fname_1715462.pdf"
                        .to_string(),
                ),
            },
        ];
        let text_documents =
            extract_text_from_documents_html_first_stripped(&documents, None).await;
        assert_eq!(text_documents.len(), 2);
        assert_eq!(
            text_documents[0].document.title.as_deref(),
            Some("Test HTML Document")
        );
        assert_eq!(
            text_documents[1].document.title.as_deref(),
            Some("Test PDF Document")
        );
        // for text_doc in text_documents {
        //     println!(
        //         "Document Title: {:?}, Extracted Text Snippet: {}",
        //         text_doc.document.title,
        //         &text_doc.text[..300.min(text_doc.text.len())]
        //     );
        // }
    }

    #[tokio::test]
    async fn test_extract_text_from_specific_html() {
        let documents = vec![Document {
            title: Some("Test HTML Document".to_string()),
            document_type: Some("HTML".to_string()),
            document_url: Some(
                "https://www.parlament.gv.at/dokument/XXVIII/III/257/fnameorig_1723642.html"
                    .to_string(),
            ),
        }];
        let text_documents =
            extract_text_from_documents_html_first_stripped(&documents, None).await;
        let input = combine_texts(&text_documents);
        dbg!(input);
    }

    #[cfg(feature = "tesseract")]
    #[tokio::test]
    async fn test_extract_text_from_original_pdf() {
        let url = "https://www.parlament.gv.at/dokument/XXVIII/A/568/imfname_1722828.pdf";
        let documents = vec![Document {
            title: Some("Test Original PDF Document".to_string()),
            document_type: Some("OriginalPDF".to_string()),
            document_url: Some(url.to_string()),
        }];

        #[cfg(not(feature = "tesseract"))]
        let ocr_engine = Arc::new(
            crate::pdf::init_ocr_engine("/home/elftausend/.dataservice/")
                .expect("Failed to initialize OCR engine"),
        );

        let api = crate::pdf::init_ocr_engine_tesseract("/usr/share/tessdata").unwrap();
        let text = extract_text_from_original_pdf_document_tesseract(
            documents[0].document_url.as_deref().unwrap(),
            api,
        )
        .await
        .unwrap();

        println!("text: {text}");

        // extract_text_from_original_pdf_document(documents[0].document_url.as_deref().unwrap(), ocr_engine).await.unwrap();
        // let text_documents =
        //     extract_text_from_documents_html_first(&documents, Some(ocr_engine)).await;
        // assert_eq!(text_documents.len(), 1);
    }

    #[tokio::test]
    async fn test_extract_text_from_html_ris() {
        // enable simple logger
        simple_logger::init_with_level(log::Level::Info).unwrap();
        // Document {
        //             title: "Html",
        //             link: "https://www.ris.bka.gv.at/Dokumente/BgblAuth/BGBLA_2026_II_18/BGBLA_2026_II_18.html",
        //             ftype: HTML,
        //         },
        let documents = vec![Document {
            title: Some("Html".to_string()),
            document_type: Some("HTML".to_string()),
            document_url: Some(
                "https://www.ris.bka.gv.at/Dokumente/BgblAuth/BGBLA_2026_II_18/BGBLA_2026_II_18.html"
                    .to_string(),
            ),
        }];
        let text_documents =
            extract_text_from_html_document(documents[0].document_url.as_deref().unwrap()).await;
        dbg!(text_documents);
    }

    #[tokio::test]
    async fn test_stack_overflow_doc() {
        let url = "https://www.parlament.gv.at/dokument/XXIV/III/138/imfname_186126.pdf";
        let documents = vec![Document {
            title: Some("Hauptdokument".to_string()),
            document_type: Some("DigitalPDF".to_string()),
            document_url: Some(url.to_string()),
        }];

        let contents = reqwest::get(url).await.unwrap().bytes().await.unwrap();

        // let (content, metadata) = extractor.extract_file_to_string(file_path).unwrap();

        // let res = extract_text_from_pdf_document(url).await.unwrap();
        // let text_documents = extract_text_from_documents_html_first(&documents, None).await;
        // println!("{}", &text_document[..])
    }
}
