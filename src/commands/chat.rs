use serenity::all::CommandInteraction;

use crate::gpt::Gpt;


pub async fn chat(gpt: &Gpt, command: &CommandInteraction) {
    let prompt = command.data.options[0].value.as_str().unwrap().to_string();
    let _result = gpt.create_chat(prompt.to_string()).await.unwrap();
}
