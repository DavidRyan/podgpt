use std::future::Future;
use std::pin::Pin;

use super::tools::Tool;

const MAX_CONTENT_LENGTH: usize = 6000;
const MAX_INLINE_LENGTH: usize = 500;

/// Shared HTTP client for URL fetching, used by both the tool and channel summary.
static CLIENT: std::sync::LazyLock<reqwest::Client> = std::sync::LazyLock::new(|| {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_default()
});

/// Fetch a URL and return its text content. Used by UrlReaderTool and ChannelSummaryTool.
pub async fn fetch_url(url: &str) -> Result<String, String> {
    if let Some(api_url) = to_fxtwitter_url(url) {
        return fetch_tweet(&api_url).await;
    }

    let res = CLIENT
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

    truncate_text(&text, MAX_CONTENT_LENGTH)
}

/// Fetch and return a short preview of a URL, for inline expansion in channel summaries.
/// Includes the source domain for context.
pub async fn fetch_url_preview(url: &str) -> String {
    let domain = url
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap_or("unknown");

    match fetch_url(url).await {
        Ok(content) => {
            let preview = truncate_text(&content, MAX_INLINE_LENGTH).unwrap_or(content);
            format!("[Link from {domain}: {preview}]")
        }
        Err(_) => format!("[Link from {domain}: could not load]"),
    }
}

async fn fetch_tweet(api_url: &str) -> Result<String, String> {
    let res = CLIENT
        .get(api_url)
        .header("User-Agent", "PodGPT-Bot/1.0")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch tweet: {e}"))?;

    if !res.status().is_success() {
        return Err(format!("HTTP {}", res.status()));
    }

    let body: serde_json::Value = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse tweet JSON: {e}"))?;

    let tweet = body
        .get("tweet")
        .filter(|t| !t.is_null())
        .ok_or_else(|| "Tweet not found.".to_string())?;

    let author = tweet["author"]["name"].as_str().unwrap_or("Unknown");
    let handle = tweet["author"]["screen_name"].as_str().unwrap_or("unknown");
    let text = tweet["text"].as_str().unwrap_or("");
    let likes = tweet["likes"].as_u64().unwrap_or(0);
    let retweets = tweet["retweets"].as_u64().unwrap_or(0);
    let replies = tweet["replies"].as_u64().unwrap_or(0);
    let created = tweet["created_at"].as_str().unwrap_or("");

    let mut output = format!(
        "@{handle} ({author})\n{text}\n\n\
         Likes: {likes} | Retweets: {retweets} | Replies: {replies}"
    );

    if !created.is_empty() {
        output.push_str(&format!("\nPosted: {created}"));
    }

    if let Some(quote) = tweet.get("quote").filter(|q| !q.is_null()) {
        let q_author = quote["author"]["screen_name"].as_str().unwrap_or("unknown");
        let q_text = quote["text"].as_str().unwrap_or("");
        output.push_str(&format!("\n\nQuoting @{q_author}: {q_text}"));
    }

    Ok(output)
}

fn truncate_text(text: &str, max: usize) -> Result<String, String> {
    if text.len() > max {
        let mut end = max;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        Ok(format!("{}...", &text[..end]))
    } else {
        Ok(text.to_string())
    }
}

/// Extract URLs from a message string.
pub fn extract_urls(text: &str) -> Vec<String> {
    text.split_whitespace()
        .filter(|word| word.starts_with("http://") || word.starts_with("https://"))
        .map(|url| url.trim_end_matches(|c: char| matches!(c, '>' | ')' | ']' | ',' | '.')))
        .map(|s| s.to_string())
        .collect()
}

// --- Tool implementation ---

pub struct UrlReaderTool;

impl UrlReaderTool {
    pub fn new() -> Self {
        Self
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
                    match fetch_url(&u).await {
                        Ok(content) => content,
                        Err(e) => format!("Error reading URL: {e}"),
                    }
                }
                None => "Invalid arguments: missing 'url' field.".to_string(),
            }
        })
    }
}

// --- Helper functions ---

pub fn to_fxtwitter_url(url: &str) -> Option<String> {
    let url = url.trim();
    let path = url
        .strip_prefix("https://twitter.com/")
        .or_else(|| url.strip_prefix("https://www.twitter.com/"))
        .or_else(|| url.strip_prefix("https://mobile.twitter.com/"))
        .or_else(|| url.strip_prefix("https://x.com/"))
        .or_else(|| url.strip_prefix("https://www.x.com/"))
        .or_else(|| url.strip_prefix("http://twitter.com/"))
        .or_else(|| url.strip_prefix("http://x.com/"))?;

    if path.contains("/status/") {
        Some(format!("https://api.fxtwitter.com/{path}"))
    } else {
        None
    }
}

fn strip_html(html: &str) -> String {
    let mut text = html.to_string();

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

    let result = result
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ");

    result.split_whitespace().collect::<Vec<_>>().join(" ")
}
