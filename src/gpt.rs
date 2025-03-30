use async_openai::{
    types::{CreateCompletionRequestArgs, CreateImageRequestArgs, ImageResponseFormat, ImageSize, RequiredAction},
    Client, config::OpenAIConfig,
};
use std::{error::Error, sync::Arc};

pub struct Gpt {
    client: Client<OpenAIConfig>
}

impl Gpt {
    pub fn new() -> Self {
        Gpt {
            client:  Client::new()
        }
    }}

impl Gpt {
    pub async fn create_image(&self, prompt: String) -> Result<(String), Box<dyn Error>> {

        let request = CreateImageRequestArgs::default()
            .prompt(prompt)
            .response_format(ImageResponseFormat::Url)
            .size(ImageSize::S256x256)
            .user("async-openai")
            .build()?;

        let response = self.client.images().create(request).await?;

        // Download and save images to ./data directory.
        // Each url is downloaded and saved in dedicated Tokio task.
        // Directory is created if it doesn't exist.
        let paths = response.save("./data").await?;

        paths
            .iter()
            .for_each(|path| println!("Image file path: {}", path.display()));


        let s = paths.first().unwrap().to_str().unwrap();
        println!("Image file path: {}", s);
        Ok(s.to_string())
    }

    pub async fn create_chat(&self, promt: String) -> Result<(String), Box<dyn Error>> {
        let request = CreateCompletionRequestArgs::default()
            .model("gpt-3.5-turbo-instruct")
            .prompt(promt)
            .build()?;
        let response = self.client.completions().create(request).await.unwrap();
        let msg = &response.choices[0].text;
        Ok(msg.to_string())
    }
}
