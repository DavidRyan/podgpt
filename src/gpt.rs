use async_openai::{
    Client,
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
        CreateChatCompletionRequestArgs, CreateImageRequestArgs, ImageResponseFormat, ImageSize,
    },
};
use std::error::Error;

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
    async fn image_prompt(&self, prompt: String) -> Result<String, Box<dyn Error>>;
}

pub trait ChatPrompt {
    async fn reply_to_chat(&mut self, promt: String) -> Result<String, Box<dyn Error>>;
    async fn create_chat(&mut self, promt: String) -> Result<String, Box<dyn Error>>;
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
    async fn image_prompt(&self, prompt: String) -> Result<String, Box<dyn Error>> {
        // TODO - get image from Discord URL 
        // - save locally
        // - upload to GPT with prompt
        // either dwnload and retur url or save and send to discord
        Ok("OK".to_string())
    }
}

