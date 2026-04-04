use crate::bot::{Context, Error};
use ::serenity::all::Attachment;

#[poise::command(slash_command, prefix_command)]
pub async fn image(
    ctx: Context<'_>,
    #[description = "Prompt for DALL-E"]
    #[rest]
    prompt: String,
) -> Result<(), Error> {
    ctx.defer().await?;
    let url = ctx.data().image.generate(&prompt).await?;
    ctx.say(format!("Image generated: {}", url)).await?;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn image_prompt(
    ctx: Context<'_>,
    #[description = "Your message text"] text: String,
    #[description = "Image to upload"] image: Attachment,
) -> Result<(), Error> {
    ctx.defer().await?;
    let image_bytes = image.download().await?;
    let url = ctx.data().image.edit(image_bytes, &text).await?;
    ctx.say(format!("Image edited: {}", url)).await?;
    Ok(())
}
