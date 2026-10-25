use serde::{Deserialize, Serialize};
use std::time::Instant;

pub const GEMINI_MODEL: &str = "gemini-2.5-flash";
pub const GEMINI_ENDPOINT: &str = "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundingSource {
    pub title: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiSearchResult {
    pub query: String,
    pub answer: String,
    pub sources: Vec<GroundingSource>,
    pub search_queries: Vec<String>,
    pub has_grounding: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiTestResult {
    pub success: bool,
    pub latency_ms: u64,
    pub message: String,
}

/// Build official request payload for Google Search grounding via Gemini API
pub fn build_gemini_search_payload(query: &str) -> serde_json::Value {
    serde_json::json!({
        "contents": [
            {
                "parts": [
                    { "text": query }
                ]
            }
        ],
        "tools": [
            {
                "google_search": {}
            }
        ]
    })
}

/// Parse Gemini response and extract text, search queries, and citation sources
pub fn parse_gemini_search_response(
    query: &str,
    response_json: &serde_json::Value,
) -> Result<GeminiSearchResult, String> {
    if let Some(err) = response_json.get("error") {
        let msg = err.get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown Gemini API error");
        return Err(format!("Gemini API Error: {msg}"));
    }

    let candidate = response_json.get("candidates")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .ok_or_else(|| "Gemini response contained no candidates".to_string())?;

    // Extract generated text
    let mut answer = String::new();
    if let Some(parts) = candidate.get("content").and_then(|c| c.get("parts")).and_then(|p| p.as_array()) {
        for part in parts {
            if let Some(t) = part.get("text").and_then(|v| v.as_str()) {
                answer.push_str(t);
            }
        }
    }

    if answer.is_empty() {
        answer = "No textual answer was returned for the query.".to_string();
    }

    // Extract Grounding Metadata
    let mut sources: Vec<GroundingSource> = Vec::new();
    let mut search_queries: Vec<String> = Vec::new();
    let mut has_grounding = false;

    if let Some(metadata) = candidate.get("groundingMetadata") {
        has_grounding = true;

        if let Some(queries) = metadata.get("webSearchQueries").and_then(|v| v.as_array()) {
            for q in queries {
                if let Some(qs) = q.as_str() {
                    search_queries.push(qs.to_string());
                }
            }
        }

        if let Some(chunks) = metadata.get("groundingChunks").and_then(|v| v.as_array()) {
            for chunk in chunks {
                if let Some(web) = chunk.get("web") {
                    let uri = web.get("uri").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let title = web.get("title").and_then(|v| v.as_str()).unwrap_or(&uri).to_string();
                    if !uri.is_empty() {
                        sources.push(GroundingSource { title, url: uri });
                    }
                }
            }
        }
    }

    Ok(GeminiSearchResult {
        query: query.to_string(),
        answer,
        sources,
        search_queries,
        has_grounding,
    })
}

/// Replace raw [cite: X, Y] placeholders with clean markdown links [Source Title](URL)
pub fn clean_citations(text: &str, sources: &[GroundingSource]) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;

    while let Some(start_idx) = rest.find("[cite:") {
        out.push_str(&rest[..start_idx]);
        let after_start = &rest[start_idx + 6..];
        if let Some(end_idx) = after_start.find(']') {
            let nums_str = after_start[..end_idx].trim();
            let mut links = Vec::new();
            for n_str in nums_str.split(',') {
                if let Ok(idx) = n_str.trim().parse::<usize>() {
                    if idx > 0 && idx <= sources.len() {
                        let src = &sources[idx - 1];
                        links.push(format!("[{}]({})", src.title, src.url));
                    }
                }
            }
            if !links.is_empty() {
                out.push_str(&format!(" ({})", links.join(", ")));
            }
            rest = &after_start[end_idx + 1..];
        } else {
            out.push_str("[cite:");
            rest = after_start;
        }
    }
    out.push_str(rest);
    out
}

/// Format the grounded answer with clean markdown source links and an appended Sources section
pub fn format_grounded_answer(result: &GeminiSearchResult) -> String {
    let mut cleaned = clean_citations(&result.answer, &result.sources);

    if !result.sources.is_empty() && !cleaned.contains("### Sources") && !cleaned.contains("### Authoritative Sources") {
        cleaned.push_str("\n\n### Sources\n");
        for (i, source) in result.sources.iter().enumerate() {
            cleaned.push_str(&format!("{}. [{}]({})\n", i + 1, source.title, source.url));
        }
    }

    cleaned
}

