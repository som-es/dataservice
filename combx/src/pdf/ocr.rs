use image::DynamicImage;
use ocrs::{ImageSource, OcrEngine, OcrEngineParams};
use rten::{Model, ModelLoadError};

pub async fn download_ocr_models(model_dir: &str) -> crate::Result<()> {
    let detection_model_url = "https://ocrs-models.s3-accelerate.amazonaws.com/text-detection.rten";
    let recognition_model_url =
        "https://ocrs-models.s3-accelerate.amazonaws.com/text-recognition.rten";

    for url in [detection_model_url, recognition_model_url] {
        let filename = url
            .split('/')
            .last()
            .ok_or_else(|| crate::Error::Custom("Invalid model URL".to_string()))?;
        let filepath = format!("{}/{}", model_dir, filename);
        if std::path::Path::new(&filepath).exists() {
            continue;
        }
        let contents = reqwest::get(url).await?.bytes().await?;
        std::fs::write(&filepath, contents)?;
    }
    Ok(())
}

pub fn init_ocr_engine(model_dir: &str) -> Result<OcrEngine, ModelLoadError> {
    let detection_model = Model::load_file(format!("{model_dir}/text-detection.rten"))?;
    let recognition_model = Model::load_file(format!("{model_dir}/text-recognition.rten"))?;

    let engine = OcrEngine::new(OcrEngineParams {
        detection_model: Some(detection_model),
        recognition_model: Some(recognition_model),
        ..Default::default()
    })
    .unwrap();
    Ok(engine)
}

pub fn image_to_text(img: DynamicImage, engine: &OcrEngine) -> crate::Result<String> {
    let img = img.into_rgb8();
    let img_source = ImageSource::from_bytes(img.as_raw(), img.dimensions())?;
    let ocr_input = engine.prepare_input(img_source)?;
    let word_rects = engine.detect_words(&ocr_input)?;
    let line_rects = engine.find_text_lines(&ocr_input, &word_rects);
    let line_texts = engine.recognize_text(&ocr_input, &line_rects)?;

    Ok(line_texts
        .iter()
        .flatten()
        .map(|l| l.to_string())
        .filter(|l| l.len() > 1)
        .collect())
}
