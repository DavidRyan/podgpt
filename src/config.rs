use std::env;

pub struct Config {
    pub discord_token: String,
    pub openai_api_key: String,
    pub model: String,
    pub max_tokens: u32,
    pub max_history_messages: usize,
    pub system_prompt: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            discord_token: env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN must be set"),
            openai_api_key: env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set"),
            model: env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o".to_string()),
            max_tokens: env::var("MAX_TOKENS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(2048),
            max_history_messages: 50,
            system_prompt: env::var("SYSTEM_PROMPT").unwrap_or_else(|_| {
                "You are a helpful AI assistant in a Discord server. Keep responses concise."
                    .to_string()
            }),
        }
    }
}
