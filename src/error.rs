use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Discord error: {0}")]
    Discord(#[from] serenity::Error),

    #[error("OpenAI error: {0}")]
    OpenAi(#[from] async_openai::error::OpenAIError),

    #[error("Image error: {0}")]
    Image(String),

    #[error("No active conversation. Use /ask first.")]
    NoConversation,

    #[error("No response content from GPT")]
    NoResponseContent,

    #[error("No image URL in response")]
    NoImageUrl,
}
