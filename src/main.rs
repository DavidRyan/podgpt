mod bot;
mod commands;
mod config;
mod error;
mod services;
mod utils;

use bot::run_discord_bot;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    run_discord_bot().await;
}
