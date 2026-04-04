use crate::bot::Context;
use crate::bot::Error;
use crate::utils::say;

#[poise::command(slash_command, prefix_command)]
pub async fn ask(
    ctx: Context<'_>,
    #[description = "Ask ChatGPT a question (starts new conversation)"]
    #[rest]
    prompt: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let user_id = ctx.author().id.to_string();

    let r = ctx
        .data()
        .gpt
        .lock()
        .await
        .create_chat(user_id, prompt)
        .await?;

    say(&ctx, r).await?;
    Ok(())
}

#[poise::command(slash_command, prefix_command)]
pub async fn reply(
    ctx: Context<'_>,
    #[description = "Reply to continue your conversation with GPT"]
    #[rest]
    prompt: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let user_id = ctx.author().id.to_string();

    let r = ctx
        .data()
        .gpt
        .lock()
        .await
        .reply_to_chat(user_id, prompt)
        .await?;

    say(&ctx, r).await?;
    Ok(())
}
