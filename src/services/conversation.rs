#[derive(Debug, Clone)]
pub enum MessageRole {
    User,
    Assistant,
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
    pub image_urls: Vec<String>,
}

#[derive(Debug)]
pub struct Conversation {
    pub messages: Vec<ChatMessage>,
}

impl Conversation {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }

    pub fn summary(&self) -> String {
        if self.messages.is_empty() {
            return "(empty)".to_string();
        }
        let first_msg = &self.messages[0].content;
        let preview = crate::utils::truncate(first_msg, 50);
        format!("\"{}\" ({} messages)", preview, self.messages.len())
    }
}
