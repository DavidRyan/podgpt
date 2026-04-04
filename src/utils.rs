pub const MAX_MESSAGE_LENGTH: usize = 1900;

pub fn split_message(message: &str) -> Vec<String> {
    if message.len() <= MAX_MESSAGE_LENGTH {
        vec![message.to_string()]
    } else {
        let mut result = Vec::new();
        let mut start = 0;

        while start < message.len() {
            let end = (start + MAX_MESSAGE_LENGTH).min(message.len());
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

pub async fn say(ctx: &crate::bot::Context<'_>, message: String) -> Result<(), crate::bot::Error> {
    for part in split_message(&message) {
        ctx.say(part).await?;
    }
    Ok(())
}
