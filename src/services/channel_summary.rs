use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use serenity::builder::GetMessages;
use serenity::http::Http;
use serenity::model::id::ChannelId;

use super::tools::Tool;
use super::url_reader::{extract_urls, fetch_url_preview, to_fxtwitter_url};

pub struct ChannelSummaryTool {
    http: Arc<Http>,
}

impl ChannelSummaryTool {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }

    async fn fetch_messages(&self, channel_id: u64, limit: u8) -> Result<String, String> {
        let channel = ChannelId::new(channel_id);
        let messages = channel
            .messages(&*self.http, GetMessages::new().limit(limit))
            .await
            .map_err(|e| format!("Failed to fetch messages: {e}"))?;

        if messages.is_empty() {
            return Ok("No messages found in this channel.".to_string());
        }

        let mut output = String::new();

        // Messages come newest-first, reverse for chronological order
        for msg in messages.iter().rev() {
            let timestamp = msg.timestamp.format("%Y-%m-%d %H:%M");
            let author = &msg.author.name;
            let content = if msg.content.is_empty() {
                if !msg.attachments.is_empty() {
                    "[attachment]".to_string()
                } else if !msg.embeds.is_empty() {
                    "[embed]".to_string()
                } else {
                    "[no text content]".to_string()
                }
            } else {
                msg.content.clone()
            };

            output.push_str(&format!("[{timestamp}] @{author}: {content}\n"));

            // Inline tweet previews for Twitter/X links
            let tweet_urls: Vec<String> = extract_urls(&content)
                .into_iter()
                .filter(|url| to_fxtwitter_url(url).is_some())
                .collect();

            for url in &tweet_urls {
                let preview = fetch_url_preview(url).await;
                output.push_str(&format!("  ↳ {preview}\n"));
            }
        }

        Ok(output)
    }
}

impl Tool for ChannelSummaryTool {
    fn name(&self) -> &str {
        "channel_summary"
    }

    fn description(&self) -> &str {
        "Read recent messages from a Discord channel to summarize what's been discussed. \
         The current channel ID is provided in the system context. \
         Use when the user asks about recent activity or wants a channel summary."
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "channel_id": {
                    "type": "string",
                    "description": "The Discord channel ID to read messages from"
                },
                "limit": {
                    "type": "integer",
                    "description": "Number of recent messages to fetch (1-50, use 25 as default)"
                }
            },
            "required": ["channel_id", "limit"],
            "additionalProperties": false
        })
    }

    fn execute(&self, arguments: &str) -> Pin<Box<dyn Future<Output = String> + Send + '_>> {
        let args = arguments.to_string();
        Box::pin(async move {
            let parsed = match serde_json::from_str::<serde_json::Value>(&args) {
                Ok(v) => v,
                Err(_) => return "Invalid arguments.".to_string(),
            };

            let channel_id = parsed["channel_id"]
                .as_str()
                .and_then(|s| s.parse::<u64>().ok());

            let limit = parsed["limit"]
                .as_u64()
                .map(|n| n.min(50).max(1) as u8)
                .unwrap_or(25);

            match channel_id {
                Some(id) => {
                    tracing::info!(channel_id = id, limit, "Fetching channel messages");
                    match self.fetch_messages(id, limit).await {
                        Ok(messages) => messages,
                        Err(e) => format!("Error: {e}"),
                    }
                }
                None => "Invalid channel_id.".to_string(),
            }
        })
    }
}
