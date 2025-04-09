use serenity_commands::Commands;

#[derive(Debug, Commands)]
pub enum AllCommands {
    /// Chat with the bot
    Chat,
    /// Generate an image
    Image,
}
