use poise::serenity_prelude as serenity;
use tokio::sync::Mutex;

use crate::services::image_generator::ImageGenerator;
use crate::services::conversation::ConversationManager;
use std::sync::Arc;

pub struct Data {
    pub gpt: Mutex<ConversationManager>,
    pub image_generator: Arc<dyn ImageGenerator>,
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;

pub fn all_commands() -> Vec<poise::Command<Data, Error>> {
    vec![
        crate::commands::chat::ask(),
        crate::commands::chat::reply(),
        crate::commands::manage::clear(),
        crate::commands::manage::history(),
        crate::commands::manage::conversations(),
        crate::commands::image::image(),
        crate::commands::image::image_prompt(),
    ]
}

pub async fn run_discord_bot() {
    let token = crate::config::discord_token();
    let intents = serenity::GatewayIntents::non_privileged();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: all_commands(),
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            println!("Bot is ready!");
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                use crate::services::image_generator::OpenAiImageGenerator;
                let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");
                let image_generator: Arc<dyn ImageGenerator> = Arc::new(OpenAiImageGenerator::new(api_key));
                Ok(Data {
                    gpt: Mutex::new(ConversationManager::new()),
                    image_generator,
                })
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;
    client.unwrap().start().await.unwrap();
}
