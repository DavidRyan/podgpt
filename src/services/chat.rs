use std::sync::Arc;

use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
        ChatCompletionRequestMessageContentPartImageArgs,
        ChatCompletionRequestMessageContentPartTextArgs,
        ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestToolMessageArgs,
        ChatCompletionRequestUserMessageArgs, ChatCompletionRequestUserMessageContent,
        ChatCompletionRequestUserMessageContentPart, ChatCompletionTool,
        CreateChatCompletionRequestArgs, FinishReason, ImageUrlArgs,
    },
    Client,
};
use dashmap::DashMap;
use tokio::sync::Mutex;

use super::conversation::{ChatMessage, Conversation, MessageRole};
use super::image::{ImageBuffer, new_image_buffer};
use super::search::SearchTool;
use super::tools::Tool;
use crate::config::Config;
use crate::error::AppError;

const MAX_TOOL_ROUNDS: usize = 5;

pub struct HistoryResult {
    pub messages: Vec<ChatMessage>,
    pub total: usize,
}

pub struct ChatResponse {
    pub text: String,
    pub images: Vec<Vec<u8>>,
}

pub struct ChatService {
    conversations: DashMap<String, Arc<Mutex<Conversation>>>,
    client: Client<OpenAIConfig>,
    config: Arc<Config>,
    tools: Vec<Box<dyn Tool>>,
    image_buffer: ImageBuffer,
}

impl ChatService {
    pub fn new(client: Client<OpenAIConfig>, config: Arc<Config>) -> Self {
        let image_buffer = new_image_buffer();
        let mut service = Self {
            conversations: DashMap::new(),
            client: client.clone(),
            config: Arc::clone(&config),
            tools: Vec::new(),
            image_buffer: image_buffer.clone(),
        };

        if let Some(ref key) = config.tavily_api_key {
            service.register_tool(SearchTool::new(key.clone()));
        }

        service.register_tool(super::image::ImageGeneratorTool::new(client, image_buffer));

        service
    }

    /// Register a tool that GPT can invoke during conversations.
    pub fn register_tool(&mut self, tool: impl Tool + 'static) {
        self.tools.push(Box::new(tool));
    }

    pub async fn create(&self, user_id: &str, prompt: &str, channel_id: u64, image_urls: Vec<String>) -> Result<ChatResponse, AppError> {
        // Clear any leftover images from previous calls
        self.image_buffer.lock().await.clear();

        let text = self.send_chat(&[], prompt, channel_id, &image_urls).await?;
        let images = std::mem::take(&mut *self.image_buffer.lock().await);

        let mut conversation = Conversation::new();
        conversation.messages.push(ChatMessage {
            role: MessageRole::User,
            content: prompt.to_string(),
            image_urls,
        });
        conversation.messages.push(ChatMessage {
            role: MessageRole::Assistant,
            content: text.clone(),
            image_urls: Vec::new(),
        });

        self.conversations
            .insert(user_id.to_string(), Arc::new(Mutex::new(conversation)));

        Ok(ChatResponse { text, images })
    }

    pub async fn reply(&self, user_id: &str, prompt: &str, channel_id: u64, image_urls: Vec<String>) -> Result<ChatResponse, AppError> {
        let conv_mutex = {
            let entry = self
                .conversations
                .get(user_id)
                .ok_or(AppError::NoConversation)?;
            Arc::clone(entry.value())
        };

        let mut conv = conv_mutex.lock().await;

        // Clear any leftover images from previous calls
        self.image_buffer.lock().await.clear();

        let max_history = self.config.max_history_messages;
        let start = conv.messages.len().saturating_sub(max_history);
        let text = self.send_chat(&conv.messages[start..], prompt, channel_id, &image_urls).await?;
        let images = std::mem::take(&mut *self.image_buffer.lock().await);

        conv.messages.push(ChatMessage {
            role: MessageRole::User,
            content: prompt.to_string(),
            image_urls,
        });
        conv.messages.push(ChatMessage {
            role: MessageRole::Assistant,
            content: text.clone(),
            image_urls: Vec::new(),
        });

        let max = max_history * 2;
        if conv.messages.len() > max {
            let keep = conv.messages.len() - max_history;
            conv.messages.drain(0..keep);
        }

        Ok(ChatResponse { text, images })
    }

    pub fn clear(&self, user_id: &str) -> bool {
        self.conversations.remove(user_id).is_some()
    }

    pub async fn get_history(&self, user_id: &str, count: usize) -> Option<HistoryResult> {
        let conv_mutex = {
            let entry = self.conversations.get(user_id)?;
            Arc::clone(entry.value())
        };

        let conv = conv_mutex.lock().await;
        let total = conv.messages.len();
        let start = total.saturating_sub(count);

        Some(HistoryResult {
            messages: conv.messages[start..].to_vec(),
            total,
        })
    }

    pub async fn get_conversation_summary(&self, user_id: &str) -> Option<String> {
        let conv_mutex = {
            let entry = self.conversations.get(user_id)?;
            Arc::clone(entry.value())
        };

        let conv = conv_mutex.lock().await;
        Some(conv.summary())
    }

