use std::env;

pub fn discord_token() -> String {
    env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN must be set")
}

pub fn openai_api_key() -> String {
    env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set")
}
