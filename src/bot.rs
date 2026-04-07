use std::sync::Arc;

use async_openai::{config::OpenAIConfig, Client};
use poise::serenity_prelude as serenity;

use crate::config::Config;
use crate::error::AppError;
use crate::services::channel_summary::ChannelSummaryTool;
use crate::services::chat::ChatService;
use crate::services::image::ImageService;
use crate::services::url_reader::UrlReaderTool;
use crate::utils::split_message;

pub struct Data {
    pub chat: ChatService,
    pub image: ImageService,
}

pub type Error = AppError;
pub type Context<'a> = poise::Context<'a, Data, Error>;

pub async fn say(ctx: &Context<'_>, message: String) -> Result<(), Error> {
    for part in split_message(&message) {
        ctx.say(part).await?;
    }
    Ok(())
}

pub fn all_commands() -> Vec<poise::Command<Data, Error>> {
    vec![
        crate::commands::chat::ask(),
        crate::commands::chat::reply(),
        crate::commands::manage::clear(),
        crate::commands::manage::history(),
        crate::commands::manage::conversations(),
        crate::commands::image::image(),
        crate::commands::image::image_prompt(),
        crate::commands::roast::roast(),
    ]
}

async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    match error {
        poise::FrameworkError::Command { error, ctx, .. } => {
            tracing::error!("Command error: {error}");
            let _ = ctx.say(format!("Something went wrong: {error}")).await;
        }
        other => {
            if let Err(e) = poise::builtins::on_error(other).await {
                tracing::error!("Unhandled error: {e}");
            }
        }
    }
}

pub async fn run_discord_bot() {
    let config = Arc::new(Config::from_env());
    let token = config.discord_token.clone();

    let openai_config = OpenAIConfig::new().with_api_key(&config.openai_api_key);
    let openai_client = Client::with_config(openai_config);
    let image = ImageService::new(openai_client.clone());

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: all_commands(),
            on_error: |error| Box::pin(on_error(error)),
            ..Default::default()
        })
        .setup(move |ctx, _ready, framework| {
            tracing::info!("Bot is ready!");
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;

                let mut chat = ChatService::new(openai_client, config);
                chat.register_tool(UrlReaderTool::new());
                chat.register_tool(ChannelSummaryTool::new(Arc::clone(&ctx.http)));

                Ok(Data { chat, image })
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(&token, serenity::GatewayIntents::non_privileged())
        .framework(framework)
        .await;
    client.unwrap().start().await.unwrap();
}
