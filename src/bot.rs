use crate::commands::chat;
//use crate::commands::image;
use crate::commands::all_commands::AllCommands;
use crate::gpt::Gpt;

use std::env;
use serenity::all::{GuildId, Interaction, Ready};
use serenity_commands::Commands;
use serenity::async_trait;
use serenity::builder::{CreateAttachment, CreateEmbed, CreateEmbedFooter, CreateMessage};
use serenity::model::channel::Message;
use serenity::prelude::*;


struct Bot{
    gpt: Gpt,
    guild_id: GuildId
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
            gpt,
            guild_id: GuildId::new(1354958928169013338)
        }).await.expect("Err creating client");

    // Start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}

#[async_trait]
impl EventHandler for Bot {

    async fn ready(&self, ctx: Context, _ready: Ready) {
        println!("{} is connected!", ctx.cache.current_user().name);
        self.guild_id.set_commands(&ctx, AllCommands::create_commands()).await.unwrap();
    }

    async fn interaction_create(&self, _ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            let command_data = AllCommands::from_command_data(&command.data).unwrap();
            let _response = match command_data {
                AllCommands::Chat => chat::chat(&self.gpt, &command).await,
                AllCommands::Image => {
                    "".to_string()
                }
            };

        }
    }

    async fn message(&self, ctx: Context, msg: Message) {


        match &msg.content.as_str() {
            x if x.contains("/chat") => {
                println!("/chat");
            }
            x if x.contains("/image") => {
                println!("/image");

                let result = x.replace("/image", "");
                let path = self.gpt.create_image(result).await.unwrap();
                let p = ["attachment://", &path].join(""); // write path in docker doesn't work'
                let image_message = CreateEmbed::default()
                    .image(p);

                let create_message = CreateMessage::new()
                    .embed(image_message)
                    .add_file(CreateAttachment::path(path).await.unwrap());

                let _ = msg.channel_id.send_message(&ctx.http, create_message).await;
            }
            _ => {
                // no op
            }
        }
    }
}