/// Execute a real Google Search grounding request to official Gemini API with multi-key failover
pub async fn execute_gemini_search(
    client: &reqwest::Client,
    api_key_str: &str,
    query: &str,
) -> Result<GeminiSearchResult, String> {
    let mut keys: Vec<&str> = Vec::new();
    for token in api_key_str.split([',', '\n', '\r']) {
        let trimmed = token.trim();
        if !trimmed.is_empty() && !keys.contains(&trimmed) {
            keys.push(trimmed);
        }
    }

    if keys.is_empty() {
        return Err("Gemini API key is required. Please configure your key in Plugins → Google Search.".into());
    }

    let trimmed_query = query.trim();
    if trimmed_query.is_empty() {
        return Err("Search query cannot be empty.".into());
    }

    let payload = build_gemini_search_payload(trimmed_query);
    let mut last_err = String::new();

    for (idx, key) in keys.iter().enumerate() {
        let masked = if key.len() > 8 {
            format!("{}...{}", &key[..4], &key[key.len() - 4..])
        } else {
            "****".to_string()
        };

        let response_res = client
            .post(GEMINI_ENDPOINT)
            .header("x-goog-api-key", *key)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await;

        let response = match response_res {
            Ok(r) => r,
            Err(e) => {
                last_err = format!("Key #{} ({masked}): Connection error: {e}", idx + 1);
                continue;
            }
        };

        let status = response.status();
        if status.is_success() {
            let json_val: serde_json::Value = response
                .json()
                .await
                .map_err(|e| format!("Failed to parse Gemini response JSON: {e}"))?;

            return parse_gemini_search_response(trimmed_query, &json_val);
        } else {
            let code = status.as_u16();
            let err_text = response.text().await.unwrap_or_default();
            last_err = match code {
                400 => format!("HTTP 400 Bad Request: Check query format. ({err_text})"),
                401 | 403 => format!("Key #{} ({masked}): HTTP 401/403 Unauthorized (Invalid API Key or missing permissions).", idx + 1),
                429 => format!("Key #{} ({masked}): HTTP 429 Rate limit or quota exhausted.", idx + 1),
                _ if code >= 500 => format!("Key #{} ({masked}): Google Gemini server error (HTTP {code}).", idx + 1),
                _ => format!("Key #{} ({masked}): HTTP {code} error: {err_text}", idx + 1),
            };

            // Permanent client error (bad request) should not rotate
            if code == 400 {
                return Err(last_err);
            }
            continue;
        }
    }

    Err(format!("All configured Gemini API keys failed. Last error: {last_err}"))
}

