use async_openai::types::{ImageInput, ImageModel};
use async_openai::{
    config::OpenAIConfig, types::{
        ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs, CreateChatCompletionRequestArgs}, Client
};
use std::io::Cursor;
use std::{error::Error, io::Write};
use reqwest::multipart::{Form, Part};
use std::fs;
use std::env;
use image::{io::Reader as ImageReader, GrayImage, Luma, ImageFormat};

#[derive(Debug)]
pub struct Conversation {
    pub _id: String,
    pub messages: Vec<String>,
}

pub struct Gpt {
    client: Client<OpenAIConfig>,
    conversation: Conversation,
}

impl Gpt {
    pub fn new() -> Self {
        Gpt {
            client: Client::with_config(OpenAIConfig::new()),
            conversation: Conversation {
                _id: "1".to_string(),
                messages: vec![],
            },
        }
    }
}

pub trait ImagePrompt {
    async fn image_prompt(&self, image: Vec<u8>, prompt: String) -> Result<String, Box<dyn Error>>;
}

pub trait ChatPrompt {
    async fn reply_to_chat(&mut self, promt: String) -> Result<String, Box<dyn Error>>;
    async fn create_chat(&mut self, promt: String) -> Result<String, Box<dyn Error>>;
}

fn save_image(image: Vec<u8>) -> Result<String, Box<dyn Error>> {
    println!("Saving image: data/image.png");
    let path = "data/image.png";
    use std::fs::File;
    let mut file = File::create(path)?;
    file.write_all(&image).unwrap();
    println!("Image saved to: {}", path);
    Ok(path.to_string())
}

impl ChatPrompt for Gpt {

    async fn reply_to_chat(&mut self, promt: String) -> Result<String, Box<dyn Error>> {
        self.conversation.messages.push(promt.clone());
        println!("Conversation: {:?}", self.conversation);

        let mapped_messages: Vec<ChatCompletionRequestMessage> = self
            .conversation
            .messages
            .iter()
            .map(|msg| {
                ChatCompletionRequestSystemMessageArgs::default()
                    .content(msg.to_string())
                    .build()
                    .unwrap()
                    .into()
            })
        .collect::<Vec<_>>();

        let request = CreateChatCompletionRequestArgs::default()
            .max_tokens(512u32)
            .model("gpt-4o")
            .messages(mapped_messages)
            .build()?;
        let response = self.client.chat().create(request).await?;
        let msg = response.choices[0]
            .message
            .clone()
            .content
            .unwrap()
            .to_string();
        self.conversation.messages.push(msg.clone());
        Ok(msg)
    }

    async fn create_chat(&mut self, promt: String) -> Result<String, Box<dyn Error>> {
        let request = CreateChatCompletionRequestArgs::default()
            .max_tokens(512u32)
            .model("gpt-4o")
            .messages([ChatCompletionRequestSystemMessageArgs::default()
                .content(promt.clone())
                .build()?
                .into()])
            .build()?;
        let response = self.client.chat().create(request).await?;
        println!("Response: {:?}", response);
        let msg = response.choices[0]
            .message
            .clone()
            .content
            .unwrap()
            .to_string();
        self.conversation = Conversation {
            _id: response.id,
            messages: vec![],
        };
        self.conversation.messages.push(promt);
        self.conversation.messages.push(msg.clone());
        println!("Conversation1: {:?}", self.conversation);
        Ok(msg)
    }
}

impl ImagePrompt for Gpt {
    async fn image_prompt(&self, image: Vec<u8>, prompt: String) -> Result<String, Box<dyn Error>> {

        let path = save_image(image).unwrap();


        let api_key = env::var("OPENAI_API_KEY")?;

        // Read the image file
        let image_bytes = fs::read(&path)?;
        let image_reader = ImageReader::with_format(Cursor::new(&image_bytes), ImageFormat::Png);
        let image = image_reader.decode()?.to_rgba8();
        use image::{imageops::FilterType, imageops, RgbaImage};

        let resized: RgbaImage = imageops::resize(&image, 512, 512, FilterType::Lanczos3);
        resized.save("data/resized.png")?;

        
        let mask = generate_white_mask(resized.width(), resized.height()).unwrap();
        let rezied_image_bytes = fs::read("data/resized.png")?;

        // Build multipart form with correct MIME
        let form = Form::new()
            .text("prompt", prompt.to_string())
            .text("n", "1")
            .part(
                "image",
                Part::bytes(rezied_image_bytes.clone())
                .file_name("image.png")
                .mime_str("image/png")?, // Important
            )
            .part(
                "mask",
                Part::bytes(mask)
                .file_name("mask.png")
                .mime_str("image/png")?,
            );

        // Send the request
        let client = reqwest::Client::new();
        let res = client
            .post("https://api.openai.com/v1/images/edits")
            .bearer_auth(api_key)
            .multipart(form)
            .send()
            .await?;

        let text = res.text().await?;
        println!("Response: {}", text);

        Ok(text.to_string())
    }
}

fn generate_white_mask(width: u32, height: u32) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut mask = GrayImage::new(width, height);
    for pixel in mask.pixels_mut() {
        *pixel = Luma([255]); // white = fully editable
    }

    let mut buf = Cursor::new(Vec::new());
    mask.write_to(&mut buf, ImageFormat::Png)?;
    Ok(buf.into_inner())
}

