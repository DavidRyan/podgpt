use super::tools::Tool;

pub struct SearchTool {
    client: reqwest::Client,
    api_key: String,
}

#[derive(serde::Deserialize)]
struct TavilyResponse {
    results: Vec<TavilyResult>,
}

#[derive(serde::Deserialize)]
struct TavilyResult {
    title: String,
    url: String,
    content: String,
}

impl SearchTool {
    pub fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
        }
    }

    async fn search(&self, query: &str) -> Result<String, String> {
        let res = self
            .client
            .post("https://api.tavily.com/search")
            .json(&serde_json::json!({
                "api_key": self.api_key,
                "query": query,
                "max_results": 5,
                "include_answer": false,
            }))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let status = res.status().as_u16();
            let body = res.text().await.unwrap_or_default();
            return Err(format!("Tavily API error ({status}): {body}"));
        }

        let tavily: TavilyResponse = res.json().await.map_err(|e| e.to_string())?;

        let mut output = String::new();
        for (i, result) in tavily.results.iter().enumerate() {
            output.push_str(&format!(
                "[{}] {}\n{}\n{}\n\n",
                i + 1,
                result.title,
                result.url,
                result.content
            ));
        }

        if output.is_empty() {
            output.push_str("No results found.");
        }

        Ok(output)
    }
}

impl Tool for SearchTool {
    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "Search the web for current information. Use when the user asks about \
         recent events, real-time data, or anything requiring up-to-date knowledge."
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query"
                }
            },
            "required": ["query"],
            "additionalProperties": false
        })
    }

    fn execute(
        &self,
        arguments: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = String> + Send + '_>> {
        let args = arguments.to_string();
        Box::pin(async move {
            let query = serde_json::from_str::<serde_json::Value>(&args)
                .ok()
                .and_then(|v| v["query"].as_str().map(|s| s.to_string()));

            match query {
                Some(q) => {
                    tracing::info!(query = %q, "Executing web search");
                    match self.search(&q).await {
                        Ok(results) => results,
                        Err(e) => format!("Search failed: {e}"),
                    }
                }
                None => "Invalid search arguments.".to_string(),
            }
        })
    }
}
