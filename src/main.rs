mod bot;
mod gpt;
mod commands;

use bot::run_discord_bot;
use gpt::Gpt;


#[tokio::main]
async fn main() {
    let gpt = Gpt::new();
    run_discord_bot(gpt).await;
}
