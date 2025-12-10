use std::collections::HashMap;

use somes_common_lib::Document;
use somes_scraper::{AustriaScraper, Scraper, TextExtractor};

pub struct TextDocument {
    pub document: Document,
    pub text: String,
}

pub fn combine_texts(documents: &[TextDocument]) -> String {
    documents
        .iter()
        .map(|doc| {
            format!(
                "{}\n {}",
                doc.document.title.as_deref().unwrap_or(""),
                doc.text
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub async fn extract_text_from_documents_html_first(documents: &[Document]) -> Vec<TextDocument> {
    let mut results = Vec::with_capacity(documents.len());
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
        let mut try_extract_pdf = false;
        match maybe_html {
            Some(html_doc) => {
                if let Some(text) = extract_text_from_document(html_doc).await {
                    results.push(TextDocument {
                        document: html_doc.clone(),
                        text,
                    });
                } else {
                    try_extract_pdf = true;
                }
            }
            None => {
                try_extract_pdf = true;
            }
        }
        if try_extract_pdf {
            let maybe_pdf = docs
                .iter()
                .find(|x| x.document_type.as_deref() == Some("DigitalPDF"));
            if let Some(pdf_doc) = maybe_pdf {
                if let Some(text) = extract_text_from_document(pdf_doc).await {
                    results.push(TextDocument {
                        document: pdf_doc.clone(),
                        text,
                    });
                }
            }
        }
    }

    results
}

pub async fn extract_text_from_documents(documents: &[Document]) -> Vec<TextDocument> {
    let mut results = Vec::with_capacity(documents.len());
    for document in documents {
        let Some(text) = extract_text_from_document(document).await else {
            continue;
        };
        results.push(TextDocument {
            document: document.clone(),
            text,
        });
    }
    results
}

pub async fn extract_text_from_document(document: &Document) -> Option<String> {
    let document_url = document.document_url.as_ref()?;
    let document_type = document.document_type.as_ref()?;
    match document_type.as_str() {
        "HTML" => extract_text_from_html_document(document_url.as_str()).await,
        "DigitalPDF" => extract_text_from_pdf_document(document_url.as_str()).await,
        _ => None,
    }
}

pub async fn extract_text_from_html_document(url: &str) -> Option<String> {
    let mut text_extractor = TextExtractor::new(AustriaScraper::BASE_URL);
    text_extractor.extract_text_from_url(&url, None, true).await
}

pub async fn extract_text_from_pdf_document(_url: &str) -> Option<String> {
    // TODO: store base url directly in the document url
    let contents = reqwest::get(format!("{}{}", AustriaScraper::BASE_URL, _url))
        .await
        .ok()?
        .bytes()
        .await
        .ok()?;
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        pdf_extract::extract_text_from_mem(&contents).ok()
    }))
    .ok()
    .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_extract_text_from_html_document() {
        let url = "/dokument/XXVIII/A/468/fname_1715462.pdf";
        let text = extract_text_from_pdf_document(url).await;
        assert!(text.is_some());
        let text = text.unwrap();
        println!("Extracted text: {}", &text[..300.min(text.len())]);
        let should = "
  Dieser Text wurde elektronisch übermittelt. Abweichungen vom Original sind möglich. 

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
        let url = "/dokument/XXVIII/A/468/fnameorig_1715462.html";
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
                document_url: Some("/dokument/XXVIII/A/468/fnameorig_1715462.html".to_string()),
            },
            Document {
                title: Some("Test HTML Document".to_string()),
                document_type: Some("DigitalPDF".to_string()),
                document_url: Some("/dokument/XXVIII/A/468/fnameorig_1715462.pdf".to_string()),
            },
            Document {
                title: Some("Test PDF Document".to_string()),
                document_type: Some("DigitalPDF".to_string()),
                document_url: Some("/dokument/XXVIII/A/468/fname_1715462.pdf".to_string()),
            },
        ];
        let text_documents = extract_text_from_documents_html_first(&documents).await;
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
}
