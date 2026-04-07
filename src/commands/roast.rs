use crate::bot::{say, Context, Error};
use ::serenity::all::User;

#[poise::command(slash_command, prefix_command)]
pub async fn roast(
    ctx: Context<'_>,
    #[description = "Who to roast"] target: User,
) -> Result<(), Error> {
    ctx.defer().await?;
    let channel_id = ctx.channel_id().get();
    let response = ctx
        .data()
        .chat
        .roast(&target.name, channel_id)
        .await?;
    say(&ctx, format!("**Roasting <@{}>:**\n\n{}", target.id, response)).await?;
    Ok(())
}
