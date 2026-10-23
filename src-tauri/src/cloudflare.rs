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
    pub role: String, // "system", "user", "assistant"
    pub content: String,
}

/// Tauri event payload for incremental stream chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunkPayload {
    pub conversation_id: String,
    pub message_id: String,
    pub chunk: String,
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

/// Execute real incremental chat streaming with automatic credential failover
pub async fn execute_streaming_chat(
    app: AppHandle,
    state: &CloudflareState,
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

    let mut all_errors: Vec<String> = Vec::new();

    for &idx in &candidates {
        if cancel_token.load(Ordering::SeqCst) {
            let _ = app.emit(
                "chat-stream-done",
                StreamDonePayload {
                    conversation_id: conversation_id.clone(),
                    message_id: message_id.clone(),
                    full_text: String::new(),
                    cancelled: true,
                    error: None,
                    prompt_tokens: None,
                    completion_tokens: None,
                    total_tokens: None,
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

        let req_body = serde_json::json!({
            "messages": messages,
            "stream": true,
            "max_tokens": max_tokens.unwrap_or(4096),
            "temperature": temperature.unwrap_or(0.2)
        });

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
                    format!("Pair #{} ({}): Cloudflare server error (HTTP {})", idx + 1, masked, code),
                ),
                _ => (
                    FailureCategory::PermanentRequestErr,
                    format!("Pair #{} ({}): HTTP {} error from Cloudflare", idx + 1, masked, code),
                ),
            };

            let mut mgr = state.manager.lock().await;
            mgr.mark_failure(idx, cat, retry_after);

            // If it's a permanent model error or malformed payload, do not retry other pairs
            if cat == FailureCategory::PermanentRequestErr {
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
            continue; // Attempt next credential pair
        }

        // Connection successful! Mark this pair as preferred
        {
            let mut mgr = state.manager.lock().await;
            mgr.mark_success(idx);
        }

        // Process SSE stream
        // Per specifications: once output has begun, do not silently restart using another pair
        let mut byte_stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut full_text = String::new();
        let mut stream_interrupted = false;
        let mut prompt_tokens: Option<u32> = None;
        let mut completion_tokens: Option<u32> = None;
        let mut total_tokens: Option<u32> = None;

        while let Some(chunk_res) = byte_stream.next().await {
            if cancel_token.load(Ordering::SeqCst) {
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
                Err(_e) => {
                    stream_interrupted = true;
                    break;
                }
            };

            let text_chunk = String::from_utf8_lossy(&bytes);
            buffer.push_str(&text_chunk);

            // Cloudflare Workers AI streams lines separated by \n or \n\n
            // Each event starts with "data: "
            while let Some(pos) = buffer.find('\n') {
                let line = buffer[..pos].trim().to_string();
                buffer = buffer[pos + 1..].to_string();

                if line.is_empty() {
                    continue;
                }

                if let Some(data_str) = line.strip_prefix("data:") {
                    let data_trimmed = data_str.trim();
                    if data_trimmed == "[DONE]" {
                        // Stream finished cleanly
                        buffer.clear();
                        break;
                    }

                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data_trimmed) {
                        if let Some(token) = extract_token_from_json(&parsed) {
                            if !token.is_empty() {
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

        // Handle any trailing data in buffer
        if !buffer.trim().is_empty() {
            let line = buffer.trim();
            if let Some(data_str) = line.strip_prefix("data:") {
                let data_trimmed = data_str.trim();
                if data_trimmed != "[DONE]" {
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data_trimmed) {
                        if let Some(token) = extract_token_from_json(&parsed) {
                            if !token.is_empty() {
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
            buffer.clear();
        }

        if stream_interrupted {
            let _ = app.emit(
                "chat-stream-done",
                StreamDonePayload {
                    conversation_id: conversation_id.clone(),
                    message_id: message_id.clone(),
                    full_text: full_text.clone(),
                    cancelled: false,
                    error: Some("Stream interrupted due to network disconnect. You can retry the request.".into()),
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
        return;
    }

    let combined_err = if all_errors.is_empty() {
        "All configured Cloudflare credential pairs failed.".into()
    } else {
        format!(
            "All configured credential pairs failed:\n• {}",
            all_errors.join("\n• ")
        )
    };

    let _ = app.emit(
        "chat-stream-done",
        StreamDonePayload {
            conversation_id: conversation_id.clone(),
            message_id: message_id.clone(),
            full_text: String::new(),
            cancelled: false,
            error: Some(combined_err),
            prompt_tokens: None,
            completion_tokens: None,
            total_tokens: None,
        },
    );

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
    conversation_id: String,
    message_id: String,
    messages: Vec<ChatMessage>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
) -> Result<(), String> {
    let state_arc = state.inner().clone();
    tokio::spawn(async move {
        execute_streaming_chat(
            app,
            &state_arc,
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
}
