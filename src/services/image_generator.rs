use async_trait::async_trait;

#[async_trait]
pub trait ImageGenerator: Send + Sync {
    async fn generate(&self, prompt: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;
}

pub struct OpenAiImageGenerator {
    api_key: String,
}

impl OpenAiImageGenerator {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }
}

#[async_trait]
impl ImageGenerator for OpenAiImageGenerator {
    async fn generate(&self, prompt: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let client = reqwest::Client::new();
        let res = client
            .post("https://api.openai.com/v1/images/generations")
            .bearer_auth(&self.api_key)
            .header("Content-Type", "application/json")
            .body(format!("{{\"prompt\": \"{}\", \"n\":1, \"size\":\"512x512\"}}", prompt.replace('"', "\\\"")))
            .send()
            .await?;
        let res_json: serde_json::Value = res.json().await?;
        let image_url = res_json["data"][0]["url"].as_str().ok_or("No image URL in response")?.to_string();
        Ok(image_url)
    }
}
