use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestAssistantMessageArgs,
        ChatCompletionRequestMessage,
        ChatCompletionRequestUserMessageArgs,
        CreateChatCompletionRequestArgs,
    },
    Client,
};
use std::collections::HashMap;
use std::error::Error as StdError;

#[derive(Debug, Clone)]
pub enum MessageRole {
    User,
    Assistant,
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct Conversation {
    pub id: String,
    pub messages: Vec<ChatMessage>,
    pub user_id: String,
    pub created_at: std::time::SystemTime,
}

impl Conversation {
    pub fn new(id: String, user_id: String) -> Self {
        Self {
            id,
            messages: Vec::new(),
            user_id,
            created_at: std::time::SystemTime::now(),
        }
    }

    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    pub fn summary(&self) -> String {
        if self.messages.is_empty() {
            return "(empty)".to_string();
        }
        let first_msg = &self.messages[0].content;
        let preview = if first_msg.len() > 50 {
            format!("{}...", &first_msg[..50])
        } else {
            first_msg.clone()
        };
        format!("\"{}\" ({} messages)", preview, self.messages.len())
    }
}

pub struct ConversationManager {
    client: Client<OpenAIConfig>,
    conversations: HashMap<String, Conversation>,
    max_tokens: u32,
    max_history_messages: usize,
}

impl ConversationManager {
    pub fn new() -> Self {
        Self {
            client: Client::with_config(OpenAIConfig::new()),
            conversations: HashMap::new(),
            max_tokens: 2048,
            max_history_messages: 50,
        }
    }

    pub fn get_conversation(&self, user_id: &str) -> Option<&Conversation> {
        self.conversations.get(user_id)
    }

    pub fn list_conversations(&self) -> Vec<&Conversation> {
        self.conversations.values().collect()
    }

    pub fn remove_conversation(&mut self, user_id: &str) {
        self.conversations.remove(user_id);
    }

    fn messages_to_api_format(&self, conversation: &Conversation, additional_prompt: Option<&str>) -> Vec<ChatCompletionRequestMessage> {
        let mut messages: Vec<ChatCompletionRequestMessage> = conversation
            .messages
            .iter()
            .map(|msg| match msg.role {
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
            })
            .collect();

        if let Some(prompt) = additional_prompt {
            messages.push(
                ChatCompletionRequestUserMessageArgs::default()
                    .content(prompt)
                    .build()
                    .unwrap()
                    .into(),
            );
        }

        messages
    }

    pub async fn create_chat(
        &mut self,
        user_id: String,
        prompt: String,
    ) -> Result<String, Box<dyn StdError + Send + Sync>> {
        let request = CreateChatCompletionRequestArgs::default()
            .max_tokens(self.max_tokens)
            .model("gpt-4o")
            .messages([ChatCompletionRequestUserMessageArgs::default()
                .content(prompt.as_str())
                .build()?
                .into()])
            .build()?;

        let response = self.client.chat().create(request).await?;

        let msg = response
            .choices
            .first()
            .and_then(|c| c.message.content.as_deref())
            .ok_or("No response content from GPT")?
            .to_string();

        let conversation_id = response.id.clone();

        let mut conversation = Conversation::new(conversation_id, user_id.clone());
        conversation.messages.push(ChatMessage {
            role: MessageRole::User,
            content: prompt,
        });
        conversation.messages.push(ChatMessage {
            role: MessageRole::Assistant,
            content: msg.clone(),
        });

        self.conversations.insert(user_id, conversation);

        Ok(msg)
    }

    pub async fn reply_to_chat(
        &mut self,
        user_id: String,
        prompt: String,
    ) -> Result<String, Box<dyn StdError + Send + Sync>> {
        let conversation = self
            .conversations
            .get(&user_id)
            .ok_or("No active conversation. Use /ask first.")?;

        let recent_messages: Vec<ChatMessage> = if conversation.messages.len() > self.max_history_messages {
            conversation.messages[conversation.messages.len() - self.max_history_messages..].to_vec()
        } else {
            conversation.messages.clone()
        };

        let temp_convo = Conversation {
            id: conversation.id.clone(),
            messages: recent_messages,
            user_id: conversation.user_id.clone(),
            created_at: conversation.created_at,
        };

        let messages = self.messages_to_api_format(&temp_convo, Some(&prompt));

        let request = CreateChatCompletionRequestArgs::default()
            .max_tokens(self.max_tokens)
            .model("gpt-4o")
            .messages(messages)
            .build()?;

        let response = self.client.chat().create(request).await?;

        let msg = response
            .choices
            .first()
            .and_then(|c| c.message.content.as_deref())
            .ok_or("No response content from GPT")?
            .to_string();

        if let Some(convo) = self.conversations.get_mut(&user_id) {
            convo.messages.push(ChatMessage {
                role: MessageRole::User,
                content: prompt,
            });
            convo.messages.push(ChatMessage {
                role: MessageRole::Assistant,
                content: msg.clone(),
            });

            if convo.messages.len() > self.max_history_messages * 2 {
                let keep = convo.messages.len() - self.max_history_messages;
                convo.messages.drain(0..keep);
            }
        }

        Ok(msg)
    }
}
