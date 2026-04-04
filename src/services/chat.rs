use std::sync::Arc;

use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
        ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
        CreateChatCompletionRequestArgs,
    },
    Client,
};
use dashmap::DashMap;
use tokio::sync::Mutex;

use super::conversation::{ChatMessage, Conversation, MessageRole};
use crate::config::Config;
use crate::error::AppError;

pub struct HistoryResult {
    pub messages: Vec<ChatMessage>,
    pub total: usize,
}

pub struct ChatService {
    conversations: DashMap<String, Arc<Mutex<Conversation>>>,
    client: Client<OpenAIConfig>,
    config: Arc<Config>,
}

impl ChatService {
    pub fn new(client: Client<OpenAIConfig>, config: Arc<Config>) -> Self {
        Self {
            conversations: DashMap::new(),
            client,
            config,
        }
    }

    pub async fn create(&self, user_id: &str, prompt: &str) -> Result<String, AppError> {
        let response = send_chat(&self.client, &self.config, &[], prompt).await?;

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

    pub async fn reply(&self, user_id: &str, prompt: &str) -> Result<String, AppError> {
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
        let response = send_chat(&self.client, &self.config, &conv.messages[start..], prompt).await?;

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

    /// Returns (user_id, summary) for the calling user only.
    pub async fn get_conversation_summary(&self, user_id: &str) -> Option<String> {
        let conv_mutex = {
            let entry = self.conversations.get(user_id)?;
            Arc::clone(entry.value())
        };

        let conv = conv_mutex.lock().await;
        Some(conv.summary())
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

async fn send_chat(
    client: &Client<OpenAIConfig>,
    config: &Config,
    history: &[ChatMessage],
    prompt: &str,
) -> Result<String, AppError> {
    let messages = build_api_messages(&config.system_prompt, history, prompt);

    let request = CreateChatCompletionRequestArgs::default()
        .max_tokens(config.max_tokens)
        .model(config.model.clone())
        .messages(messages)
        .build()?;

    let response = client.chat().create(request).await?;

    response
        .choices
        .first()
        .and_then(|c| c.message.content.as_deref())
        .map(|s| s.to_string())
        .ok_or(AppError::NoResponseContent)
}
