use crate::bot::{say, Context, Error};

#[poise::command(slash_command, prefix_command)]
pub async fn ask(
    ctx: Context<'_>,
    #[description = "Ask ChatGPT a question (starts new conversation)"]
    #[rest]
    prompt: String,
) -> Result<(), Error> {
    ctx.defer().await?;
    let user_id = ctx.author().id.to_string();
    let response = ctx.data().chat.create(&user_id, &prompt).await?;
    say(&ctx, response).await?;
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
    let response = ctx.data().chat.reply(&user_id, &prompt).await?;
    say(&ctx, response).await?;
    Ok(())
}
