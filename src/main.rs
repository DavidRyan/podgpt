mod bot;
mod commands;
mod gpt;
mod services;

use bot::run_discord_bot;

#[tokio::main]
async fn main() {
    run_discord_bot().await;
}
