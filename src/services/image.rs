use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use async_openai::{
    config::OpenAIConfig,
    types::{
        CreateImageRequestArgs, Image, ImageModel,
    },
    Client,
};
use tokio::sync::Mutex;

use super::tools::Tool;
use crate::error::AppError;

pub struct ImageService {
    client: Client<OpenAIConfig>,
}

/// Result of image generation — either a URL or raw PNG bytes.
pub enum GeneratedImage {
    Url(String),
    Bytes(Vec<u8>),
}

impl ImageService {
    pub fn new(client: Client<OpenAIConfig>) -> Self {
        Self { client }
    }

    pub async fn generate(&self, prompt: &str) -> Result<GeneratedImage, AppError> {
        let request = CreateImageRequestArgs::default()
            .prompt(prompt)
            .model(ImageModel::Other("gpt-image-1".to_string()))
            .n(1u8)
            .build()?;

        let response = self.client.images().create(request).await?;

        match response.data.first().map(|img| img.as_ref()) {
            Some(Image::B64Json { b64_json, .. }) => {
                use base64::Engine;
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(b64_json.as_str())
                    .map_err(|e| AppError::Image(e.to_string()))?;
                Ok(GeneratedImage::Bytes(bytes))
            }
            Some(Image::Url { url, .. }) => Ok(GeneratedImage::Url(url.clone())),
            _ => Err(AppError::NoImageUrl),
        }
    }
}

/// Shared buffer for images generated during a tool-call loop.
pub type ImageBuffer = Arc<Mutex<Vec<Vec<u8>>>>;

pub fn new_image_buffer() -> ImageBuffer {
    Arc::new(Mutex::new(Vec::new()))
}

/// Tool that GPT can call to generate images during conversations.
pub struct ImageGeneratorTool {
    client: Client<OpenAIConfig>,
    buffer: ImageBuffer,
}

impl ImageGeneratorTool {
    pub fn new(client: Client<OpenAIConfig>, buffer: ImageBuffer) -> Self {
        Self { client, buffer }
    }

    async fn generate_image(&self, prompt: &str) -> String {
        let service = ImageService { client: self.client.clone() };
        match service.generate(prompt).await {
            Ok(GeneratedImage::Bytes(bytes)) => {
                tracing::info!(bytes = bytes.len(), "Image generated successfully");
                self.buffer.lock().await.push(bytes);
                "Image generated successfully. It will be attached to your response.".to_string()
            }
            Ok(GeneratedImage::Url(url)) => {
                tracing::info!(%url, "Image generated with URL");
                format!("Image generated: {url}")
            }
            Err(e) => {
                tracing::error!(error = %e, "Image generation failed");
                format!("Failed to generate image: {e}")
            }
        }
    }
}

impl Tool for ImageGeneratorTool {
    fn name(&self) -> &str {
        "generate_image"
    }

    fn description(&self) -> &str {
        "Generate an image using AI. Use when the user asks you to create, draw, or generate \
         an image. Provide a detailed, descriptive prompt for the best results."
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "prompt": {
                    "type": "string",
                    "description": "A detailed description of the image to generate"
                }
            },
            "required": ["prompt"],
            "additionalProperties": false
        })
    }

    fn execute(&self, arguments: &str) -> Pin<Box<dyn Future<Output = String> + Send + '_>> {
        let args = arguments.to_string();
        Box::pin(async move {
            let prompt = serde_json::from_str::<serde_json::Value>(&args)
                .ok()
                .and_then(|v| v["prompt"].as_str().map(|s| s.to_string()));

            match prompt {
                Some(p) => {
                    tracing::info!(prompt = %p, "Generating image via tool");
                    self.generate_image(&p).await
                }
                None => "Invalid arguments: missing 'prompt' field.".to_string(),
            }
        })
    }
}
