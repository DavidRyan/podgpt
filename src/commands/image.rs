use crate::bot::Context;
use crate::bot::Error;
use crate::utils::say;
use ::serenity::all::Attachment;

#[poise::command(slash_command, prefix_command)]
pub async fn image(
    ctx: Context<'_>,
    #[description = "Prompt for DALL·E"]
    #[rest]
    prompt: String,
) -> Result<(), Error> {
    ctx.defer().await?;
    let url = ctx.data().image_generator.generate(&prompt).await?;
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

    let r = crate::services::image::ImageService::edit_image(image_bytes, text).await?;

    say(&ctx, r).await?;
    Ok(())
}
