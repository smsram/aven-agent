use keyring::Entry;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;
use futures_util::StreamExt;

pub const SERVICE_NAME: &str = "com.aven.agent.cloudflare";
pub const CREDENTIALS_KEY: &str = "credentials_v1";
pub const MODEL_ID: &str = "@cf/qwen/qwen3.8-27b";

/// A structured, ordered pair of Cloudflare credentials
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CloudflareCredentialPair {
    pub account_id: String,
    pub api_key: String,
}

/// Safe metadata for a credential pair to expose to the frontend (never reveals raw API keys)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialPairMetadata {
    pub pair_index: usize,
    pub account_id_masked: String,
    pub is_valid: bool,
}

/// Overall credentials status safe for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialsStatus {
    pub configured_count: usize,
    pub pairs: Vec<CredentialPairMetadata>,
    pub preferred_index: usize,
    pub model_id: String,
}

/// Result of testing an individual credential pair
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairTestResult {
    pub pair_index: usize,
    pub account_id_masked: String,
    pub status: String, // "connected", "failed"
    pub error_message: Option<String>,
}

/// Result of testing all configured credential pairs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverallTestResult {
    pub all_tested: bool,
    pub any_connected: bool,
    pub results: Vec<PairTestResult>,
}

/// Chat message for Cloudflare Workers AI API payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String, // "system", "user", "assistant", "tool"
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tool_calls: Option<Vec<serde_json::Value>>,
}

/// Tauri event payload for incremental stream chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunkPayload {
    pub conversation_id: String,
    pub message_id: String,
    pub chunk: String,
}

/// Tauri event payload for real-time tool/agent activity status (e.g. searching, executing)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatActivityStatusPayload {
    pub conversation_id: String,
    pub message_id: String,
    pub status: String, // "searching", "synthesizing", "tool_running", "done", "idle"
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub integration_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tool_name: Option<String>,
}

/// Tauri event payload when stream finishes or encounters terminal state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamDonePayload {
    pub conversation_id: String,
    pub message_id: String,
    pub full_text: String,
    pub cancelled: bool,
    pub error: Option<String>,
    pub prompt_tokens: Option<u32>,
    pub completion_tokens: Option<u32>,
    pub total_tokens: Option<u32>,
}

/// Check if a user request asks for current, recent, today's, or time-sensitive information
pub fn is_time_sensitive_request(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    let keywords = [
        "latest",
        "recent",
        "today",
        "yesterday",
        "this week",
        "news",
        "current",
        "developments",
        "what happened",
        "release date",
        "releases",
        "search the web",
        "search web",
        "google search",
        "search for",
        "compare the latest",
        "who won",
        "stock price",
        "weather",
    ];

    keywords.iter().any(|&k| lower.contains(k))
}

/// Parse in-text tool call format like <tool_call>{"name": "...", "arguments": {...}}</tool_call>
pub fn parse_tool_call_from_text(text: &str) -> Option<(String, serde_json::Value)> {
    if let Some(start) = text.find("<tool_call>") {
        let after = &text[start + 11..];
        if let Some(end) = after.find("</tool_call>") {
            let json_str = after[..end].trim();
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(json_str) {
                let name = val.get("name").and_then(|v| v.as_str())?.to_string();
                let args = val.get("arguments").cloned().unwrap_or(serde_json::json!({}));
                return Some((name, args));
            }
        }
    }
    None
}

/// Extract structured tool call from parsed SSE JSON chunks (OpenAI or Workers AI format)
pub fn extract_tool_call_from_json(val: &serde_json::Value) -> Option<(String, serde_json::Value)> {
    if let Some(tool_calls) = val.get("tool_calls").and_then(|v| v.as_array()) {
        if let Some(first) = tool_calls.first() {
            let name = first.get("name").and_then(|v| v.as_str())?.to_string();
            let args = first.get("arguments").cloned().unwrap_or(serde_json::json!({}));
            return Some((name, args));
        }
    }
    if let Some(choices) = val.get("choices").and_then(|v| v.as_array()) {
        for choice in choices {
            if let Some(delta) = choice.get("delta") {
                if let Some(tc_arr) = delta.get("tool_calls").and_then(|v| v.as_array()) {
                    if let Some(first) = tc_arr.first() {
                        if let Some(func) = first.get("function") {
                            let name = func.get("name").and_then(|v| v.as_str())?.to_string();
                            let args = func.get("arguments")
                                .and_then(|v| {
                                    if let Some(s) = v.as_str() {
                                        serde_json::from_str(s).ok()
                                    } else {
                                        Some(v.clone())
                                    }
                                })
                                .unwrap_or(serde_json::json!({}));
                            return Some((name, args));
                        }
                    }
                }
            }
        }
    }
    None
}

/// Accumulator for streaming function tool calls across multiple SSE chunks
#[derive(Debug, Default, Clone)]
pub struct AccumulatedToolCall {
    pub id: String,
    pub name: String,
    pub arguments_buf: String,
}

