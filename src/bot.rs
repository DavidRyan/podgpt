use crate::gpt::Gpt;
use crate::gpt::ImagePrompt;
use crate::gpt::ChatPrompt;

use poise::serenity_prelude as serenity;
use std::env;
use tokio::sync::Mutex;

struct Data {
    gpt: Mutex<Gpt>,
}
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

#[poise::command(slash_command, prefix_command)]
async fn image_prompt(
    ctx: Context<'_>,
    #[description = "Ask Chat Gpt"]
    #[rest]
    prompt: String,
) -> Result<(), Error> {
    ctx.defer().await?;
    let r = ctx
        .data()
        .gpt
        .lock()
        .await
        .image_prompt(prompt)
        .await
        .unwrap();
    say(&ctx, r).await?;
    Ok(())
}

#[poise::command(slash_command, prefix_command)]
async fn ask(
    ctx: Context<'_>,
    #[description = "Ask Chat Gpt"]
    #[rest]
    prompt: String,
) -> Result<(), Error> {
    println!("Prompt: {}", prompt);
    ctx.defer().await?;
    let r = ctx
        .data()
        .gpt
        .lock()
        .await
        .create_chat(prompt)
        .await
        .unwrap();
    println!("Response: {}", r);
    say(&ctx, r).await?;
    Ok(())
}

#[poise::command(slash_command, prefix_command)]
async fn reply(
    ctx: Context<'_>,
    #[description = "Reply to Chat Gpt"]
    #[rest]
    prompt: String,
) -> Result<(), Error> {
    ctx.defer().await?;
    let r = ctx
        .data()
        .gpt
        .lock()
        .await
        .reply_to_chat(prompt)
        .await
        .unwrap();
    println!("Response: {}", r);
    say(&ctx, r).await?;
    Ok(())
}


fn split_message(message: &str) -> Vec<String> {
    const MAX_LENGTH: usize = 1900;
    if message.len() <= MAX_LENGTH {
        vec![message.to_string()]
    } else {
        let mut result = Vec::new();
        let mut start = 0;

        while start < message.len() {
            let end = (start + MAX_LENGTH).min(message.len());
            let split_point = message[start..end]
                .rfind('\n')
                .map(|pos| start + pos + 1)
                .unwrap_or(end);

            result.push(message[start..split_point].to_string());
            start = split_point;
        }

        result
    }
}

async fn say(ctx: &Context<'_>, r: String) -> Result<(), Error> {
    let split = split_message(&r);
    println!("Split: {:?}", split);
    for m in split.iter() {
        println!("Response: {}", m);
        ctx.say(m).await?;
    }
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
            commands: vec![ask(), reply(), register(), image_prompt()],
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            println!("Bot is ready!");
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {
                    gpt: Mutex::new(Gpt::new()),
                })
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;
    client.unwrap().start().await.unwrap();
}
