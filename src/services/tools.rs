use std::future::Future;
use std::pin::Pin;

use async_openai::types::{ChatCompletionTool, ChatCompletionToolType, FunctionObject};

/// Trait for tools that GPT can invoke during a conversation.
///
/// Implement this to add a new tool:
/// 1. Define the tool's name, description, and JSON Schema parameters.
/// 2. Implement `execute` to handle the function call arguments and return a string result.
/// 3. Register the tool in `ChatService::new` via `register_tool`.
pub trait Tool: Send + Sync {
    /// Unique name matching what GPT will call (e.g. "web_search").
    fn name(&self) -> &str;

    /// Human-readable description for GPT's tool-use decision.
    fn description(&self) -> &str;

    /// JSON Schema for the function parameters.
    fn parameters(&self) -> serde_json::Value;

    /// Execute the tool given the raw JSON arguments string from GPT.
    /// Returns a string result that gets sent back to GPT as the tool response.
    fn execute(&self, arguments: &str) -> Pin<Box<dyn Future<Output = String> + Send + '_>>;

    /// Build the OpenAI API tool definition. Default implementation works for all tools.
    fn to_chat_completion_tool(&self) -> ChatCompletionTool {
        ChatCompletionTool {
            r#type: ChatCompletionToolType::Function,
            function: FunctionObject {
                name: self.name().to_string(),
                description: Some(self.description().to_string()),
                parameters: Some(self.parameters()),
                strict: Some(true),
            },
        }
    }
}