impl AccumulatedToolCall {
    pub fn update_from_chunk(&mut self, val: &serde_json::Value) {
        if let Some(choices) = val.get("choices").and_then(|v| v.as_array()) {
            for choice in choices {
                if let Some(delta) = choice.get("delta") {
                    if let Some(tc_arr) = delta.get("tool_calls").and_then(|v| v.as_array()) {
                        for tc in tc_arr {
                            if let Some(id) = tc.get("id").and_then(|v| v.as_str()) {
                                if !id.is_empty() {
                                    self.id = id.to_string();
                                }
                            }
                            if let Some(func) = tc.get("function") {
                                if let Some(n) = func.get("name").and_then(|v| v.as_str()) {
                                    self.name.push_str(n);
                                }
                                if let Some(args) = func.get("arguments").and_then(|v| v.as_str()) {
                                    self.arguments_buf.push_str(args);
                                }
                            } else {
                                if let Some(n) = tc.get("name").and_then(|v| v.as_str()) {
                                    self.name.push_str(n);
                                }
                                if let Some(args) = tc.get("arguments") {
                                    if let Some(s) = args.as_str() {
                                        self.arguments_buf.push_str(s);
                                    } else {
                                        self.arguments_buf = args.to_string();
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Also check top-level tool_calls
        if let Some(tc_arr) = val.get("tool_calls").and_then(|v| v.as_array()) {
            for tc in tc_arr {
                if let Some(id) = tc.get("id").and_then(|v| v.as_str()) {
                    if !id.is_empty() {
                        self.id = id.to_string();
                    }
                }
                if let Some(func) = tc.get("function") {
                    if let Some(n) = func.get("name").and_then(|v| v.as_str()) {
                        self.name.push_str(n);
                    }
                    if let Some(args) = func.get("arguments").and_then(|v| v.as_str()) {
                        self.arguments_buf.push_str(args);
                    }
                } else {
                    if let Some(n) = tc.get("name").and_then(|v| v.as_str()) {
                        self.name.push_str(n);
                    }
                    if let Some(args) = tc.get("arguments") {
                        if let Some(s) = args.as_str() {
                            self.arguments_buf.push_str(s);
                        } else {
                            self.arguments_buf = args.to_string();
                        }
                    }
                }
            }
        }
    }

    pub fn is_valid(&self) -> bool {
        !self.name.trim().is_empty()
    }

    pub fn parse_arguments(&self) -> serde_json::Value {
        let trimmed = self.arguments_buf.trim();
        if trimmed.is_empty() {
            serde_json::json!({})
        } else {
            serde_json::from_str(trimmed).unwrap_or_else(|_| serde_json::json!({}))
        }
    }
}

/// Sanitize capability IDs into valid tool identifiers (letters, numbers, underscores only)
pub fn sanitize_tool_name(id: &str) -> String {
    let sanitized = id
        .replace("::", "_")
        .replace(':', "_")
        .replace('-', "_")
        .replace(' ', "_");
    let filtered: String = sanitized.chars().filter(|c| c.is_alphanumeric() || *c == '_').collect();
    if filtered.is_empty() {
        "tool".to_string()
    } else {
        filtered
    }
}

/// Map a model-requested tool name to a registered capability
pub fn resolve_capability<'a>(
    name: &str,
    capabilities: &'a [crate::plugins::registry::RegisteredCapability],
) -> Option<&'a crate::plugins::registry::RegisteredCapability> {
    let clean = sanitize_tool_name(name).to_lowercase();
    capabilities.iter().find(|c| {
        c.id.to_lowercase() == clean
            || sanitize_tool_name(&c.id).to_lowercase() == clean
            || sanitize_tool_name(&c.name).to_lowercase() == clean
            || (c.source_id == "api_gemini_search" && (clean == "google_search" || clean == "search"))
            || (c.source_id == "native_system" && (clean == "native_system_info" || clean == "system_info" || clean == "native_system"))
            || (c.source_id == "native_browser" && (clean == "native_open_url" || clean == "open_url" || clean == "native_browser"))
            || (c.source_id == "native_app_launcher" && (clean == "native_launch_app" || clean == "launch_app" || clean == "native_app_launcher"))
            || (c.source_id == "native_fs" && (clean == "native_file_access" || clean == "file_access" || clean == "native_fs"))
    })
}

/// Extract generated text tokens from diverse Cloudflare Workers AI streaming chunk formats:
/// Supports:
/// - Standard Cloudflare Workers AI OpenAI-compatible format: `choices[].delta.content` and `choices[].delta.reasoning_content`
/// - Legacy/direct Workers AI format: `response`
/// - Built-in thinking/reasoning format: `reasoning_content`, `thinking`, `thought`
/// - Envelope format: `result.response`
/// - Fallback formats: `content`, `text`
pub fn extract_token_from_json(val: &serde_json::Value) -> Option<String> {
    // 1. OpenAI-compatible SSE: choices[0].delta.content & choices[0].delta.reasoning_content
    if let Some(choices) = val.get("choices").and_then(|v| v.as_array()) {
        let mut token = String::new();
        for choice in choices {
            if let Some(delta) = choice.get("delta") {
                if let Some(c) = delta.get("content").and_then(|v| v.as_str()) {
                    token.push_str(c);
                }
                if let Some(rc) = delta.get("reasoning_content").and_then(|v| v.as_str()) {
                    token.push_str(rc);
                }
            }
            if let Some(t) = choice.get("text").and_then(|v| v.as_str()) {
                token.push_str(t);
            }
        }
        if !token.is_empty() {
            return Some(token);
        }
    }

    // 2. Direct "response" field (legacy Workers AI format)
    if let Some(r) = val.get("response").and_then(|v| v.as_str()) {
        if !r.is_empty() {
            return Some(r.to_string());
        }
    }

    // 3. Direct thinking / reasoning content
    if let Some(rc) = val.get("reasoning_content").and_then(|v| v.as_str()) {
        if !rc.is_empty() {
            return Some(rc.to_string());
        }
    }
    if let Some(t) = val.get("thinking").and_then(|v| v.as_str()) {
        if !t.is_empty() {
            return Some(t.to_string());
        }
    }
    if let Some(t) = val.get("thought").and_then(|v| v.as_str()) {
        if !t.is_empty() {
            return Some(t.to_string());
        }
    }

    // 4. Cloudflare result wrapper
    if let Some(res) = val.get("result") {
        if let Some(r) = res.get("response").and_then(|v| v.as_str()) {
            if !r.is_empty() {
                return Some(r.to_string());
            }
        }
        if let Some(r) = res.get("content").and_then(|v| v.as_str()) {
            if !r.is_empty() {
                return Some(r.to_string());
            }
        }
    }

    // 5. Fallback fields
    if let Some(c) = val.get("content").and_then(|v| v.as_str()) {
        if !c.is_empty() {
            return Some(c.to_string());
        }
    }
    if let Some(t) = val.get("text").and_then(|v| v.as_str()) {
        if !t.is_empty() {
            return Some(t.to_string());
        }
    }

    None
}

/// Failure classification for failover decision
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FailureCategory {
    AuthFailure,         // 401/403
    RateLimit,           // 429
    ServerOrNetwork,     // 5xx, timeout, connect err
    PermanentRequestErr, // 400, 404
}

/// Parse and validate comma-separated Account IDs and API Keys.
/// Rules:
/// - Trims whitespace around every entry.
/// - Ignores empty entries caused by leading, trailing, or repeated commas.
/// - Requires at least one valid credential pair.
/// - Validates that each Account ID has exactly one corresponding API Key.
/// - If counts differ, returns a clear validation error without silently pairing unrelated credentials.
pub fn parse_and_validate_pairs(
    account_ids_raw: &str,
    api_keys_raw: &str,
) -> Result<Vec<CloudflareCredentialPair>, String> {
    let accounts: Vec<String> = account_ids_raw
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let keys: Vec<String> = api_keys_raw
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if accounts.is_empty() && keys.is_empty() {
        return Err("No credentials provided. Please enter at least one Account ID and API Key.".into());
    }

    if accounts.len() != keys.len() {
        return Err(format!(
            "Mismatched count: {} Account ID(s) provided, but {} API Key(s) provided. Each Account ID must have a matching API Key.",
            accounts.len(),
            keys.len()
        ));
    }

    if accounts.is_empty() {
        return Err("At least one valid credential pair is required.".into());
    }

    let mut pairs = Vec::with_capacity(accounts.len());
    for (i, (acc, key)) in accounts.into_iter().zip(keys.into_iter()).enumerate() {
        // Basic sanity check on format
        if acc.contains(' ') || key.contains(' ') {
            return Err(format!(
                "Credential pair #{} contains invalid internal whitespace.",
                i + 1
            ));
        }
        pairs.push(CloudflareCredentialPair {
            account_id: acc,
            api_key: key,
        });
    }

    Ok(pairs)
}

/// Mask an Account ID for safe display in UI/logs (e.g. "a1b2...e5f6")
pub fn mask_account_id(id: &str) -> String {
    if id.len() <= 8 {
        if id.is_empty() {
            "****".into()
        } else {
            format!("{}****", &id[..1.min(id.len())])
        }
    } else {
        format!("{}...{}", &id[..4], &id[id.len() - 4..])
    }
}

/// Save pairs securely to OS Credential Manager
pub fn save_credentials_to_keyring(pairs: &[CloudflareCredentialPair]) -> Result<(), String> {
    let json = serde_json::to_string(pairs)
        .map_err(|e| format!("Failed to serialize credentials: {e}"))?;
    let entry = Entry::new(SERVICE_NAME, CREDENTIALS_KEY)
        .map_err(|e| format!("Failed to create keyring entry: {e}"))?;
    entry
        .set_password(&json)
        .map_err(|e| format!("Failed to write credentials to OS Credential Manager: {e}"))?;
    Ok(())
}

/// Load pairs securely from OS Credential Manager
pub fn load_credentials_from_keyring() -> Result<Vec<CloudflareCredentialPair>, String> {
    let entry = Entry::new(SERVICE_NAME, CREDENTIALS_KEY)
        .map_err(|e| format!("Failed to create keyring entry: {e}"))?;
    match entry.get_password() {
        Ok(json) => {
            let pairs: Vec<CloudflareCredentialPair> = serde_json::from_str(&json)
                .map_err(|e| format!("Failed to deserialize credentials: {e}"))?;
            Ok(pairs)
        }
        Err(keyring::Error::NoEntry) => Ok(Vec::new()),
        Err(e) => Err(format!("Keyring error: {e}")),
    }
}

/// Delete credentials from OS Credential Manager
pub fn clear_credentials_from_keyring() -> Result<(), String> {
    let entry = Entry::new(SERVICE_NAME, CREDENTIALS_KEY)
        .map_err(|e| format!("Failed to create keyring entry: {e}"))?;
    match entry.delete_credential() {
        Ok(_) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("Failed to delete credentials from OS Credential Manager: {e}")),
    }
}

/// In-memory manager for rotation, failover, and cooldowns
#[derive(Debug)]
pub struct CredentialManager {
    pub pairs: Vec<CloudflareCredentialPair>,
    pub preferred_index: usize,
    pub cooldowns: HashMap<usize, Instant>,
    pub failure_counts: HashMap<usize, usize>,
}

impl CredentialManager {
    pub fn new(pairs: Vec<CloudflareCredentialPair>) -> Self {
        Self {
            pairs,
            preferred_index: 0,
            cooldowns: HashMap::new(),
            failure_counts: HashMap::new(),
        }
    }

    pub fn set_pairs(&mut self, pairs: Vec<CloudflareCredentialPair>) {
        self.pairs = pairs;
        self.preferred_index = 0;
        self.cooldowns.clear();
        self.failure_counts.clear();
    }

    /// Return candidate pair indices in failover priority order.
    /// Always begins with preferred_index, then checks remaining pairs.
    /// Pairs in active cooldown are placed at the end unless all are in cooldown.
    pub fn get_candidate_indices(&self) -> Vec<usize> {
        let n = self.pairs.len();
        if n == 0 {
            return Vec::new();
        }

        let now = Instant::now();
        let mut available = Vec::new();
        let mut in_cooldown = Vec::new();

        for step in 0..n {
            let idx = (self.preferred_index + step) % n;
            if let Some(exp) = self.cooldowns.get(&idx) {
                if *exp > now {
                    in_cooldown.push((idx, *exp));
                    continue;
                }
            }
            available.push(idx);
        }

        // If all are in cooldown, include the ones that expire earliest
        if available.is_empty() {
            in_cooldown.sort_by_key(|&(_, exp)| exp);
            return in_cooldown.into_iter().map(|(idx, _)| idx).collect();
        }

        // Append cooldown pairs to the end as last-resort fallback
        in_cooldown.sort_by_key(|&(_, exp)| exp);
        for (idx, _) in in_cooldown {
            available.push(idx);
        }

        available
    }

    pub fn mark_success(&mut self, index: usize) {
        if index < self.pairs.len() {
            self.preferred_index = index;
            self.cooldowns.remove(&index);
            self.failure_counts.insert(index, 0);
        }
    }

    pub fn mark_failure(&mut self, index: usize, category: FailureCategory, retry_after_secs: Option<u64>) {
        let count = self.failure_counts.entry(index).or_insert(0);
        *count += 1;

        let duration = match category {
            FailureCategory::AuthFailure => Duration::from_secs(300), // 5 min cooldown for bad token
            FailureCategory::RateLimit => {
                Duration::from_secs(retry_after_secs.unwrap_or(30).clamp(5, 300))
            }
            FailureCategory::ServerOrNetwork => Duration::from_secs(10), // Short cooldown for transient
            FailureCategory::PermanentRequestErr => Duration::from_secs(0),
        };

        if duration > Duration::ZERO {
            self.cooldowns.insert(index, Instant::now() + duration);
        }
    }

    pub fn get_metadata(&self) -> Vec<CredentialPairMetadata> {
        self.pairs
            .iter()
            .enumerate()
            .map(|(i, pair)| CredentialPairMetadata {
                pair_index: i,
                account_id_masked: mask_account_id(&pair.account_id),
                is_valid: !pair.account_id.is_empty() && !pair.api_key.is_empty(),
            })
            .collect()
    }
}

/// Global shared state for Cloudflare service
pub struct CloudflareState {
    pub manager: Arc<Mutex<CredentialManager>>,
    pub http_client: reqwest::Client,
    pub active_cancellations: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
}

impl CloudflareState {
    pub fn new(initial_pairs: Vec<CloudflareCredentialPair>) -> Self {
        let http_client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(120))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            manager: Arc::new(Mutex::new(CredentialManager::new(initial_pairs))),
            http_client,
            active_cancellations: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

// -------------------------------------------------------------------------------------------------
// Cloudflare Workers AI Client Logic
// -------------------------------------------------------------------------------------------------

/// Test a single credential pair with a bounded non-streaming request
pub async fn test_single_pair(
    client: &reqwest::Client,
    pair: &CloudflareCredentialPair,
    pair_index: usize,
) -> PairTestResult {
    let masked = mask_account_id(&pair.account_id);
    let url = format!(
        "https://api.cloudflare.com/client/v4/accounts/{}/ai/run/{}",
        pair.account_id, MODEL_ID
    );

    let payload = serde_json::json!({
        "messages": [
            { "role": "user", "content": "ping" }
        ],
        "max_tokens": 1,
        "stream": false
    });

    let resp_res = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", pair.api_key))
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await;

    match resp_res {
        Ok(resp) => {
            let status = resp.status();
            if status.is_success() {
                PairTestResult {
                    pair_index,
                    account_id_masked: masked,
                    status: "connected".into(),
                    error_message: None,
                }
            } else {
                let status_code = status.as_u16();
                let sanitized_msg = match status_code {
                    401 => "Unauthorized (HTTP 401). Invalid API Key.".into(),
                    403 => "Forbidden (HTTP 403). Ensure API Token has 'Workers AI: Read/Write' permissions for this Account ID.".into(),
                    404 => format!("Not Found (HTTP 404). Verify Account ID '{}' and model access for '{}'.", masked, MODEL_ID),
                    429 => "Rate limited (HTTP 429). Account currently at capacity.".into(),
                    code if code >= 500 => format!("Cloudflare service error (HTTP {code})."),
                    code => format!("HTTP {code} response from Cloudflare AI gateway."),
                };
                PairTestResult {
                    pair_index,
                    account_id_masked: masked,
                    status: "failed".into(),
                    error_message: Some(sanitized_msg),
                }
            }
        }
        Err(e) => {
            let msg = if e.is_timeout() {
                "Connection timed out (15s limit reached).".into()
            } else if e.is_connect() {
                "Failed to establish network connection to api.cloudflare.com.".into()
            } else {
                "Network request failed.".into()
            };
            PairTestResult {
                pair_index,
                account_id_masked: masked,
                status: "failed".into(),
                error_message: Some(msg),
            }
        }
    }
}

/// Execute real incremental chat streaming with automatic credential failover and runtime tool execution
pub async fn execute_streaming_chat(
    app: AppHandle,
    state: &CloudflareState,
    plugins: &Arc<crate::plugins::PluginsManager>,
    conversation_id: String,
    message_id: String,
    messages: Vec<ChatMessage>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
) {
    let cancel_token = Arc::new(AtomicBool::new(false));
    {
        let mut map = state.active_cancellations.lock().await;
        map.insert(conversation_id.clone(), cancel_token.clone());
    }

    let candidates = {
        let mgr = state.manager.lock().await;
        mgr.get_candidate_indices()
    };

    if candidates.is_empty() {
        let _ = app.emit(
            "chat-stream-done",
            StreamDonePayload {
                conversation_id: conversation_id.clone(),
                message_id: message_id.clone(),
                full_text: String::new(),
                cancelled: false,
                error: Some(
                    "No Cloudflare credentials configured. Open Settings → API to configure Account ID and API Key."
                        .into(),
                ),
                prompt_tokens: None,
                completion_tokens: None,
                total_tokens: None,
            },
        );
        let mut map = state.active_cancellations.lock().await;
        map.remove(&conversation_id);
        return;
    }

    // 1. Inspect capability availability and user intent
    let plugins_state = plugins.get_state().await;
    let google_search_available = plugins_state
        .capabilities
        .iter()
        .any(|c| (c.id == "google_search" || c.id == "api::gemini_search::search") && c.is_available);

    let latest_user_prompt = messages
        .iter()
        .rev()
        .find(|m| m.role == "user")
        .map(|m| m.content.clone())
        .unwrap_or_default();

    let is_time_sensitive = is_time_sensitive_request(&latest_user_prompt);

    // Normalize messages: Cloudflare requires role: "system" strictly at index 0.
    // If any subsequent messages have role: "system", consolidate them into the initial system prompt.
    let mut effective_messages = Vec::new();
    let mut system_prompt_content = String::new();

    for m in messages {
        if m.role == "system" {
            if system_prompt_content.is_empty() {
                system_prompt_content = m.content;
            } else {
                system_prompt_content.push_str("\n\n");
                system_prompt_content.push_str(&m.content);
            }
        } else {
            effective_messages.push(m);
        }
    }

    if system_prompt_content.is_empty() {
        system_prompt_content = "You are Aven, a sophisticated engineering AI assistant.".to_string();
    }

    system_prompt_content.push_str(
        "\n\nYou have access to local tools and capabilities. Always call the corresponding tool when relevant:\n- Call `native_system_info` to get the user's computer/device name, username, OS, and system architecture.\n- Call `google_search` to find current news, recent events, or live web info.\n- Call `native_open_url` to open links in the default web browser.\n- Call `native_launch_app` to launch approved Windows applications.\n- Call `native_file_access` to safely read or list files in the approved project workspace.\nNever invent device names, fake citations, or pretend you cannot check system details when `native_system_info` is available."
    );

    // 2. If time-sensitive and Google Search via Gemini API is available:
    if is_time_sensitive && google_search_available {
        let _ = app.emit(
            "chat-activity-status",
            ChatActivityStatusPayload {
                conversation_id: conversation_id.clone(),
                message_id: message_id.clone(),
                status: "searching".to_string(),
                description: format!("Searching the web for: \"{}\"...", latest_user_prompt),
                integration_id: Some("api_gemini_search".to_string()),
                tool_name: Some("google_search".to_string()),
            },
        );

        let search_req = crate::plugins::registry::CapabilityExecutionRequest {
            capability_id: "google_search".to_string(),
            arguments: serde_json::json!({ "query": latest_user_prompt }),
            user_confirmed: true,
        };

        let search_res = plugins.execute_capability_internal(search_req).await;

        match search_res {
            Ok(res) if res.success => {
                let formatted = res
                    .result
                    .as_ref()
                    .and_then(|r| r.get("formatted_answer"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                let _ = app.emit(
                    "chat-activity-status",
                    ChatActivityStatusPayload {
                        conversation_id: conversation_id.clone(),
                        message_id: message_id.clone(),
                        status: "synthesizing".to_string(),
                        description: "Synthesizing answer with verified citations...".to_string(),
                        integration_id: Some("api_gemini_search".to_string()),
                        tool_name: Some("google_search".to_string()),
                    },
                );

                system_prompt_content.push_str(&format!(
                    "\n\nAuthoritative live Google Search Grounding results for \"{}\":\n\n{}\n\nProvide a comprehensive, well-structured final answer incorporating these verified findings. Cite each source with clickable markdown links [Source Title](URL). Do not output raw [cite: ...] placeholders or raw JSON.",
                    latest_user_prompt, formatted
                ));
            }
            Ok(res) => {
                let err_desc = res
                    .error
                    .unwrap_or_else(|| "Search request failed".to_string());
                system_prompt_content.push_str(&format!(
                    "\n\nGoogle Search notice: Live web search was attempted for '{}' but returned: {}. Note this limitation honestly without fabricating fake results.",
                    latest_user_prompt, err_desc
                ));
            }
            Err(e) => {
                system_prompt_content.push_str(&format!(
                    "\n\nGoogle Search notice: Live web search was attempted for '{}' but encountered an error: {}. Note this limitation honestly without fabricating fake results.",
                    latest_user_prompt, e
                ));
            }
        }
    }

    // Insert the consolidated system message strictly at index 0
    effective_messages.insert(
        0,
        ChatMessage {
            role: "system".to_string(),
            content: system_prompt_content,
            name: None,
            tool_call_id: None,
            tool_calls: None,
        },
    );

    // Build tools array for model payload with sanitized identifiers
    let mut tools_list: Vec<serde_json::Value> = Vec::new();
    for cap in &plugins_state.capabilities {
        if cap.is_available {
            let tool_name = sanitize_tool_name(&cap.id);
            let mut schema = cap.input_schema.clone();
            if !schema.is_object() || schema.get("type").is_none() {
                schema = serde_json::json!({
                    "type": "object",
                    "properties": {}
                });
            }
            tools_list.push(serde_json::json!({
                "name": tool_name,
                "description": cap.description,
                "parameters": schema,
            }));
        }
    }

    let mut all_errors: Vec<String> = Vec::new();
    let mut full_text = String::new();
    let mut prompt_tokens: Option<u32> = None;
    let mut completion_tokens: Option<u32> = None;
    let mut total_tokens: Option<u32> = None;
    let mut stream_interrupted = false;

    let mut current_step: usize = 0;
    const MAX_TOOL_STEPS: usize = 3;

    while current_step < MAX_TOOL_STEPS {
        current_step += 1;
        let mut turn_succeeded = false;
        let mut turn_accumulated_tool = AccumulatedToolCall::default();
        let mut turn_text = String::new();
        let mut fallback_prompt_converted = false;

        for &idx in &candidates {
            if cancel_token.load(Ordering::SeqCst) {
                let _ = app.emit(
                    "chat-activity-status",
                    ChatActivityStatusPayload {
                        conversation_id: conversation_id.clone(),
                        message_id: message_id.clone(),
                        status: "done".to_string(),
                        description: String::new(),
                        integration_id: None,
                        tool_name: None,
                    },
                );
                let _ = app.emit(
                    "chat-stream-done",
                    StreamDonePayload {
                        conversation_id: conversation_id.clone(),
                        message_id: message_id.clone(),
                        full_text: full_text.clone(),
                        cancelled: true,
                        error: None,
                        prompt_tokens,
                        completion_tokens,
                        total_tokens,
                    },
                );
                let mut map = state.active_cancellations.lock().await;
                map.remove(&conversation_id);
                return;
            }

            let pair = {
                let mgr = state.manager.lock().await;
                mgr.pairs.get(idx).cloned()
            };

            let pair = match pair {
                Some(p) => p,
                None => continue,
            };

            let masked = mask_account_id(&pair.account_id);
            let url = format!(
                "https://api.cloudflare.com/client/v4/accounts/{}/ai/run/{}",
                pair.account_id, MODEL_ID
            );

            let mut req_body_map = serde_json::Map::new();
            req_body_map.insert(
                "messages".to_string(),
                serde_json::to_value(&effective_messages).unwrap_or_default(),
            );
            req_body_map.insert("stream".to_string(), serde_json::Value::Bool(true));
            req_body_map.insert(
                "max_tokens".to_string(),
                serde_json::json!(max_tokens.unwrap_or(4096)),
            );
            req_body_map.insert(
                "temperature".to_string(),
                serde_json::json!(temperature.unwrap_or(0.2)),
            );

            // Supply tool definitions only if within tool invocation allowance
            if !tools_list.is_empty() && current_step < MAX_TOOL_STEPS {
                req_body_map.insert(
                    "tools".to_string(),
                    serde_json::Value::Array(tools_list.clone()),
                );
            }

            let req_body = serde_json::Value::Object(req_body_map);

            let send_res = state
                .http_client
                .post(&url)
                .header("Authorization", format!("Bearer {}", pair.api_key))
                .header("Content-Type", "application/json")
                .json(&req_body)
                .send()
                .await;

            let response = match send_res {
                Ok(r) => r,
                Err(_e) => {
                    let err_desc = format!("Pair #{} ({}): network error", idx + 1, masked);
                    all_errors.push(err_desc);
                    let mut mgr = state.manager.lock().await;
                    mgr.mark_failure(idx, FailureCategory::ServerOrNetwork, None);
                    continue;
                }
            };

            let status = response.status();
            if !status.is_success() {
                let code = status.as_u16();
                let retry_after = response
                    .headers()
                    .get("Retry-After")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok());

                let err_body = response.text().await.unwrap_or_default();
                let detail = serde_json::from_str::<serde_json::Value>(&err_body)
                    .ok()
                    .and_then(|val| {
                        val.get("errors")
                            .and_then(|e| e.as_array())
                            .and_then(|arr| arr.first())
                            .and_then(|f| f.get("message"))
                            .and_then(|m| m.as_str())
                            .map(|s| s.to_string())
                            .or_else(|| {
                                val.get("messages")
                                    .and_then(|m| m.as_array())
                                    .and_then(|arr| arr.first())
                                    .and_then(|f| f.as_str())
                                    .map(|s| s.to_string())
                            })
                    })
                    .unwrap_or_else(|| {
                        if err_body.trim().is_empty() {
                            format!("HTTP {code}")
                        } else {
                            err_body.chars().take(200).collect()
                        }
                    });

                // Resilience fallback: If Cloudflare rejects role: "tool" with HTTP 400 on synthesis,
                // replace the assistant tool call and tool message with a user prompt and retry.
                if current_step > 1 && code == 400 && !fallback_prompt_converted && effective_messages.len() >= 2 {
                    let last_tool = effective_messages.pop();
                    let _last_asst = effective_messages.pop();
                    let tool_name_info = last_tool.as_ref().and_then(|m| m.name.clone()).unwrap_or_else(|| "tool".to_string());
                    let tool_result_info = last_tool.map(|m| m.content).unwrap_or_default();

                    effective_messages.push(ChatMessage {
                        role: "user".to_string(),
                        content: format!(
                            "[Verified Tool Result for {}]:\n{}\n\nBased on these verified results, please provide a comprehensive and direct answer to the user.",
                            tool_name_info, tool_result_info
                        ),
                        name: None,
                        tool_call_id: None,
                        tool_calls: None,
                    });
                    fallback_prompt_converted = true;

                    let retry_body = serde_json::json!({
                        "messages": effective_messages,
                        "stream": true,
                        "max_tokens": max_tokens.unwrap_or(4096),
                        "temperature": temperature.unwrap_or(0.2)
                    });

                    let retry_res = state
                        .http_client
                        .post(&url)
                        .header("Authorization", format!("Bearer {}", pair.api_key))
                        .header("Content-Type", "application/json")
                        .json(&retry_body)
                        .send()
                        .await;

                    if let Ok(retry_resp) = retry_res {
                        if retry_resp.status().is_success() {
                            // Retry succeeded with fallback formatting!
                            let mut byte_stream = retry_resp.bytes_stream();
                            let mut buffer = String::new();
                            turn_accumulated_tool = AccumulatedToolCall::default();
                            turn_text.clear();

                            while let Some(chunk_res) = byte_stream.next().await {
                                if cancel_token.load(Ordering::SeqCst) {
                                    let mut map = state.active_cancellations.lock().await;
                                    map.remove(&conversation_id);
                                    return;
                                }

                                let bytes = match chunk_res {
                                    Ok(b) => b,
                                    Err(_) => {
                                        stream_interrupted = true;
                                        break;
                                    }
                                };

                                let text_chunk = String::from_utf8_lossy(&bytes);
                                buffer.push_str(&text_chunk);

                                while let Some(pos) = buffer.find('\n') {
                                    let line = buffer[..pos].trim().to_string();
                                    buffer = buffer[pos + 1..].to_string();

                                    if line.is_empty() {
                                        continue;
                                    }

                                    if let Some(data_str) = line.strip_prefix("data:") {
                                        let data_trimmed = data_str.trim();
                                        if data_trimmed == "[DONE]" {
                                            buffer.clear();
                                            break;
                                        }

                                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data_trimmed) {
                                            if let Some(token) = extract_token_from_json(&parsed) {
                                                if !token.is_empty() {
                                                    turn_text.push_str(&token);
                                                    full_text.push_str(&token);
                                                    let _ = app.emit(
                                                        "chat-stream-chunk",
                                                        StreamChunkPayload {
                                                            conversation_id: conversation_id.clone(),
                                                            message_id: message_id.clone(),
                                                            chunk: token,
                                                        },
                                                    );
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            turn_succeeded = true;
                            break;
                        }
                    }
                }

                let (cat, err_msg) = match code {
                    401 => (
                        FailureCategory::AuthFailure,
                        format!("Pair #{} ({}): HTTP 401 Unauthorized (Invalid API Key)", idx + 1, masked),
                    ),
                    403 => (
                        FailureCategory::AuthFailure,
                        format!("Pair #{} ({}): HTTP 403 Forbidden (Check Workers AI permissions)", idx + 1, masked),
                    ),
                    404 => (
                        FailureCategory::PermanentRequestErr,
                        format!("Model '{}' not found or invalid account (HTTP 404).", MODEL_ID),
                    ),
                    429 => (
                        FailureCategory::RateLimit,
                        format!("Pair #{} ({}): HTTP 429 Rate limited", idx + 1, masked),
                    ),
                    _ if code >= 500 => (
                        FailureCategory::ServerOrNetwork,
                        format!("Pair #{} ({}): Cloudflare server error (HTTP {}): {}", idx + 1, masked, code, detail),
                    ),
                    _ => (
                        FailureCategory::PermanentRequestErr,
                        format!("Pair #{} ({}): HTTP {} error from Cloudflare: {}", idx + 1, masked, code, detail),
                    ),
                };

                let mut mgr = state.manager.lock().await;
                mgr.mark_failure(idx, cat, retry_after);

                if cat == FailureCategory::PermanentRequestErr {
                    let _ = app.emit(
                        "chat-activity-status",
                        ChatActivityStatusPayload {
                            conversation_id: conversation_id.clone(),
                            message_id: message_id.clone(),
                            status: "done".to_string(),
                            description: String::new(),
                            integration_id: None,
                            tool_name: None,
                        },
                    );
                    let _ = app.emit(
                        "chat-stream-done",
                        StreamDonePayload {
                            conversation_id: conversation_id.clone(),
                            message_id: message_id.clone(),
                            full_text: String::new(),
                            cancelled: false,
                            error: Some(err_msg),
                            prompt_tokens: None,
                            completion_tokens: None,
                            total_tokens: None,
                        },
                    );
                    let mut map = state.active_cancellations.lock().await;
                    map.remove(&conversation_id);
                    return;
                }

                all_errors.push(err_msg);
                continue;
            }

            // Connection successful! Mark this pair as preferred
            {
                let mut mgr = state.manager.lock().await;
                mgr.mark_success(idx);
            }

            // Process SSE stream
            let mut byte_stream = response.bytes_stream();
            let mut buffer = String::new();
            turn_accumulated_tool = AccumulatedToolCall::default();
            turn_text.clear();

            while let Some(chunk_res) = byte_stream.next().await {
                if cancel_token.load(Ordering::SeqCst) {
                    let _ = app.emit(
                        "chat-activity-status",
                        ChatActivityStatusPayload {
                            conversation_id: conversation_id.clone(),
                            message_id: message_id.clone(),
                            status: "done".to_string(),
                            description: String::new(),
                            integration_id: None,
                            tool_name: None,
                        },
                    );
                    let _ = app.emit(
                        "chat-stream-done",
                        StreamDonePayload {
                            conversation_id: conversation_id.clone(),
                            message_id: message_id.clone(),
                            full_text: full_text.clone(),
                            cancelled: true,
                            error: None,
                            prompt_tokens,
                            completion_tokens,
                            total_tokens,
                        },
                    );
                    let mut map = state.active_cancellations.lock().await;
                    map.remove(&conversation_id);
                    return;
                }

                let bytes = match chunk_res {
                    Ok(b) => b,
                    Err(_) => {
                        stream_interrupted = true;
                        break;
                    }
                };

                let text_chunk = String::from_utf8_lossy(&bytes);
                buffer.push_str(&text_chunk);

                while let Some(pos) = buffer.find('\n') {
                    let line = buffer[..pos].trim().to_string();
                    buffer = buffer[pos + 1..].to_string();

                    if line.is_empty() {
                        continue;
                    }

                    if let Some(data_str) = line.strip_prefix("data:") {
                        let data_trimmed = data_str.trim();
                        if data_trimmed == "[DONE]" {
                            buffer.clear();
                            break;
                        }

                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data_trimmed) {
                            turn_accumulated_tool.update_from_chunk(&parsed);

                            if let Some(token) = extract_token_from_json(&parsed) {
                                if !token.is_empty() {
                                    turn_text.push_str(&token);
                                    // If not currently accumulating a tool call, stream directly to user
                                    if !turn_accumulated_tool.is_valid() {
                                        full_text.push_str(&token);
                                        let _ = app.emit(
                                            "chat-stream-chunk",
                                            StreamChunkPayload {
                                                conversation_id: conversation_id.clone(),
                                                message_id: message_id.clone(),
                                                chunk: token,
                                            },
                                        );
                                    }
                                }
                            }

                            if let Some(usage) = parsed.get("usage") {
                                if let Some(pt) = usage.get("prompt_tokens").and_then(|v| v.as_u64()) {
                                    prompt_tokens = Some(pt as u32);
                                }
                                if let Some(ct) = usage.get("completion_tokens").and_then(|v| v.as_u64()) {
                                    completion_tokens = Some(ct as u32);
                                }
                                if let Some(tt) = usage.get("total_tokens").and_then(|v| v.as_u64()) {
                                    total_tokens = Some(tt as u32);
                                }
                            }
                        }
                    }
                }
            }

            // Handle trailing buffer
            if !buffer.trim().is_empty() {
                let line = buffer.trim();
                if let Some(data_str) = line.strip_prefix("data:") {
                    let data_trimmed = data_str.trim();
                    if data_trimmed != "[DONE]" {
                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data_trimmed) {
                            turn_accumulated_tool.update_from_chunk(&parsed);
                            if let Some(token) = extract_token_from_json(&parsed) {
                                if !token.is_empty() {
                                    turn_text.push_str(&token);
                                    if !turn_accumulated_tool.is_valid() {
                                        full_text.push_str(&token);
                                        let _ = app.emit(
                                            "chat-stream-chunk",
                                            StreamChunkPayload {
                                                conversation_id: conversation_id.clone(),
                                                message_id: message_id.clone(),
                                                chunk: token,
                                            },
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
                buffer.clear();
            }

            turn_succeeded = true;
            break;
        }

        if !turn_succeeded {
            break;
        }

        // Check if a tool call was detected (either via native delta.tool_calls or in-text tag)
        let in_text_tool = if !turn_accumulated_tool.is_valid() {
            parse_tool_call_from_text(&turn_text)
        } else {
            None
        };

        if turn_accumulated_tool.is_valid() || in_text_tool.is_some() {
            let (tool_name, tool_args, tool_call_id) = if turn_accumulated_tool.is_valid() {
                let name = turn_accumulated_tool.name.clone();
                let args = turn_accumulated_tool.parse_arguments();
                let id = if turn_accumulated_tool.id.is_empty() {
                    format!("call_{}", &message_id[..std::cmp::min(8, message_id.len())])
                } else {
                    turn_accumulated_tool.id.clone()
                };
                (name, args, id)
            } else {
                let (name, args) = in_text_tool.unwrap();
                let id = format!("call_{}", &message_id[..std::cmp::min(8, message_id.len())]);
                (name, args, id)
            };

            let resolved = resolve_capability(&tool_name, &plugins_state.capabilities);
            let display_name = resolved.map(|c| c.name.as_str()).unwrap_or(&tool_name);

            let _ = app.emit(
                "chat-activity-status",
                ChatActivityStatusPayload {
                    conversation_id: conversation_id.clone(),
                    message_id: message_id.clone(),
                    status: "tool_running".to_string(),
                    description: format!("Executing tool: {}...", display_name),
                    integration_id: resolved.map(|c| c.source_id.clone()),
                    tool_name: Some(tool_name.clone()),
                },
            );

            let exec_res = if let Some(cap) = resolved {
                let req = crate::plugins::registry::CapabilityExecutionRequest {
                    capability_id: cap.id.clone(),
                    arguments: tool_args.clone(),
                    user_confirmed: true,
                };
                plugins.execute_capability_internal(req).await
            } else {
                Err(format!("Capability '{}' not found in registry", tool_name))
            };

            let tool_content_str = match &exec_res {
                Ok(r) if r.success => {
                    serde_json::to_string(&r.result.clone().unwrap_or(serde_json::json!({ "status": "ok" }))).unwrap_or_default()
                }
                Ok(r) => {
                    serde_json::json!({ "error": r.error.clone().unwrap_or_else(|| "Execution failed".to_string()) }).to_string()
                }
                Err(e) => {
                    serde_json::json!({ "error": e }).to_string()
                }
            };

            // Append assistant tool call and tool response to conversation history
            effective_messages.push(ChatMessage {
                role: "assistant".to_string(),
                content: String::new(),
                name: None,
                tool_call_id: None,
                tool_calls: Some(vec![serde_json::json!({
                    "id": tool_call_id,
                    "type": "function",
                    "name": tool_name,
                    "function": {
                        "name": tool_name,
                        "arguments": serde_json::to_string(&tool_args).unwrap_or_default(),
                    }
                })]),
            });

            effective_messages.push(ChatMessage {
                role: "tool".to_string(),
                content: tool_content_str.clone(),
                name: Some(tool_name.clone()),
                tool_call_id: Some(tool_call_id),
                tool_calls: None,
            });

            let _ = app.emit(
                "chat-activity-status",
                ChatActivityStatusPayload {
                    conversation_id: conversation_id.clone(),
                    message_id: message_id.clone(),
                    status: "synthesizing".to_string(),
                    description: "Synthesizing answer with verified findings...".to_string(),
                    integration_id: resolved.map(|c| c.source_id.clone()),
                    tool_name: Some(tool_name.clone()),
                },
            );

            // Continue loop to synthesize final user-facing response with model
            continue;
        }

        // If this was a synthesis turn after tool execution, but the model returned no text:
        if current_step > 1 && full_text.trim().is_empty() {
            // Provide authoritative synthesized fallback from the tool result
            if let Some(tool_msg) = effective_messages.iter().rev().find(|m| m.role == "tool") {
                if let Ok(parsed_res) = serde_json::from_str::<serde_json::Value>(&tool_msg.content) {
                    if let Some(dev_name) = parsed_res.get("device_name").and_then(|v| v.as_str()) {
                        let os = parsed_res.get("os").and_then(|v| v.as_str()).unwrap_or("Windows");
                        let user = parsed_res.get("user_name").and_then(|v| v.as_str()).unwrap_or("");
                        let fallback_answer = if !user.is_empty() {
                            format!("Your device name is **{}** (logged in as `{}` on {}).", dev_name, user, os)
                        } else {
                            format!("Your device name is **{}** (running {}).", dev_name, os)
                        };
                        full_text = fallback_answer.clone();
                        let _ = app.emit(
                            "chat-stream-chunk",
                            StreamChunkPayload {
                                conversation_id: conversation_id.clone(),
                                message_id: message_id.clone(),
                                chunk: fallback_answer,
                            },
                        );
                    }
                }
            }
        }

        // Final response completed
        break;
    }

    let _ = app.emit(
        "chat-activity-status",
        ChatActivityStatusPayload {
            conversation_id: conversation_id.clone(),
            message_id: message_id.clone(),
            status: "done".to_string(),
            description: String::new(),
            integration_id: None,
            tool_name: None,
        },
    );

    if stream_interrupted {
        let _ = app.emit(
            "chat-stream-done",
            StreamDonePayload {
                conversation_id: conversation_id.clone(),
                message_id: message_id.clone(),
                full_text: full_text.clone(),
                cancelled: false,
                error: Some(
                    "Stream interrupted due to network disconnect. You can retry the request."
                        .into(),
                ),
                prompt_tokens,
                completion_tokens,
                total_tokens,
            },
        );
    } else if full_text.is_empty() && !all_errors.is_empty() {
        let combined_err = format!(
            "All configured credential pairs failed:\n• {}",
            all_errors.join("\n• ")
        );
        let _ = app.emit(
            "chat-stream-done",
            StreamDonePayload {
                conversation_id: conversation_id.clone(),
                message_id: message_id.clone(),
                full_text: String::new(),
                cancelled: false,
                error: Some(combined_err),
                prompt_tokens,
                completion_tokens,
                total_tokens,
            },
        );
    } else {
        let _ = app.emit(
            "chat-stream-done",
            StreamDonePayload {
                conversation_id: conversation_id.clone(),
                message_id: message_id.clone(),
                full_text,
                cancelled: false,
                error: None,
                prompt_tokens,
                completion_tokens,
                total_tokens,
            },
        );
    }

    let mut map = state.active_cancellations.lock().await;
    map.remove(&conversation_id);
}

// -------------------------------------------------------------------------------------------------
// Tauri Commands
// -------------------------------------------------------------------------------------------------

#[tauri::command]
pub async fn get_cloudflare_status(
    state: tauri::State<'_, Arc<CloudflareState>>,
) -> Result<CredentialsStatus, String> {
    let mgr = state.manager.lock().await;
    Ok(CredentialsStatus {
        configured_count: mgr.pairs.len(),
        pairs: mgr.get_metadata(),
        preferred_index: mgr.preferred_index,
        model_id: MODEL_ID.to_string(),
    })
}

#[tauri::command]
pub async fn save_cloudflare_credentials(
    state: tauri::State<'_, Arc<CloudflareState>>,
    account_ids: String,
    api_keys: String,
) -> Result<CredentialsStatus, String> {
    let pairs = parse_and_validate_pairs(&account_ids, &api_keys)?;
    save_credentials_to_keyring(&pairs)?;
    {
        let mut mgr = state.manager.lock().await;
        mgr.set_pairs(pairs);
    }
    let mgr = state.manager.lock().await;
    Ok(CredentialsStatus {
        configured_count: mgr.pairs.len(),
        pairs: mgr.get_metadata(),
        preferred_index: mgr.preferred_index,
        model_id: MODEL_ID.to_string(),
    })
}

#[tauri::command]
pub async fn test_cloudflare_credentials(
    state: tauri::State<'_, Arc<CloudflareState>>,
) -> Result<OverallTestResult, String> {
    let (pairs, client) = {
        let mgr = state.manager.lock().await;
        (mgr.pairs.clone(), state.http_client.clone())
    };

    if pairs.is_empty() {
        return Ok(OverallTestResult {
            all_tested: true,
            any_connected: false,
            results: Vec::new(),
        });
    }

    let mut results = Vec::new();
    let mut any_connected = false;

    for (i, pair) in pairs.iter().enumerate() {
        let res = test_single_pair(&client, pair, i).await;
        if res.status == "connected" {
            any_connected = true;
        }
        results.push(res);
    }

    Ok(OverallTestResult {
        all_tested: true,
        any_connected,
        results,
    })
}

#[tauri::command]
pub async fn clear_cloudflare_credentials(
    state: tauri::State<'_, Arc<CloudflareState>>,
) -> Result<(), String> {
    clear_credentials_from_keyring()?;
    {
        let mut mgr = state.manager.lock().await;
        mgr.set_pairs(Vec::new());
    }
    Ok(())
}

#[tauri::command]
pub async fn start_chat_stream(
    app: AppHandle,
    state: tauri::State<'_, Arc<CloudflareState>>,
    plugins: tauri::State<'_, Arc<crate::plugins::PluginsManager>>,
    conversation_id: String,
    message_id: String,
    messages: Vec<ChatMessage>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
) -> Result<(), String> {
    let state_arc = state.inner().clone();
    let plugins_arc = plugins.inner().clone();
    tokio::spawn(async move {
        execute_streaming_chat(
            app,
            &state_arc,
            &plugins_arc,
            conversation_id,
            message_id,
            messages,
            max_tokens,
            temperature,
        )
        .await;
    });
    Ok(())
}

#[tauri::command]
pub async fn stop_chat_stream(
    state: tauri::State<'_, Arc<CloudflareState>>,
    conversation_id: String,
) -> Result<(), String> {
    let map = state.active_cancellations.lock().await;
    if let Some(cancel_token) = map.get(&conversation_id) {
        cancel_token.store(true, Ordering::SeqCst);
    }
    Ok(())
}

// -------------------------------------------------------------------------------------------------
// Unit Tests for Credential Parsing and Failover Logic
// -------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_single_pair() {
        let res = parse_and_validate_pairs("acc_1", "key_1").unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].account_id, "acc_1");
        assert_eq!(res[0].api_key, "key_1");
    }

    #[test]
    fn test_parse_valid_multiple_pairs_with_spaces_and_commas() {
        let accs = " acc_1 ,  acc_2, acc_3 ,, ";
        let keys = " key_1, key_2 ,  key_3, ";
        let res = parse_and_validate_pairs(accs, keys).unwrap();
        assert_eq!(res.len(), 3);
        assert_eq!(res[0].account_id, "acc_1");
        assert_eq!(res[0].api_key, "key_1");
        assert_eq!(res[1].account_id, "acc_2");
        assert_eq!(res[1].api_key, "key_2");
        assert_eq!(res[2].account_id, "acc_3");
        assert_eq!(res[2].api_key, "key_3");
    }

    #[test]
    fn test_mismatched_pair_count_error() {
        let accs = "acc_1, acc_2";
        let keys = "key_1";
        let err = parse_and_validate_pairs(accs, keys).unwrap_err();
        assert!(err.contains("Mismatched count"));
    }

    #[test]
    fn test_empty_credentials_error() {
        let err = parse_and_validate_pairs("  ,  ", "").unwrap_err();
        assert!(err.contains("No credentials provided"));
    }

    #[test]
    fn test_mask_account_id() {
        assert_eq!(mask_account_id("1234567890abcdef"), "1234...cdef");
        assert_eq!(mask_account_id("short"), "s****");
        assert_eq!(mask_account_id(""), "****");
    }

    #[test]
    fn test_failover_rotation_order() {
        let pairs = vec![
            CloudflareCredentialPair { account_id: "a1".into(), api_key: "k1".into() },
            CloudflareCredentialPair { account_id: "a2".into(), api_key: "k2".into() },
            CloudflareCredentialPair { account_id: "a3".into(), api_key: "k3".into() },
        ];
        let mut mgr = CredentialManager::new(pairs);

        // Initially preferred is 0 -> order is [0, 1, 2]
        assert_eq!(mgr.get_candidate_indices(), vec![0, 1, 2]);

        // Fail 0 with 429
        mgr.mark_failure(0, FailureCategory::RateLimit, Some(60));
        // Candidate list should prioritize available [1, 2], with 0 at the end
        assert_eq!(mgr.get_candidate_indices(), vec![1, 2, 0]);

        // Pair 1 succeeds -> preferred becomes 1
        mgr.mark_success(1);
        assert_eq!(mgr.preferred_index, 1);
        // Next request starts with 1
        assert_eq!(mgr.get_candidate_indices(), vec![1, 2, 0]);
    }

    #[test]
    fn test_extract_token_openai_format() {
        let json_str = r#"{"id":"chatcmpl-123","object":"chat.completion.chunk","model":"@cf/qwen/qwen3.8-27b","choices":[{"index":0,"delta":{"content":"Hello world"}}]}"#;
        let val: serde_json::Value = serde_json::from_str(json_str).unwrap();
        assert_eq!(extract_token_from_json(&val), Some("Hello world".to_string()));
    }

    #[test]
    fn test_extract_token_reasoning_delta() {
        let json_str = r#"{"choices":[{"index":0,"delta":{"reasoning_content":"Thinking step"}}]}"#;
        let val: serde_json::Value = serde_json::from_str(json_str).unwrap();
        assert_eq!(extract_token_from_json(&val), Some("Thinking step".to_string()));
    }

    #[test]
    fn test_extract_token_legacy_response_field() {
        let json_str = r#"{"response":"Direct response chunk"}"#;
        let val: serde_json::Value = serde_json::from_str(json_str).unwrap();
        assert_eq!(extract_token_from_json(&val), Some("Direct response chunk".to_string()));
    }

    #[test]
    fn test_extract_token_final_usage_chunk_is_none() {
        let json_str = r#"{"response":"","usage":{"prompt_tokens":53,"completion_tokens":49,"total_tokens":102}}"#;
        let val: serde_json::Value = serde_json::from_str(json_str).unwrap();
        assert_eq!(extract_token_from_json(&val), None);
    }

    #[test]
    fn test_is_time_sensitive_request() {
        assert!(is_time_sensitive_request("Give latest AI news."));
        assert!(is_time_sensitive_request("What happened in AI today?"));
        assert!(is_time_sensitive_request("Find recent developments in Godot."));
        assert!(is_time_sensitive_request("Search the web for current information about a company."));
        assert!(is_time_sensitive_request("Compare the latest versions of these libraries."));
        assert!(!is_time_sensitive_request("Explain what is recursion in computer science."));
        assert!(!is_time_sensitive_request("Write a hello world program in C++."));
    }

    #[test]
    fn test_parse_tool_call_from_text() {
        let text = "Thinking...\n<tool_call>{\"name\": \"google_search\", \"arguments\": {\"query\": \"latest AI news\"}}</tool_call>";
        let res = parse_tool_call_from_text(text);
        assert!(res.is_some());
        let (name, args) = res.unwrap();
        assert_eq!(name, "google_search");
        assert_eq!(args["query"], "latest AI news");
    }

    #[test]
    fn test_extract_tool_call_from_json() {
        let json_str = r#"{"tool_calls":[{"name":"google_search","arguments":{"query":"latest Godot release"}}]}"#;
        let val: serde_json::Value = serde_json::from_str(json_str).unwrap();
        let res = extract_tool_call_from_json(&val);
        assert!(res.is_some());
        let (name, args) = res.unwrap();
        assert_eq!(name, "google_search");
        assert_eq!(args["query"], "latest Godot release");
    }

    #[test]
    fn test_sanitize_tool_name() {
        assert_eq!(sanitize_tool_name("native::system_info"), "native_system_info");
        assert_eq!(sanitize_tool_name("Open URL in Browser"), "Open_URL_in_Browser");
        assert_eq!(sanitize_tool_name("mcp::github::get-issue"), "mcp_github_get_issue");
        assert_eq!(sanitize_tool_name(""), "tool");
    }

    #[test]
    fn test_accumulated_tool_call_from_streaming_deltas() {
        let chunk1: serde_json::Value = serde_json::from_str(
            r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"id":"call_abc123","type":"function","function":{"name":"native_system","arguments":""}}]}}]}"#
        ).unwrap();
        let chunk2: serde_json::Value = serde_json::from_str(
            r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"function":{"name":"_info","arguments":"{}"}}]}}]}"#
        ).unwrap();

        let mut acc = AccumulatedToolCall::default();
        acc.update_from_chunk(&chunk1);
        acc.update_from_chunk(&chunk2);

        assert_eq!(acc.id, "call_abc123");
        assert_eq!(acc.name, "native_system_info");
        assert!(acc.is_valid());
        assert_eq!(acc.parse_arguments(), serde_json::json!({}));
    }

    #[test]
    fn test_resolve_capability_mappings() {
        let caps = crate::plugins::registry::get_default_native_capabilities("d:\\test");
        let found = resolve_capability("native_system_info", &caps);
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, "native::system_info");

        let found_open = resolve_capability("native_open_url", &caps);
        assert!(found_open.is_some());
        assert_eq!(found_open.unwrap().id, "native::open_url");
    }
}
