use std::io::Cursor;

use async_openai::{
    config::OpenAIConfig,
    types::{
        CreateImageEditRequestArgs, CreateImageRequestArgs, DallE2ImageSize, Image, ImageInput,
        ImageSize, InputSource,
    },
    Client,
};
use image::imageops::resize;
use image::{GrayImage, ImageFormat, Luma};

use crate::error::AppError;

pub struct ImageService {
    client: Client<OpenAIConfig>,
}

impl ImageService {
    pub fn new(client: Client<OpenAIConfig>) -> Self {
        Self { client }
    }

    pub async fn generate(&self, prompt: &str) -> Result<String, AppError> {
        let request = CreateImageRequestArgs::default()
            .prompt(prompt)
            .n(1u8)
            .size(ImageSize::S512x512)
            .build()?;

        let response = self.client.images().create(request).await?;

        extract_url(&response.data)
    }

    pub async fn edit(&self, image_bytes: Vec<u8>, prompt: &str) -> Result<String, AppError> {
        let img = image::ImageReader::new(Cursor::new(&image_bytes))
            .with_guessed_format()?
            .decode()?
            .to_rgba8();

        let resized = resize(&img, 512, 512, image::imageops::FilterType::Lanczos3);
        let mut resized_bytes = Vec::new();
        resized.write_to(&mut Cursor::new(&mut resized_bytes), ImageFormat::Png)?;

        let mask_bytes = generate_white_mask(resized.width(), resized.height())?;

        let request = CreateImageEditRequestArgs::default()
            .image(ImageInput {
                source: InputSource::VecU8 {
                    filename: "image.png".to_string(),
                    vec: resized_bytes,
                },
            })
            .mask(ImageInput {
                source: InputSource::VecU8 {
                    filename: "mask.png".to_string(),
                    vec: mask_bytes,
                },
            })
            .prompt(prompt)
            .n(1u8)
            .size(DallE2ImageSize::S512x512)
            .build()?;

        let response = self.client.images().create_edit(request).await?;

        extract_url(&response.data)
    }
}

fn extract_url(data: &[std::sync::Arc<Image>]) -> Result<String, AppError> {
    match data.first().map(|img| img.as_ref()) {
        Some(Image::Url { url, .. }) => Ok(url.clone()),
        _ => Err(AppError::NoImageUrl),
    }
}

fn generate_white_mask(width: u32, height: u32) -> Result<Vec<u8>, AppError> {
    let mut mask = GrayImage::new(width, height);
    for pixel in mask.pixels_mut() {
        *pixel = Luma([255]);
    }

    let mut buf = Cursor::new(Vec::new());
    mask.write_to(&mut buf, ImageFormat::Png)?;
    Ok(buf.into_inner())
}
