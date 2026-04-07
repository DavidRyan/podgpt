use std::sync::Arc;

use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
        ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestToolMessageArgs,
        ChatCompletionRequestUserMessageArgs, ChatCompletionTool,
        CreateChatCompletionRequestArgs, FinishReason,
    },
    Client,
};
use dashmap::DashMap;
use tokio::sync::Mutex;

use super::conversation::{ChatMessage, Conversation, MessageRole};
use super::search::SearchTool;
use super::tools::Tool;
use crate::config::Config;
use crate::error::AppError;

const MAX_TOOL_ROUNDS: usize = 5;

pub struct HistoryResult {
    pub messages: Vec<ChatMessage>,
    pub total: usize,
}

pub struct ChatService {
    conversations: DashMap<String, Arc<Mutex<Conversation>>>,
    client: Client<OpenAIConfig>,
    config: Arc<Config>,
    tools: Vec<Box<dyn Tool>>,
}

impl ChatService {
    pub fn new(client: Client<OpenAIConfig>, config: Arc<Config>) -> Self {
        let mut service = Self {
            conversations: DashMap::new(),
            client,
            config: Arc::clone(&config),
            tools: Vec::new(),
        };

        if let Some(ref key) = config.tavily_api_key {
            service.register_tool(SearchTool::new(key.clone()));
        }

        service
    }

    /// Register a tool that GPT can invoke during conversations.
    pub fn register_tool(&mut self, tool: impl Tool + 'static) {
        self.tools.push(Box::new(tool));
    }

    pub async fn create(&self, user_id: &str, prompt: &str, channel_id: u64) -> Result<String, AppError> {
        let response = self.send_chat(&[], prompt, channel_id).await?;

        let mut conversation = Conversation::new();
        conversation.messages.push(ChatMessage {
            role: MessageRole::User,
            content: prompt.to_string(),
        });
        conversation.messages.push(ChatMessage {
            role: MessageRole::Assistant,
            content: response.clone(),
        });

        self.conversations
            .insert(user_id.to_string(), Arc::new(Mutex::new(conversation)));

        Ok(response)
    }

    pub async fn reply(&self, user_id: &str, prompt: &str, channel_id: u64) -> Result<String, AppError> {
        let conv_mutex = {
            let entry = self
                .conversations
                .get(user_id)
                .ok_or(AppError::NoConversation)?;
            Arc::clone(entry.value())
        };

        let mut conv = conv_mutex.lock().await;

        let max_history = self.config.max_history_messages;
        let start = conv.messages.len().saturating_sub(max_history);
        let response = self.send_chat(&conv.messages[start..], prompt, channel_id).await?;

        conv.messages.push(ChatMessage {
            role: MessageRole::User,
            content: prompt.to_string(),
        });
        conv.messages.push(ChatMessage {
            role: MessageRole::Assistant,
            content: response.clone(),
        });

        let max = max_history * 2;
        if conv.messages.len() > max {
            let keep = conv.messages.len() - max_history;
            conv.messages.drain(0..keep);
        }

        Ok(response)
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
            "Based on the following recent Discord messages, write a funny, witty roast of the user \
             \"{target}\". Focus on what they've said, how they talk, their interests, and their \
             behavior in chat. Keep it lighthearted and playful — nothing genuinely cruel. \
             If you can't find messages from them, roast them for being a lurker.\n\n\
             Messages:\n{channel_messages}"
        );

        let system = "You are a comedian performing a roast at a friend's gathering. \
            Be funny, creative, and savage — but always in good fun. \
            Keep it to 2-3 short paragraphs max.";

        let messages = build_api_messages(system, &[], &prompt);

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
    ) -> Result<String, AppError> {
        let system = format!(
            "{}\n\nCurrent Discord channel ID: {}",
            self.config.system_prompt, channel_id
        );
        let mut messages = build_api_messages(&system, history, prompt);
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

            return choice
                .message
                .content
                .as_deref()
                .map(|s| s.to_string())
                .ok_or(AppError::NoResponseContent);
        }

        Err(AppError::NoResponseContent)
    }
}

fn build_api_messages(
    system_prompt: &str,
    history: &[ChatMessage],
    new_prompt: &str,
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
                .content(msg.content.as_str())
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
            .content(new_prompt)
            .build()
            .unwrap()
            .into(),
    );

    result
}
