use image::DynamicImage;
use lopdf::{Document, Object};

const MAX_IMAGE_COUNT: usize = 16; // MAX_IMAGE_COUNT pages of law de facto

pub fn extract_images_from_pdf(contents: &[u8]) -> crate::Result<Vec<((u32, u16), DynamicImage)>> {
    let doc = Document::load_mem(&contents)?;
    let mut collected_images = Vec::new();

    for (_id, object) in &doc.objects {
        if collected_images.len() >= MAX_IMAGE_COUNT {
            break
        }
        match object {
            Object::Stream(stream) => {
                let is_image = stream
                    .dict
                    .get(b"Subtype")
                    .and_then(|o| o.as_name())
                    .map(|n| n == b"Image")
                    .unwrap_or(false);

                if is_image {
                    let filter = stream
                        .dict
                        .get(b"Filter")
                        .and_then(|o: &Object| o.as_name())
                        .unwrap_or(b"Raw");

                    // let width = stream.dict.get(b"Width").and_then(|o| o.as_i64()).unwrap_or(0) as u32;
                    // let height = stream.dict.get(b"Height").and_then(|o| o.as_i64()).unwrap_or(0) as u32;

                    if filter == b"DCTDecode" {
                        if let Ok(img) = image::load_from_memory(&stream.content) {
                            collected_images.push((*_id, img));
                        }
                    }
                    /*else if filter == b"FlateDecode" {
                        // Decompress zlib
                        if let Ok(data) = stream.decompressed_content() {
                            let color_space = stream.dict.get(b"ColorSpace")
                                .and_then(|o| o.as_name())
                                .unwrap_or(b"DeviceRGB");

                            if color_space == b"DeviceGray" {
                                // 1 byte per pixel -> Luma8
                                if let Some(buffer) = ImageBuffer::<Luma<u8>, Vec<u8>>::from_vec(width, height, data) {
                                    collected_images.push(DynamicImage::ImageLuma8(buffer));
                                }
                            } else {
                                // Default to DeviceRGB: 3 bytes per pixel -> Rgb8
                                // (Note: If it's CMYK, this might look odd, but Image-rs handles RGB best)
                                if let Some(buffer) = ImageBuffer::<Rgb<u8>, Vec<u8>>::from_vec(width, height, data) {
                                    collected_images.push(DynamicImage::ImageRgb8(buffer));
                                }
                            }
                        }
                    }*/
                }
            }
            _ => {}
        }
    }

    // for (i, (id, img)) in collected_images.iter().enumerate() {
    //     img.save(format!("extracted_image_{:?}.png", id)).unwrap();
    // }

    Ok(collected_images)
}
#[cfg(test)]
mod tests {
    use crate::pdf::img_extraction::extract_images_from_pdf;

    #[tokio::test]
    async fn test_pdf_image_extraction() {
        // let url = "https://parlament.gv.at/dokument/XXVIII/A/651/imfname_1729221.pdf";
        // let contents = reqwest::get(url).await.unwrap().bytes().await.unwrap();

        let url2 = "https://parlament.gv.at/dokument/XXVIII/A/644/imfname_1729196.pdf";
        let contents2 = reqwest::get(url2).await.unwrap().bytes().await.unwrap();
        extract_images_from_pdf(&contents2).unwrap();
    }
}
