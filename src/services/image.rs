use image::imageops::resize;
use image::{GrayImage, ImageFormat, Luma};
use reqwest::multipart::{Form, Part};
use std::error::Error as StdError;
use std::io::Cursor;

pub struct ImageService;

impl ImageService {
    pub async fn edit_image(image: Vec<u8>, prompt: String) -> Result<String, Box<dyn StdError + Send + Sync>> {
        let api_key = crate::config::openai_api_key();

        let image_reader = image::ImageReader::with_format(Cursor::new(&image), ImageFormat::Png);
        let image = image_reader.decode()?.to_rgba8();

        let resized = resize(&image, 512, 512, image::imageops::FilterType::Lanczos3);
        let mut resized_image_bytes = Vec::new();
        resized.write_to(&mut Cursor::new(&mut resized_image_bytes), ImageFormat::Png)?;

        let mask = generate_white_mask(resized.width(), resized.height())?;

        let form = Form::new()
            .text("prompt", prompt.to_string())
            .text("n", "1")
            .part(
                "image",
                Part::bytes(resized_image_bytes)
                    .file_name("image.png")
                    .mime_str("image/png")?,
            )
            .part(
                "mask",
                Part::bytes(mask)
                    .file_name("mask.png")
                    .mime_str("image/png")?,
            );

        let client = reqwest::Client::new();
        let res = client
            .post("https://api.openai.com/v1/images/edits")
            .bearer_auth(api_key)
            .multipart(form)
            .send()
            .await?;

        let text = res.text().await?;
        println!("Image edit response: {}", text);

        Ok(text)
    }
}

fn generate_white_mask(width: u32, height: u32) -> Result<Vec<u8>, Box<dyn StdError + Send + Sync>> {
    let mut mask = GrayImage::new(width, height);
    for pixel in mask.pixels_mut() {
        *pixel = Luma([255]);
    }

    let mut buf = Cursor::new(Vec::new());
    mask.write_to(&mut buf, ImageFormat::Png)?;
    Ok(buf.into_inner())
}