    /// Generate a roast of a user based on their recent messages in the channel.
    pub async fn roast(
        &self,
        target: &str,
        channel_id: u64,
    ) -> Result<String, AppError> {
        // Fetch recent messages using the channel_summary tool
        let mut channel_messages = String::from("No messages found.");
        for tool in &self.tools {
            if tool.name() == "channel_summary" {
                let args = serde_json::json!({
                    "channel_id": channel_id.to_string(),
                    "limit": 50
                })
                .to_string();
                channel_messages = tool.execute(&args).await;
                break;
            }
        }

        let prompt = format!(
            "Based on the following recent Discord messages, write a brutal roast of the user \
             \"{target}\". Tear apart what they've said, how they talk, their terrible opinions, \
             their personality, and their behavior in chat. Be specific — quote them and use their \
             own words against them. Go for the jugular. \
             If you can't find messages from them, absolutely destroy them for being a lurker \
             who contributes nothing.\n\n\
             Messages:\n{channel_messages}"
        );

        let system = "You are the meanest comedian at a no-holds-barred roast between close friends. \
            Nothing is off limits. Be ruthless, personal, and creative with your insults. \
            Use sarcasm, backhanded compliments, and cutting observations. \
            Make it sting but make it funny. No generic insults — everything should be specific \
            to what this person actually said and did in chat. \
            Keep it to 3-5 sentences max.";

        let messages = build_api_messages(system, &[], &prompt, &[]);

        let request = CreateChatCompletionRequestArgs::default()
            .model(self.config.model.clone())
            .max_completion_tokens(self.config.max_tokens)
            .messages(messages)
            .build()?;

        let response = self.client.chat().create(request).await?;

        response
            .choices
            .first()
            .and_then(|c| c.message.content.as_deref())
            .map(|s| s.to_string())
            .ok_or(AppError::NoResponseContent)
    }

    // --- private ---

    fn tool_definitions(&self) -> Option<Vec<ChatCompletionTool>> {
        if self.tools.is_empty() {
            return None;
        }
        Some(self.tools.iter().map(|t| t.to_chat_completion_tool()).collect())
    }

    async fn execute_tool_call(&self, name: &str, arguments: &str) -> String {
        for tool in &self.tools {
            if tool.name() == name {
                return tool.execute(arguments).await;
            }
        }
        format!("Unknown tool: {name}")
    }

    async fn send_chat(
        &self,
        history: &[ChatMessage],
        prompt: &str,
        channel_id: u64,
        image_urls: &[String],
    ) -> Result<String, AppError> {
        let image_hint = if image_urls.is_empty() {
            String::new()
        } else {
            "\n\nThe user has shared images. Be sure to acknowledge and react to them. \
             If they're funny, roast them or add witty commentary. Don't just describe \
             the image — riff on it like you're in a group chat with friends."
                .to_string()
        };
        let system = format!(
            "{}\n\nCurrent Discord channel ID: {}{}",
            self.config.system_prompt, channel_id, image_hint
        );
        let mut messages = build_api_messages(&system, history, prompt, image_urls);
        let tools = self.tool_definitions();

        for _ in 0..MAX_TOOL_ROUNDS {
            let mut request_builder = CreateChatCompletionRequestArgs::default();
            request_builder
                .model(self.config.model.clone())
                .max_completion_tokens(self.config.max_tokens)
                .messages(messages.clone());

            if let Some(ref t) = tools {
                request_builder.tools(t.clone());
            }

            let request = request_builder.build()?;
            let response = self.client.chat().create(request).await?;

            let choice = response.choices.first().ok_or(AppError::NoResponseContent)?;

            if choice.finish_reason == Some(FinishReason::ToolCalls) {
                if let Some(ref tool_calls) = choice.message.tool_calls {
                    messages.push(
                        ChatCompletionRequestAssistantMessageArgs::default()
                            .tool_calls(tool_calls.clone())
                            .build()?
                            .into(),
                    );

                    for call in tool_calls {
                        let result = self
                            .execute_tool_call(&call.function.name, &call.function.arguments)
                            .await;
                        messages.push(
                            ChatCompletionRequestToolMessageArgs::default()
                                .tool_call_id(&call.id)
                                .content(result)
                                .build()?
                                .into(),
                        );
                    }
                    continue;
                }
            }

            return Ok(choice
                .message
                .content
                .as_deref()
                .unwrap_or("")
                .to_string());
        }

        Err(AppError::NoResponseContent)
    }
}

fn build_user_content(text: &str, image_urls: &[String]) -> ChatCompletionRequestUserMessageContent {
    if image_urls.is_empty() {
        return ChatCompletionRequestUserMessageContent::Text(text.to_string());
    }

    let mut parts: Vec<ChatCompletionRequestUserMessageContentPart> = Vec::new();

    parts.push(ChatCompletionRequestUserMessageContentPart::Text(
        ChatCompletionRequestMessageContentPartTextArgs::default()
            .text(text)
            .build()
            .unwrap(),
    ));

    for url in image_urls {
        parts.push(ChatCompletionRequestUserMessageContentPart::ImageUrl(
            ChatCompletionRequestMessageContentPartImageArgs::default()
                .image_url(ImageUrlArgs::default().url(url).build().unwrap())
                .build()
                .unwrap(),
        ));
    }

    ChatCompletionRequestUserMessageContent::Array(parts)
}

fn build_api_messages(
    system_prompt: &str,
    history: &[ChatMessage],
    new_prompt: &str,
    image_urls: &[String],
) -> Vec<ChatCompletionRequestMessage> {
    let mut result: Vec<ChatCompletionRequestMessage> = Vec::new();

    result.push(
        ChatCompletionRequestSystemMessageArgs::default()
            .content(system_prompt)
            .build()
            .unwrap()
            .into(),
    );

    for msg in history {
        result.push(match msg.role {
            MessageRole::User => ChatCompletionRequestUserMessageArgs::default()
                .content(build_user_content(&msg.content, &msg.image_urls))
                .build()
                .unwrap()
                .into(),
            MessageRole::Assistant => ChatCompletionRequestAssistantMessageArgs::default()
                .content(msg.content.as_str())
                .build()
                .unwrap()
                .into(),
        });
    }

    result.push(
        ChatCompletionRequestUserMessageArgs::default()
            .content(build_user_content(new_prompt, image_urls))
            .build()
            .unwrap()
            .into(),
    );

    result
}
