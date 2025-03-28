
use crate::gpt::Gpt;
use std::env;

use serenity::async_trait;
use serenity::builder::{CreateAttachment, CreateEmbed, CreateEmbedFooter, CreateMessage};
use serenity::model::channel::Message;
use serenity::prelude::*;

struct Bot{
    gpt: Gpt
}


pub async fn run_discord_bot(gpt: Gpt) {
    // Login with a bot token from the environment
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    // Set gateway intents, which decides what events the bot will be notified about
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    // Create a new instance of the Client, logging in as a bot.
    let mut client =
        Client::builder(&token, intents).event_handler(Bot{
            gpt
        }).await.expect("Err creating client");

    // Start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}

#[async_trait]
impl EventHandler for Bot {
    async fn message(&self, ctx: Context, msg: Message) {


        match &msg.content.as_str() {
            x if x.contains("/chat") => {
                println!("/chat");
                // cut off /chat and send the message to gpt
                let result = self.gpt.create_chat().await.unwrap();
                let result = msg.channel_id.say(&ctx.http, result).await;
            }
            x if x.contains("/image") => {
                println!("/image");
                let image_message = CreateEmbed::default()
                    .image("attachment://image.png");

                let create_message = CreateMessage::new()
                    .embed(image_message)
                    .add_file(CreateAttachment::path("./image.png").await.unwrap());

                let result = msg.channel_id.send_message(&ctx.http, create_message).await;
            }
            _ => {
                // no op
            }
        }
    }
}
