use std::future::Future;
use std::pin::Pin;

use super::tools::Tool;

const MAX_CONTENT_LENGTH: usize = 6000;

pub struct UrlReaderTool {
    client: reqwest::Client,
}

impl UrlReaderTool {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
        }
    }

    async fn fetch(&self, url: &str) -> Result<String, String> {
        let res = self
            .client
            .get(url)
            .header("User-Agent", "PodGPT-Bot/1.0")
            .send()
            .await
            .map_err(|e| format!("Failed to fetch URL: {e}"))?;

        if !res.status().is_success() {
            return Err(format!("HTTP {}", res.status()));
        }

        let content_type = res
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let body = res
            .text()
            .await
            .map_err(|e| format!("Failed to read response: {e}"))?;

        let text = if content_type.contains("text/html") {
            strip_html(&body)
        } else {
            body
        };

        if text.len() > MAX_CONTENT_LENGTH {
            let mut end = MAX_CONTENT_LENGTH;
            while !text.is_char_boundary(end) {
                end -= 1;
            }
            Ok(format!("{}...\n\n[Truncated — showing first {MAX_CONTENT_LENGTH} characters]", &text[..end]))
        } else {
            Ok(text)
        }
    }
}

impl Tool for UrlReaderTool {
    fn name(&self) -> &str {
        "read_url"
    }

    fn description(&self) -> &str {
        "Fetch and read the text content of a web page or URL. Use when the user shares a link \
         or asks about content at a specific URL."
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The full URL to fetch (must include https://)"
                }
            },
            "required": ["url"],
            "additionalProperties": false
        })
    }

    fn execute(&self, arguments: &str) -> Pin<Box<dyn Future<Output = String> + Send + '_>> {
        let args = arguments.to_string();
        Box::pin(async move {
            let url = serde_json::from_str::<serde_json::Value>(&args)
                .ok()
                .and_then(|v| v["url"].as_str().map(|s| s.to_string()));

            match url {
                Some(u) => {
                    tracing::info!(url = %u, "Fetching URL");
                    match self.fetch(&u).await {
                        Ok(content) => content,
                        Err(e) => format!("Error reading URL: {e}"),
                    }
                }
                None => "Invalid arguments: missing 'url' field.".to_string(),
            }
        })
    }
}

fn strip_html(html: &str) -> String {
    let mut text = html.to_string();

    // Remove script and style blocks
    for tag in &["script", "style", "noscript"] {
        loop {
            let lower = text.to_lowercase();
            let open = format!("<{}", tag);
            let close = format!("</{}>", tag);
            if let Some(start) = lower.find(&open) {
                if let Some(end_offset) = lower[start..].find(&close) {
                    text.replace_range(start..start + end_offset + close.len(), " ");
                    continue;
                }
            }
            break;
        }
    }

    // Strip remaining HTML tags
    let mut result = String::with_capacity(text.len());
    let mut in_tag = false;
    for ch in text.chars() {
        match ch {
            '<' => in_tag = true,
            '>' if in_tag => {
                in_tag = false;
                result.push(' ');
            }
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }

    // Decode common HTML entities
    let result = result
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ");

    // Collapse whitespace
    result.split_whitespace().collect::<Vec<_>>().join(" ")
}
