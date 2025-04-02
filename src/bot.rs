use crate::gpt::Gpt;

use std::env;
use poise::serenity_prelude as serenity;
use tokio::sync::Mutex;

struct Data {
    gpt: Mutex<Gpt>
} // User data, which is stored and accessible in all command invocations
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;


#[poise::command(slash_command, prefix_command)]
async fn ask(
    ctx: Context<'_>,
    #[description = "Ask Chat Gpt"] #[rest] prompt: String,
) -> Result<(), Error> {
    ctx.defer().await?;
    let r = ctx.data().gpt.lock().await.create_chat(prompt).await.unwrap();
    println!("Response: {}", r);
    ctx.say(r).await?;
    Ok(())
}

#[poise::command(slash_command, prefix_command)]
async fn reply(
    ctx: Context<'_>,
    #[description = "Reply to Chat Gpt"] #[rest] prompt: String,
) -> Result<(), Error> {
    ctx.defer().await?;
    let r = ctx.data().gpt.lock().await.reply_to_chat(prompt).await.unwrap();
    println!("Response: {}", r);
    ctx.say(r).await?;
    Ok(())
}

#[poise::command(prefix_command)]
pub async fn register(ctx: Context<'_>) -> Result<(), Error> {
    poise::builtins::register_application_commands_buttons(ctx).await?;
    Ok(())
}

pub async fn run_discord_bot() {
    // Login with a bot token from the environment
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    let intents = serenity::GatewayIntents::non_privileged();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![ask(), reply(), register()],
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            println!("Bot is ready!");
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {
                    gpt: Mutex::new(Gpt::new())
                })
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;
    client.unwrap().start().await.unwrap();    
}
    

