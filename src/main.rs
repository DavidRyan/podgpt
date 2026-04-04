mod bot;
mod commands;
mod config;
mod services;
mod utils;

use bot::run_discord_bot;

#[tokio::main]
async fn main() {
    run_discord_bot().await;
}