/// Test connection to Google Gemini API using provided API Key(s)
pub async fn test_gemini_connection(
    client: &reqwest::Client,
    api_key_str: &str,
) -> Result<ApiTestResult, String> {
    let mut keys: Vec<&str> = Vec::new();
    for token in api_key_str.split([',', '\n', '\r']) {
        let trimmed = token.trim();
        if !trimmed.is_empty() && !keys.contains(&trimmed) {
            keys.push(trimmed);
        }
    }

    if keys.is_empty() {
        return Ok(ApiTestResult {
            success: false,
            latency_ms: 0,
            message: "API Key is empty. Please enter your Gemini API Key.".into(),
        });
    }

    let ping_payload = serde_json::json!({
        "contents": [
            { "parts": [{ "text": "ping" }] }
        ]
    });

    let mut any_succeeded = false;
    let mut total_latency = 0u64;
    let mut messages = Vec::new();

    for (idx, key) in keys.iter().enumerate() {
        let start = Instant::now();
        let resp = client
            .post(GEMINI_ENDPOINT)
            .header("x-goog-api-key", *key)
            .header("Content-Type", "application/json")
            .json(&ping_payload)
            .send()
            .await;

        let latency = start.elapsed().as_millis() as u64;

        match resp {
            Ok(r) if r.status().is_success() => {
                any_succeeded = true;
                total_latency = latency;
                messages.push(format!("Key #{} connected ({latency}ms)", idx + 1));
            }
            Ok(r) => {
                let code = r.status().as_u16();
                messages.push(format!("Key #{} failed (HTTP {code})", idx + 1));
            }
            Err(e) => {
                messages.push(format!("Key #{} network error: {e}", idx + 1));
            }
        }
    }

    if any_succeeded {
        Ok(ApiTestResult {
            success: true,
            latency_ms: total_latency,
            message: format!(
                "Connected successfully to Gemini 2.5 Flash. Status: {}",
                messages.join(", ")
            ),
        })
    } else {
        Ok(ApiTestResult {
            success: false,
            latency_ms: 0,
            message: format!("All keys failed: {}", messages.join("; ")),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_gemini_search_payload() {
        let payload = build_gemini_search_payload("latest rust version");
        assert_eq!(payload["contents"][0]["parts"][0]["text"], "latest rust version");
        assert!(payload["tools"][0].get("google_search").is_some());
    }

    #[test]
    fn test_parse_gemini_search_response_with_grounding() {
        let sample = serde_json::json!({
            "candidates": [
                {
                    "content": {
                        "parts": [
                            { "text": "Rust 1.85 was released recently." }
                        ]
                    },
                    "groundingMetadata": {
                        "webSearchQueries": ["latest rust version release date"],
                        "groundingChunks": [
                            {
                                "web": {
                                    "uri": "https://blog.rust-lang.org",
                                    "title": "Rust Blog"
                                }
                            }
                        ]
                    }
                }
            ]
        });

        let result = parse_gemini_search_response("latest rust version", &sample).unwrap();
        assert_eq!(result.answer, "Rust 1.85 was released recently.");
        assert!(result.has_grounding);
        assert_eq!(result.sources.len(), 1);
        assert_eq!(result.sources[0].url, "https://blog.rust-lang.org");
        assert_eq!(result.sources[0].title, "Rust Blog");
        assert_eq!(result.search_queries, vec!["latest rust version release date"]);
    }

    #[test]
    fn test_parse_gemini_search_response_without_grounding() {
        let sample = serde_json::json!({
            "candidates": [
                {
                    "content": {
                        "parts": [
                            { "text": "Direct response without search." }
                        ]
                    }
                }
            ]
        });

        let result = parse_gemini_search_response("math question", &sample).unwrap();
        assert_eq!(result.answer, "Direct response without search.");
        assert!(!result.has_grounding);
        assert!(result.sources.is_empty());
    }

    #[test]
    fn test_parse_gemini_search_error() {
        let sample = serde_json::json!({
            "error": {
                "code": 403,
                "message": "API key not valid"
            }
        });

        let result = parse_gemini_search_response("query", &sample);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("API key not valid"));
    }

    #[test]
    fn test_clean_citations_replaces_placeholders() {
        let sources = vec![
            GroundingSource {
                title: "Rust Lang".to_string(),
                url: "https://www.rust-lang.org".to_string(),
            },
            GroundingSource {
                title: "Crates.io".to_string(),
                url: "https://crates.io".to_string(),
            },
        ];
        let raw = "Rust is fast [cite: 1]. Ecosystem is large [cite: 2, 99].";
        let cleaned = clean_citations(raw, &sources);
        assert!(cleaned.contains("([Rust Lang](https://www.rust-lang.org))"));
        assert!(cleaned.contains("([Crates.io](https://crates.io))"));
        assert!(!cleaned.contains("[cite:"));
    }

    #[test]
    fn test_format_grounded_answer_appends_sources() {
        let result = GeminiSearchResult {
            query: "rust version".to_string(),
            answer: "Latest is 1.85 [cite: 1].".to_string(),
            sources: vec![GroundingSource {
                title: "Release Notes".to_string(),
                url: "https://github.com/rust-lang/rust".to_string(),
            }],
            search_queries: vec!["rust version".to_string()],
            has_grounding: true,
        };
        let formatted = format_grounded_answer(&result);
        assert!(formatted.contains("### Sources"));
        assert!(formatted.contains("1. [Release Notes](https://github.com/rust-lang/rust)"));
        assert!(!formatted.contains("[cite: 1]"));
    }
}
