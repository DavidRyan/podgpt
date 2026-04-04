use crate::bot::{say, Context, Error};
use crate::services::conversation::MessageRole;
use crate::utils::truncate;

#[poise::command(slash_command, prefix_command)]
pub async fn clear(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer().await?;
    let user_id = ctx.author().id.to_string();

    if ctx.data().chat.clear(&user_id) {
        ctx.say("Conversation cleared.").await?;
    } else {
        ctx.say("No active conversation to clear.").await?;
    }
    Ok(())
}

#[poise::command(slash_command, prefix_command)]
pub async fn history(
    ctx: Context<'_>,
    #[description = "Number of recent messages to show (default: 6)"] count: Option<usize>,
) -> Result<(), Error> {
    ctx.defer().await?;
    let user_id = ctx.author().id.to_string();
    let count = count.unwrap_or(6);

    let result = match ctx.data().chat.get_history(&user_id, count).await {
        Some(r) => r,
        None => {
            ctx.say("No active conversation. Use /ask to start one.")
                .await?;
            return Ok(());
        }
    };

    if result.messages.is_empty() {
        ctx.say("Conversation is empty.").await?;
        return Ok(());
    }

    let mut output = format!(
        "**Conversation history** (showing {} of {} messages):\n\n",
        result.messages.len(),
        result.total
    );

    for (i, msg) in result.messages.iter().enumerate() {
        let role = match msg.role {
            MessageRole::User => "You",
            MessageRole::Assistant => "GPT",
        };
        output.push_str(&format!("**{}:** {}\n", role, truncate(&msg.content, 100)));
        if i < result.messages.len() - 1 {
            output.push('\n');
        }
    }

    say(&ctx, output).await?;
    Ok(())
}

#[poise::command(slash_command, prefix_command)]
pub async fn conversations(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer().await?;
    let user_id = ctx.author().id.to_string();

    match ctx.data().chat.get_conversation_summary(&user_id).await {
        Some(summary) => {
            ctx.say(format!("**Your conversation:** {}", summary))
                .await?;
        }
        None => {
            ctx.say("You have no active conversation.").await?;
        }
    }
    Ok(())
}
