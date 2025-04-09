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

impl Gpt {
    pub async fn _create_image(&self, prompt: String) -> Result<String, Box<dyn Error>> {
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

    pub async fn reply_to_chat(&mut self, promt: String) -> Result<String, Box<dyn Error>> {
        self.conversation.messages.push(promt.clone());
        println!("Conversation: {:?}", self.conversation);

        //ChatCompletionRequestMessage
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

    pub async fn create_chat(&mut self, promt: String) -> Result<String, Box<dyn Error>> {
        let request = CreateChatCompletionRequestArgs::default()
            .max_tokens(512u32)
            .model("gpt-4o")
            .messages([ChatCompletionRequestSystemMessageArgs::default()
                .content(promt.clone())
                .build()?
                .into()])
            .build()?;
        let response = self.client.chat().create(request).await?;
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
