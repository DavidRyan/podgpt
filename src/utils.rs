pub const MAX_MESSAGE_LENGTH: usize = 1900;

pub fn truncate(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_chars {
        s.to_string()
    } else {
        let byte_end = s
            .char_indices()
            .nth(max_chars)
            .map(|(i, _)| i)
            .unwrap_or(s.len());
        format!("{}...", &s[..byte_end])
    }
}

pub fn split_message(message: &str) -> Vec<String> {
    if message.len() <= MAX_MESSAGE_LENGTH {
        return vec![message.to_string()];
    }

    let mut result = Vec::new();
    let mut start = 0;

    while start < message.len() {
        let remaining = &message[start..];
        if remaining.len() <= MAX_MESSAGE_LENGTH {
            result.push(remaining.to_string());
            break;
        }

        // Find a char boundary at or before start + MAX_MESSAGE_LENGTH
        let mut end = start + MAX_MESSAGE_LENGTH;
        while !message.is_char_boundary(end) {
            end -= 1;
        }

        // Prefer splitting at a newline
        let split_point = message[start..end]
            .rfind('\n')
            .map(|pos| start + pos + 1)
            .unwrap_or(end);

        result.push(message[start..split_point].to_string());
        start = split_point;
    }

    result
}
