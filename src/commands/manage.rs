use crate::bot::Context;
use crate::bot::Error;
use crate::services::conversation::MessageRole;
use crate::utils::say;

#[poise::command(slash_command, prefix_command)]
pub async fn clear(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer().await?;

    let user_id = ctx.author().id.to_string();
    let mut gpt = ctx.data().gpt.lock().await;

    if gpt.get_conversation(&user_id).is_some() {
        gpt.remove_conversation(&user_id);
        ctx.say("Conversation cleared.").await?;
    } else {
        ctx.say("No active conversation to clear.").await?;
    }

    Ok(())
}

#[poise::command(slash_command, prefix_command)]
pub async fn history(
    ctx: Context<'_>,
    #[description = "Number of recent messages to show (default: 6)"]
    count: Option<usize>,
) -> Result<(), Error> {
    ctx.defer().await?;

    let user_id = ctx.author().id.to_string();
    let gpt = ctx.data().gpt.lock().await;

    let conversation = match gpt.get_conversation(&user_id) {
        Some(c) => c,
        None => {
            ctx.say("No active conversation. Use /ask to start one.").await?;
            return Ok(());
        }
    };

    let count = count.unwrap_or(6);
    let total = conversation.message_count();
    let start = if total > count { total - count } else { 0 };
    let recent_messages = &conversation.messages[start..];

    if recent_messages.is_empty() {
        ctx.say("Conversation is empty.").await?;
        return Ok(());
    }

    let mut output = format!("**Conversation history** (showing {} of {} messages):\n\n", recent_messages.len(), total);

    for (i, msg) in recent_messages.iter().enumerate() {
        let role = match msg.role {
            MessageRole::User => "You",
            MessageRole::Assistant => "GPT",
        };
        let preview = if msg.content.len() > 100 {
            format!("{}...", &msg.content[..100])
        } else {
            msg.content.clone()
        };
        output.push_str(&format!("**{}:** {}\n", role, preview));

        if i < recent_messages.len() - 1 {
            output.push('\n');
        }
    }

    say(&ctx, output).await?;
    Ok(())
}

#[poise::command(slash_command, prefix_command)]
pub async fn conversations(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer().await?;

    let gpt = ctx.data().gpt.lock().await;
    let all_convos = gpt.list_conversations();

    if all_convos.is_empty() {
        ctx.say("No active conversations.").await?;
        return Ok(());
    }

    let mut output = String::from("**Active conversations:**\n\n");

    for (i, convo) in all_convos.iter().enumerate() {
        output.push_str(&format!(
            "{}. User <{}>: {}\n",
            i + 1,
            convo.user_id,
            convo.summary()
        ));
    }

    say(&ctx, output).await?;
    Ok(())
}
