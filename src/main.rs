mod bot;
mod gpt;
mod commands;

use bot::run_discord_bot;


#[tokio::main]
async fn main() {
    run_discord_bot().await;
}

