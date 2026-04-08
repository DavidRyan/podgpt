use poise::CreateReply;
use serenity::all::CreateAttachment;

use crate::bot::{Context, Error};
use crate::services::image::GeneratedImage;

#[poise::command(slash_command, prefix_command)]
pub async fn image(
    ctx: Context<'_>,
    #[description = "Describe the image you want to generate"]
    #[rest]
    prompt: String,
) -> Result<(), Error> {
    ctx.defer().await?;
    let result = ctx.data().image.generate(&prompt).await?;

    match result {
        GeneratedImage::Url(url) => {
            ctx.say(url).await?;
        }
        GeneratedImage::Bytes(bytes) => {
            let attachment = CreateAttachment::bytes(bytes, "image.png");
            ctx.send(CreateReply::default().attachment(attachment)).await?;
        }
    }

    Ok(())
}
