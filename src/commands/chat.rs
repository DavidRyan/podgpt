use poise::CreateReply;
use serenity::all::{Attachment, CreateAttachment};
use serenity::builder::GetMessages;

use crate::bot::{say, Context, Error};
use crate::services::chat::ChatResponse;

fn image_urls_from_attachment(attachment: &Option<Attachment>) -> Vec<String> {
    attachment
        .iter()
        .filter(|a| {
            a.content_type
                .as_deref()
                .map(|ct| ct.starts_with("image/"))
                .unwrap_or(false)
        })
        .map(|a| a.url.clone())
        .collect()
}

/// Fetch image URLs from the last few messages in the channel so GPT can see
/// recently posted images even when the user doesn't attach one directly.
async fn recent_channel_images(ctx: &Context<'_>, limit: u8) -> Vec<String> {
    let messages = match ctx
        .channel_id()
        .messages(ctx.http(), GetMessages::new().limit(limit))
        .await
    {
        Ok(msgs) => msgs,
        Err(e) => {
            tracing::warn!(
                channel_id = %ctx.channel_id(),
                error = %e,
                "Failed to fetch recent channel messages for images — bot may lack Read Message History permission"
            );
            return Vec::new();
        }
    };

    let urls: Vec<String> = messages
        .iter()
        .flat_map(|msg| &msg.attachments)
        .filter(|a| {
            a.content_type
                .as_deref()
                .map(|ct| ct.starts_with("image/"))
                .unwrap_or(false)
        })
        .map(|a| a.url.clone())
        .collect();

    tracing::info!(
        channel_id = %ctx.channel_id(),
        image_count = urls.len(),
        "Found images in recent channel messages"
    );

    urls
}

/// Send a ChatResponse, attaching any generated images.
async fn send_response(ctx: &Context<'_>, response: ChatResponse) -> Result<(), Error> {
    if response.images.is_empty() {
        say(ctx, response.text).await?;
    } else {
        let text = if response.text.is_empty() {
            "Here you go!".to_string()
        } else {
            response.text
        };
        let mut reply = CreateReply::default().content(&text);
        for (i, bytes) in response.images.iter().enumerate() {
            let filename = if response.images.len() == 1 {
                "image.png".to_string()
            } else {
                format!("image_{}.png", i + 1)
            };
            reply = reply.attachment(CreateAttachment::bytes(bytes.clone(), filename));
        }
        ctx.send(reply).await?;
    }
    Ok(())
}

#[poise::command(slash_command, prefix_command)]
pub async fn ask(
    ctx: Context<'_>,
    #[description = "Attach an image for GPT to analyze"] image: Option<Attachment>,
    #[description = "Ask ChatGPT a question (starts new conversation)"]
    #[rest]
    prompt: String,
) -> Result<(), Error> {
    ctx.defer().await?;
    let user_id = ctx.author().id.to_string();
    let channel_id = ctx.channel_id().get();
    let mut urls = image_urls_from_attachment(&image);
    if urls.is_empty() {
        urls = recent_channel_images(&ctx, 10).await;
    }
    let response = ctx.data().chat.create(&user_id, &prompt, channel_id, urls).await?;
    send_response(&ctx, response).await?;
    Ok(())
}

#[poise::command(slash_command, prefix_command)]
pub async fn reply(
    ctx: Context<'_>,
    #[description = "Attach an image for GPT to analyze"] image: Option<Attachment>,
    #[description = "Reply to continue your conversation with GPT"]
    #[rest]
    prompt: String,
) -> Result<(), Error> {
    ctx.defer().await?;
    let user_id = ctx.author().id.to_string();
    let channel_id = ctx.channel_id().get();
    let mut urls = image_urls_from_attachment(&image);
    if urls.is_empty() {
        urls = recent_channel_images(&ctx, 10).await;
    }
    let response = ctx.data().chat.reply(&user_id, &prompt, channel_id, urls).await?;
    send_response(&ctx, response).await?;
    Ok(())
}
