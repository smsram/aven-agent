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
pub const DEFAULT_MODEL_ID: &str = "@cf/google/gemma-4-26b-a4b-it";
pub const LEGACY_MODEL_ID: &str = "@cf/qwen/qwen3.8-27b";
pub const MODEL_ID: &str = DEFAULT_MODEL_ID;

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

/// Chat message for Cloudflare Workers AI API payload supporting text and multimodal vision payloads
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String, // "system", "user", "assistant", "tool"
    pub content: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tool_calls: Option<Vec<serde_json::Value>>,
}

impl ChatMessage {
    pub fn new_text(role: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            content: serde_json::Value::String(content.into()),
            name: None,
            tool_call_id: None,
            tool_calls: None,
        }
    }

    pub fn new_multimodal(role: impl Into<String>, text: &str, image_base64: &str, format: &str) -> Self {
        let fmt = if format.is_empty() { "jpeg" } else { format };
        Self {
            role: role.into(),
            content: serde_json::json!([
                { "type": "text", "text": text },
                { "type": "image_url", "image_url": { "url": format!("data:image/{fmt};base64,{image_base64}") } }
            ]),
            name: None,
            tool_call_id: None,
            tool_calls: None,
        }
    }

    pub fn text_content(&self) -> String {
        match &self.content {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Array(arr) => {
                let mut parts = Vec::new();
                for item in arr {
                    if let Some(t) = item.get("text").and_then(|v| v.as_str()) {
                        parts.push(t);
                    }
                }
                parts.join("\n")
            }
            other => other.to_string(),
        }
    }
}

/// Structured execution diagnostics for desktop automation actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredDiagnostic {
    pub step: usize,
    pub selected_method: String,
    pub target_id: Option<String>,
    pub action: String,
    pub result: String, // "success", "failed", "unverified"
    pub verification_evidence: Option<String>,
    pub recovery_reason: Option<String>,
}

/// Summary metrics tracking automation operations and reliability diagnostics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct AutomationMetrics {
    pub screenshot_count: usize,
    pub uia_query_count: usize,
    pub model_requests_count: usize,
    pub retry_count: usize,
    pub verification_success_count: usize,
    pub verification_failure_count: usize,
    pub interaction_methods_used: Vec<String>,
}

/// Classify the interaction method being executed
pub fn determine_interaction_method(tool_name: &str, tool_args: &serde_json::Value) -> String {
    let lower = tool_name.to_lowercase();
    if lower.contains("inspect_ui") {
        let action = tool_args.get("action").and_then(|v| v.as_str()).unwrap_or("");
        if action == "invoke" || action == "set_value" || action == "select_item" || action == "expand_collapse" {
            "uia_semantic".to_string()
        } else {
            "uia_query".to_string()
        }
    } else if lower.contains("keyboard_action") {
        "native_keyboard".to_string()
    } else if lower.contains("mouse_action") {
        if tool_args.get("image_x").is_some() || tool_args.get("image_y").is_some() {
            "vision_coordinate_mouse".to_string()
        } else {
            "desktop_coordinate_mouse".to_string()
        }
    } else if lower.contains("capture_screenshot") {
        "vision_perception".to_string()
    } else if lower.contains("launch_app") || lower.contains("window_action") {
        "native_window_api".to_string()
    } else {
        "integration_api".to_string()
    }
}

/// Redact passwords, secrets, and raw base64 data strings from tool arguments before logging
pub fn redact_sensitive_tool_args(tool_name: &str, args: &serde_json::Value) -> serde_json::Value {
    let mut cloned = args.clone();
    if let Some(obj) = cloned.as_object_mut() {
        if obj.contains_key("image_base64") {
            obj.insert("image_base64".to_string(), serde_json::json!("[image_base64_omitted]"));
        }
        if obj.contains_key("image_data") {
            obj.insert("image_data".to_string(), serde_json::json!("[image_data_omitted]"));
        }
        for (k, v) in obj.iter_mut() {
            let k_lower = k.to_lowercase();
            if k_lower.contains("password")
                || k_lower.contains("secret")
                || k_lower.contains("token")
                || k_lower.contains("pin")
                || k_lower.contains("credential")
            {
                *v = serde_json::json!("***REDACTED***");
            }
        }
        if tool_name.contains("keyboard") {
            if let Some(q) = obj.get("element_query").and_then(|v| v.as_str()) {
                let q_lower = q.to_lowercase();
                if q_lower.contains("pass") || q_lower.contains("pwd") || q_lower.contains("pin") {
                    if obj.contains_key("text") {
                        obj.insert("text".to_string(), serde_json::json!("***REDACTED***"));
                    }
                }
            }
        }
    }
    cloned
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

/// Check if a user request asks for desktop action or execution
pub fn is_action_request(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();

    // Distinguish creative/conversational writing from desktop UI control
    if (lower.starts_with("write a poem")
        || lower.starts_with("write an essay")
        || lower.starts_with("write a story")
        || lower.starts_with("write code")
        || lower.starts_with("write a function")
        || lower.starts_with("write a script"))
        && !lower.contains("notepad")
        && !lower.contains("file")
        && !lower.contains("app")
        && !lower.contains("open")
    {
        return false;
    }

    let action_phrases = [
        "open", "launch", "type ", "type \"", "type '", "click", "notepad", "whatsapp",
        "calculator", "calc", "chrome", "edge", "browser", "window", "close ", "press ",
        "send ", "start menu", "screenshot", "capture screen", "take a screenshot",
        "desktop", "mouse", "keyboard", "double click", "right click", "drag",
    ];

    action_phrases.iter().any(|&k| lower.contains(k))
}

/// Strictly sanitize user-visible activity descriptions so raw HTML, source code,
/// internal prompts, serialized payloads, and unrelated file contents cannot appear.
pub fn sanitize_activity_label(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return "Analyzing screen".to_string();
    }

    let lower = trimmed.to_lowercase();
    if lower.contains("<!doctype")
        || lower.contains("<html")
        || lower.contains("<body")
        || lower.contains("<div")
        || lower.contains("<svg")
        || lower.contains("<script")
        || lower.contains("<style")
        || lower.contains("<?xml")
        || (trimmed.starts_with('{') && trimmed.ends_with('}'))
        || (trimmed.starts_with('[') && trimmed.ends_with(']'))
    {
        return "Analyzing screen".to_string();
    }

    // Strip any HTML angle brackets
    let mut clean = String::new();
    let mut in_tag = false;
    for c in trimmed.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            clean.push(c);
        }
    }

    let first_line = clean.lines().next().unwrap_or("").trim();
    if first_line.is_empty() {
        return "Analyzing screen".to_string();
    }

    if first_line.len() > 45 {
        let mut truncated = first_line.chars().take(42).collect::<String>();
        truncated.push_str("...");
        truncated
    } else {
        first_line.to_string()
    }
}

/// Detect if model output indicates future or pending intent rather than task completion
pub fn indicates_pending_action(text: &str) -> bool {
    let lower = text.to_lowercase();
    let intent_phrases = [
        "now i will", "now i'll", "now i am", "now i'm",
        "i will now", "i'll now", "i am now", "i'm now",
        "next i will", "next, i will", "next i'll", "next, i'll",
        "next we will", "next, we will", "next we'll", "next, we'll",
        "let me now", "let us now", "now let me", "now let's", "let's now",
        "i will add", "i'll add", "now add ", "now adding",
        "i will apply", "i'll apply", "now apply ", "now applying",
        "i will insert", "i'll insert", "now insert ", "now inserting",
        "i will select", "i'll select", "now select ", "now selecting",
        "i will click", "i'll click", "now click ", "now clicking",
        "i will type", "i'll type", "now type ", "now typing",
        "i will write", "i'll write", "now write ", "now writing",
        "i will open", "i'll open", "now open ", "now opening",
        "i will focus", "i'll focus", "now focus ", "now focusing",
        "i will navigate", "i'll navigate", "now navigate ", "now navigating",
        "i will proceed", "i'll proceed", "proceeding to",
        "i will then", "then i will", "then i'll", "moving on to",
        "about to", "going to add", "going to type", "going to click",
        "the next step is", "first, i will", "first i will", "first i'll",
        "next step:", "next step is to",
    ];
    intent_phrases.iter().any(|&p| lower.contains(p))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateOutcome {
    AllowCompletion,
    RequireContinuation {
        prompt: String,
        reason: &'static str,
    },
}

pub fn map_tool_to_factual_label(tool_name: &str, tool_args: &serde_json::Value) -> String {
    format_tool_activity_description(tool_name, tool_args, "")
}

pub fn evaluate_task_completion_gate(
    user_prompt: &str,
    executed_tools: &[String],
    _current_step: usize,
    _max_steps: usize,
    turn_text: &str,
) -> GateOutcome {
    if !is_action_request(user_prompt) {
        return GateOutcome::AllowCompletion;
    }

    // Do NOT automatically allow completion based on arbitrary step counts.
    // Tasks must continue until their requested outcome is verified with evidence,
    // or an explicit terminal failure/recovery blocker is identified.

    let lower_prompt = user_prompt.to_lowercase();
    let _lower_turn = turn_text.to_lowercase();

    // 1. If model explicitly indicates pending or future intent, prompt it to execute immediately
    if indicates_pending_action(turn_text) {
        return GateOutcome::RequireContinuation {
            prompt: "Please proceed immediately with executing the next planned step by calling the required desktop tool directly. Do not stop until all actions are executed.".to_string(),
            reason: "model_indicated_pending_action",
        };
    }

    // 2. Check Launch requirement
    let wants_launch = lower_prompt.contains("open ")
        || lower_prompt.contains("launch ")
        || lower_prompt.contains("start ")
        || lower_prompt.contains("word")
        || lower_prompt.contains("winword")
        || lower_prompt.contains("notepad")
        || lower_prompt.contains("calculator")
        || lower_prompt.contains("calc")
        || lower_prompt.contains("chrome")
        || lower_prompt.contains("edge")
        || lower_prompt.contains("explorer");
    let has_launched = executed_tools.iter().any(|t| t.contains("launch_app"));
    if wants_launch && !has_launched {
        let app_hint = if lower_prompt.contains("word") || lower_prompt.contains("winword") {
            "winword"
        } else if lower_prompt.contains("notepad") {
            "notepad"
        } else if lower_prompt.contains("calc") {
            "calc"
        } else if lower_prompt.contains("chrome") {
            "chrome"
        } else if lower_prompt.contains("edge") {
            "msedge"
        } else if lower_prompt.contains("explorer") {
            "explorer"
        } else {
            "the application"
        };
        return GateOutcome::RequireContinuation {
            prompt: format!("The requested application has not been launched yet. Please call desktop_control_launch_app with app_name: '{app_hint}' now."),
            reason: "unexecuted_launch",
        };
    }

    // 3. Check Typing / Introduction requirement
    let wants_type = lower_prompt.contains("type ")
        || lower_prompt.contains("write ")
        || lower_prompt.contains("type \"")
        || lower_prompt.contains("type '")
        || lower_prompt.contains("enter ")
        || lower_prompt.contains("introduce")
        || lower_prompt.contains("introduction");
    let has_typed = executed_tools.iter().any(|t| t.contains("keyboard") || (t.contains("input") && t.contains("type")));
    if wants_type && !has_typed {
        return GateOutcome::RequireContinuation {
            prompt: "The requested text has not been typed yet. Please call desktop_control_keyboard_action with action: 'type_text' to type the requested text into the active target now.".to_string(),
            reason: "unexecuted_typing",
        };
    }

    // 4. Check Document Formatting / Borders requirement
    let wants_borders = lower_prompt.contains("border")
        || lower_prompt.contains("borders")
        || lower_prompt.contains("page border")
        || lower_prompt.contains("page borders")
        || lower_prompt.contains("margin")
        || lower_prompt.contains("watermark");
    let border_interactions = executed_tools.iter().filter(|t| t.contains("mouse") || t.contains("keyboard") || t.contains("inspect_ui")).count();
    let has_border_verification = executed_tools.iter().any(|t| t.contains("capture_screenshot") || t.contains("inspect_ui"));
    if wants_borders && (border_interactions < 3 || !has_border_verification) {
        return GateOutcome::RequireContinuation {
            prompt: "The user requested page borders ('make sure the page have borders'). Please navigate to the Design tab or Page Borders control in Word, apply the page border, and capture a screenshot to visually verify that the page border is applied before completing.".to_string(),
            reason: "unexecuted_borders",
        };
    }

    // 5. Check Click / Interaction requirement
    let wants_click = lower_prompt.contains("click ")
        || lower_prompt.contains("press ")
        || lower_prompt.contains("select ")
        || lower_prompt.contains("choose ");
    let has_clicked = executed_tools.iter().any(|t| t.contains("mouse"));
    if wants_click && !has_clicked {
        return GateOutcome::RequireContinuation {
            prompt: "The requested click/selection has not been performed yet. Please capture a screenshot with desktop_control_capture_screenshot to locate the control visually, then click it using desktop_control_mouse_action.".to_string(),
            reason: "unexecuted_click",
        };
    }

    // 6. Check Save / Dialog handling requirement
    let wants_save = lower_prompt.contains("save") || lower_prompt.contains("save as") || lower_prompt.contains("file picker");
    let interaction_count = executed_tools.iter().filter(|t| !t.contains("capture_screenshot")).count();
    if wants_save && interaction_count < 2 && !_lower_turn.contains("saved") {
        return GateOutcome::RequireContinuation {
            prompt: "The requested save operation requires saving the file or confirming the dialog. Please proceed with triggering Save (e.g. Ctrl+S or clicking Save), entering the filename if needed, and confirming.".to_string(),
            reason: "unexecuted_save",
        };
    }

    // 7. Action-by-action Verification Evidence Gate:
    // If meaningful desktop actions were executed (e.g. typing or clicking), but no follow-up screenshot
    // has been captured to verify the resulting screen state and confirm the outcome:
    let last_tool = executed_tools.last().map(|s| s.as_str()).unwrap_or("");
    let is_state_altering = last_tool.contains("keyboard") || last_tool.contains("mouse") || last_tool.contains("launch_app");
    let has_recent_verification = executed_tools.iter().rev().take(2).any(|t| t.contains("capture_screenshot") || t.contains("wait_for_ui"));

    if is_state_altering && !has_recent_verification {
        return GateOutcome::RequireContinuation {
            prompt: "Desktop action was executed. Please call desktop_control_capture_screenshot now to observe the screen, verify the outcome with visual evidence, and confirm the task is complete.".to_string(),
            reason: "unverified_desktop_state",
        };
    }

    GateOutcome::AllowCompletion
}

/// Robust JSON string repair for fragmented, unclosed, or streaming tool call arguments
pub fn repair_json_string(raw: &str) -> serde_json::Value {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return serde_json::json!({});
    }

    // Strip markdown code block wrappers if present
    let cleaned = if (trimmed.starts_with("```json") || trimmed.starts_with("```")) && trimmed.ends_with("```") {
        let after_fence = if let Some(stripped) = trimmed.strip_prefix("```json") {
            stripped
        } else {
            trimmed.strip_prefix("```").unwrap_or(trimmed)
        };
        let before_end = after_fence.strip_suffix("```").unwrap_or(after_fence);
        before_end.trim()
    } else {
        trimmed
    };

    // 1. Direct parse attempt
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(cleaned) {
        return val;
    }

    // 2. Try balancing unclosed braces and quotes
    let mut candidate = cleaned.to_string();

    // Remove trailing commas before closing braces: e.g. ", }" -> " }"
    while candidate.contains(",}") {
        candidate = candidate.replace(",}", "}");
    }
    while candidate.contains(", ]") || candidate.contains(",]") {
        candidate = candidate.replace(", ]", "]").replace(",]", "]");
    }

    // If dangling trailing comma at the end: e.g. `{"app_name": "notepad",`
    if candidate.trim_end().ends_with(',') {
        let trimmed_c = candidate.trim_end();
        candidate = trimmed_c[..trimmed_c.len() - 1].to_string();
    }

    // Check quote parity: if unescaped quotes count is odd, append quote
    let mut quote_count = 0;
    let mut prev_char = ' ';
    for c in candidate.chars() {
        if c == '"' && prev_char != '\\' {
            quote_count += 1;
        }
        prev_char = c;
    }
    if quote_count % 2 != 0 {
        candidate.push('"');
    }

    // Balance braces and brackets
    let mut open_braces: usize = 0;
    let mut open_brackets: usize = 0;
    let mut in_str = false;
    let mut last_c = ' ';
    for c in candidate.chars() {
        if c == '"' && last_c != '\\' {
            in_str = !in_str;
        } else if !in_str {
            match c {
                '{' => open_braces += 1,
                '}' => open_braces = open_braces.saturating_sub(1),
                '[' => open_brackets += 1,
                ']' => open_brackets = open_brackets.saturating_sub(1),
                _ => {}
            }
        }
        last_c = c;
    }

    for _ in 0..open_brackets {
        candidate.push(']');
    }
    for _ in 0..open_braces {
        candidate.push('}');
    }

    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&candidate) {
        return val;
    }

    // 3. Heuristic extraction of common parameters if JSON syntax is still slightly off
    let mut extracted = serde_json::Map::new();
    let common_keys = ["app_name", "text", "action", "window_id", "query", "url", "path", "command"];
    for key in &common_keys {
        let key_pattern = format!("\"{}\":", key);
        if let Some(pos) = cleaned.find(&key_pattern) {
            let rest = cleaned[pos + key_pattern.len()..].trim_start();
            if rest.starts_with('"') {
                let rest_str = &rest[1..];
                if let Some(quote_end) = rest_str.find('"') {
                    extracted.insert(key.to_string(), serde_json::json!(&rest_str[..quote_end]));
                }
            } else if let Some(num_end) = rest.find(|c: char| !c.is_numeric() && c != '.') {
                if let Ok(n) = rest[..num_end].parse::<i64>() {
                    extracted.insert(key.to_string(), serde_json::json!(n));
                }
            }
        }
    }

    if !extracted.is_empty() {
        return serde_json::Value::Object(extracted);
    }

    serde_json::json!({})
}

/// Parse in-text tool call format like <tool_call>{"name": "...", "arguments": {...}}</tool_call>
/// or markdown ```json ... ``` blocks containing a tool name and arguments/parameters
pub fn parse_tool_call_from_text(text: &str) -> Option<(String, serde_json::Value)> {
    // 1. Look for <tool_call>...</tool_call> tag
    if let Some(start) = text.find("<tool_call>") {
        let after = &text[start + 11..];
        if let Some(end) = after.find("</tool_call>") {
            let json_str = after[..end].trim();
            let val = repair_json_string(json_str);
            if let Some(name) = val.get("name").and_then(|v| v.as_str()) {
                let args = val.get("arguments")
                    .or_else(|| val.get("parameters"))
                    .cloned()
                    .unwrap_or(serde_json::json!({}));
                return Some((name.to_string(), args));
            }
        }
    }

    // 2. Look for markdown fenced json blocks ```json { "name": ... } ```
    let mut search_idx = 0;
    while let Some(code_start) = text[search_idx..].find("```") {
        let actual_start = search_idx + code_start + 3;
        let rest = &text[actual_start..];
        let content_start = if let Some(first_nl) = rest.find('\n') {
            actual_start + first_nl + 1
        } else {
            actual_start
        };

        if let Some(code_end) = text[content_start..].find("```") {
            let block = text[content_start..content_start + code_end].trim();
            let val = repair_json_string(block);
            if let Some(name) = val.get("name").and_then(|v| v.as_str()) {
                let args = val.get("arguments")
                    .or_else(|| val.get("parameters"))
                    .cloned()
                    .unwrap_or(serde_json::json!({}));
                return Some((name.to_string(), args));
            }
            search_idx = content_start + code_end + 3;
        } else {
            break;
        }
    }

    // 3. Look for raw JSON object { "name": "desktop_control_...", "arguments": ... }
    if let Some(first_brace) = text.find('{') {
        if let Some(last_brace) = text.rfind('}') {
            if last_brace > first_brace {
                let candidate = &text[first_brace..=last_brace];
                let val = repair_json_string(candidate);
                if let Some(name) = val.get("name").and_then(|v| v.as_str()) {
                    if name.starts_with("desktop_control_") || name.starts_with("native_") || name == "google_search" {
                        let args = val.get("arguments")
                            .or_else(|| val.get("parameters"))
                            .cloned()
                            .unwrap_or(serde_json::json!({}));
                        return Some((name.to_string(), args));
                    }
                }
            }
        }
    }

    None
}

/// Strip tool call tags and blocks from user-visible streaming text
pub fn strip_tool_calls_from_text(text: &str) -> String {
    let mut result = text.to_string();

    // Strip <tool_call>...</tool_call>
    while let Some(start) = result.find("<tool_call>") {
        if let Some(end) = result[start..].find("</tool_call>") {
            result.replace_range(start..start + end + 12, "");
        } else {
            result.truncate(start);
            break;
        }
    }

    // Strip code fenced json tool blocks ```json\n{"name": ...}\n```
    while let Some(start) = result.find("```") {
        if let Some(end_offset) = result[start + 3..].find("```") {
            let block_end = start + 3 + end_offset + 3;
            let inner = &result[start..block_end];
            if inner.contains("\"name\"") && (inner.contains("\"arguments\"") || inner.contains("\"parameters\"") || inner.contains("desktop_control_") || inner.contains("native_")) {
                result.replace_range(start..block_end, "");
            } else {
                break;
            }
        } else {
            if result[start..].contains("\"name\"") {
                result.truncate(start);
            }
            break;
        }
    }

    result.trim().to_string()
}

/// Strip internal reasoning and thinking tags from user-visible streaming text
pub fn strip_reasoning_from_text(text: &str) -> String {
    let mut result = text.to_string();
    while let Some(start) = result.find("<thought>") {
        if let Some(end) = result[start..].find("</thought>") {
            result.replace_range(start..start + end + 10, "");
        } else {
            result.truncate(start);
            break;
        }
    }
    while let Some(start) = result.find("<thinking>") {
        if let Some(end) = result[start..].find("</thinking>") {
            result.replace_range(start..start + end + 11, "");
        } else {
            result.truncate(start);
            break;
        }
    }
    result.trim().to_string()
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
                                        Some(repair_json_string(s))
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
                                    if !n.is_empty() {
                                        if self.name.is_empty() {
                                            self.name = n.to_string();
                                        } else if !self.name.contains(n) && self.name.len() < 64 {
                                            self.name.push_str(n);
                                        }
                                    }
                                }
                                if let Some(args) = func.get("arguments").and_then(|v| v.as_str()) {
                                    self.arguments_buf.push_str(args);
                                }
                            } else {
                                if let Some(n) = tc.get("name").and_then(|v| v.as_str()) {
                                    if !n.is_empty() {
                                        if self.name.is_empty() {
                                            self.name = n.to_string();
                                        } else if !self.name.contains(n) && self.name.len() < 64 {
                                            self.name.push_str(n);
                                        }
                                    }
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
                        if !n.is_empty() {
                            if self.name.is_empty() {
                                self.name = n.to_string();
                            } else if !self.name.contains(n) && self.name.len() < 64 {
                                self.name.push_str(n);
                            }
                        }
                    }
                    if let Some(args) = func.get("arguments").and_then(|v| v.as_str()) {
                        self.arguments_buf.push_str(args);
                    }
                } else {
                    if let Some(n) = tc.get("name").and_then(|v| v.as_str()) {
                        if !n.is_empty() {
                            if self.name.is_empty() {
                                self.name = n.to_string();
                            } else if !self.name.contains(n) && self.name.len() < 64 {
                                self.name.push_str(n);
                            }
                        }
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
        repair_json_string(&self.arguments_buf)
    }
}

/// Sanitize capability IDs into valid tool identifiers (letters, numbers, underscores only)
pub fn sanitize_tool_name(id: &str) -> String {
    let sanitized = id
        .replace("::", "_")
        .replace(':', "_")
        .replace('-', "_")
        .replace('.', "_")
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
            || (c.source_id == "native_desktop_control" && (
                clean == c.id.to_lowercase()
                || clean == format!("native_{}", c.id.to_lowercase())
                || clean.ends_with(&c.id.to_lowercase())
                || (c.id == "desktop_control_inspect_ui" && (
                    clean == "read_ui"
                    || clean == "readui"
                    || clean == "computer_read_ui"
                    || clean == "computer_readui"
                    || clean == "computerreadui"
                    || clean == "find_element"
                    || clean == "findelement"
                    || clean == "computer_find_element"
                    || clean == "computer_findelement"
                    || clean == "computerfindelement"
                    || clean == "find_elements"
                    || clean == "findelements"
                    || clean == "inspect_ui"
                    || clean == "inspectui"
                    || clean == "get_focused_control"
                    || clean == "get_focused_element"
                    || clean == "focused_control"
                    || clean == "inspect_control"
                    || clean == "set_value"
                ))
                || (c.id == "desktop_control_mouse_action" && (
                    clean == "click"
                    || clean == "computer_click"
                    || clean == "computerclick"
                    || clean == "mouse_click"
                    || clean == "mouse_action"
                    || clean == "mouse"
                    || clean == "computer_mouse"
                    || clean == "scroll"
                    || clean == "computer_scroll"
                    || clean == "computerscroll"
                    || clean == "mouse_scroll"
                    || clean == "double_click"
                    || clean == "right_click"
                ))
                || (c.id == "desktop_control_keyboard_action" && (
                    clean == "type"
                    || clean == "type_text"
                    || clean == "typetext"
                    || clean == "computer_type"
                    || clean == "computertype"
                    || clean == "computer_type_text"
                    || clean == "computertypetext"
                    || clean == "keyboard"
                    || clean == "keyboard_action"
                    || clean == "computer_keyboard"
                    || clean == "press_key"
                    || clean == "shortcut"
                    || clean == "send_keys"
                ))
                || (c.id == "desktop_control_capture_screenshot" && (
                    clean == "screenshot"
                    || clean == "capture_screenshot"
                    || clean == "take_screenshot"
                    || clean == "computer_screenshot"
                ))
                || (c.id == "desktop_control_window_action" && (
                    clean == "window_action"
                    || clean == "window"
                    || clean == "computer_window"
                    || clean == "focus_window"
                    || clean == "activate_window"
                    || clean == "minimize_window"
                    || clean == "maximize_window"
                    || clean == "close_window"
                ))
                || (c.id == "desktop_control_launch_app" && (
                    clean == "launch_app"
                    || clean == "native_launch_app"
                    || clean == "native_app_launcher"
                    || clean == "open_app"
                    || clean == "open_application"
                ))
            ))
    })
}

/// Check if an AI model supports multimodal vision/image perception
pub fn is_vision_model(model_id: &str) -> bool {
    let lower = model_id.to_lowercase();
    lower.contains("vision")
        || lower.contains("uform")
        || lower.contains("llava")
        || lower.contains("gpt-4o")
        || lower.contains("gemini-1.5")
        || lower.contains("gemini-2")
        || lower.contains("claude-3")
}

/// Normalize tool arguments when models use common shorthand patterns
pub fn normalize_tool_arguments(
    tool_name: &str,
    capability_id: &str,
    mut args: serde_json::Value,
) -> serde_json::Value {
    let clean = sanitize_tool_name(tool_name).to_lowercase();
    if capability_id == "desktop_control_inspect_ui" {
        if args.get("action").is_none() {
            if clean.contains("find_element") || clean.contains("findelement") {
                args["action"] = serde_json::json!("find_element");
            } else if clean.contains("inspect_window") || clean.contains("inspectwindow") {
                args["action"] = serde_json::json!("inspect_window");
            } else if clean.contains("read_text") || clean.contains("readtext") {
                args["action"] = serde_json::json!("read_text");
            } else if clean.contains("check_state") || clean.contains("checkstate") {
                args["action"] = serde_json::json!("check_state");
            } else if clean.contains("get_element_bounds") || clean.contains("bounds") {
                args["action"] = serde_json::json!("get_element_bounds");
            } else if clean.contains("revalidate") {
                args["action"] = serde_json::json!("revalidate_element");
            } else if clean.contains("focused") {
                args["action"] = serde_json::json!("get_focused_control");
            } else if clean.contains("scroll") {
                args["action"] = serde_json::json!("scroll");
            } else if clean.contains("read") {
                args["action"] = serde_json::json!("read_tree");
            } else {
                args["action"] = serde_json::json!("get_focused_control");
            }
        }
        if args.get("query").is_none() {
            if let Some(target) = args.get("target").or_else(|| args.get("element")).or_else(|| args.get("name")) {
                args["query"] = target.clone();
            }
        }
    } else if capability_id == "desktop_control_mouse_action" {
        if args.get("action").is_none() {
            if clean.contains("scroll") {
                args["action"] = serde_json::json!("scroll");
            } else if clean.contains("double") {
                args["action"] = serde_json::json!("double_click");
            } else if clean.contains("right") {
                args["action"] = serde_json::json!("right_click");
            } else {
                args["action"] = serde_json::json!("click");
            }
        }
    } else if capability_id == "desktop_control_keyboard_action" {
        if args.get("action").is_none() {
            if clean.contains("shortcut") {
                args["action"] = serde_json::json!("shortcut");
            } else if clean.contains("press") {
                args["action"] = serde_json::json!("key_press");
            } else {
                args["action"] = serde_json::json!("type_text");
            }
        }
        if args.get("text").is_none() {
            if let Some(val) = args.get("content").or_else(|| args.get("value")).or_else(|| args.get("input")) {
                args["text"] = val.clone();
            }
        }
        if args.get("element_query").is_none() {
            if let Some(target) = args.get("query").or_else(|| args.get("target")).or_else(|| args.get("element")) {
                args["element_query"] = target.clone();
            }
        }
    } else if capability_id == "desktop_control_window_action" {
        if args.get("action").is_none() {
            if clean.contains("focus") || clean.contains("activate") {
                args["action"] = serde_json::json!("focus");
            } else if clean.contains("minimize") {
                args["action"] = serde_json::json!("minimize");
            } else if clean.contains("maximize") {
                args["action"] = serde_json::json!("maximize");
            } else if clean.contains("close") {
                args["action"] = serde_json::json!("close");
            }
        }
        if args.get("window_id").is_none() {
            if let Some(hwnd) = args.get("hwnd").or_else(|| args.get("window")).or_else(|| args.get("id")) {
                args["window_id"] = hwnd.clone();
            }
        }
    }
    args
}

/// Compute clear, human-readable real-time activity status for tool executions
pub fn format_tool_activity_description(
    tool_name: &str,
    tool_args: &serde_json::Value,
    _display_name: &str,
) -> String {
    let lower = tool_name.to_lowercase();
    let label = if lower.contains("capture_screenshot") {
        "Capturing screen".to_string()
    } else if lower.contains("get_screen_info") {
        "Analyzing screen".to_string()
    } else if lower.contains("file_access") {
        let action = tool_args.get("action").and_then(|v| v.as_str()).unwrap_or("read");
        if action == "list" {
            "Listing directory".to_string()
        } else {
            "Reading file".to_string()
        }
    } else if lower.contains("system_info") {
        "Inspecting system".to_string()
    } else if lower.contains("open_url") {
        "Opening browser".to_string()
    } else if lower.contains("launch_app") {
        let app_name = tool_args
            .get("app_name")
            .or_else(|| tool_args.get("app_path"))
            .and_then(|v| v.as_str())
            .unwrap_or("application");
        let safe_app = if app_name.contains("word") || app_name.contains("winword") {
            "Microsoft Word"
        } else if app_name.contains("notepad") {
            "Notepad"
        } else if app_name.contains("calc") {
            "Calculator"
        } else if app_name.contains("edge") {
            "Microsoft Edge"
        } else if app_name.contains("chrome") {
            "Google Chrome"
        } else if app_name.contains("explorer") {
            "File Explorer"
        } else {
            "application"
        };
        format!("Opening {safe_app}")
    } else if lower.contains("keyboard") {
        let action = tool_args.get("action").and_then(|v| v.as_str()).unwrap_or("type_text");
        if action == "type_text" {
            "Typing".to_string()
        } else if action == "key_combination" || action == "press_key" {
            "Typing".to_string()
        } else {
            "Typing".to_string()
        }
    } else if lower.contains("mouse") {
        let target_desc = tool_args.get("target_description").and_then(|v| v.as_str());
        if let Some(desc) = target_desc {
            let dl = desc.to_lowercase();
            if dl.contains("design") {
                "Opening Design tab".to_string()
            } else if dl.contains("border") {
                "Applying Page Borders".to_string()
            } else if dl.contains("save") || dl.contains("picker") || dl.contains("dialog") {
                "Opening file picker".to_string()
            } else if dl.contains("file") || dl.contains("item") {
                "Selecting file".to_string()
            } else {
                let sanitized_desc = sanitize_activity_label(desc);
                if sanitized_desc.len() > 25 {
                    "Finding target".to_string()
                } else {
                    format!("Finding target: {sanitized_desc}")
                }
            }
        } else {
            "Clicking".to_string()
        }
    } else if lower.contains("wait_for_ui") {
        "Waiting for application".to_string()
    } else if lower.contains("inspect_ui") {
        let action = tool_args.get("action").and_then(|v| v.as_str()).unwrap_or("inspect_window");
        match action {
            "find_element" | "get_focused_control" | "get_focused_element" => "Searching UIA".to_string(),
            "get_element_bounds" => "Finding target".to_string(),
            "invoke" | "click_element" => "Clicking".to_string(),
            "set_value" => "Typing".to_string(),
            "revalidate_element" | "check_state" => "Verifying result".to_string(),
            _ => "Verifying result".to_string(),
        }
    } else if lower.contains("window_action") {
        "Managing window".to_string()
    } else if lower.contains("search") {
        "Searching web".to_string()
    } else {
        "Analyzing screen".to_string()
    };

    sanitize_activity_label(&label)
}

/// Extract user-facing visible text tokens from Cloudflare Workers AI streaming chunk formats.
/// Never extracts or exposes internal reasoning, thinking, or scratchpad content.
pub fn extract_visible_token(val: &serde_json::Value) -> Option<String> {
    // 1. OpenAI-compatible SSE: choices[0].delta.content (IGNORE delta.reasoning_content)
    if let Some(choices) = val.get("choices").and_then(|v| v.as_array()) {
        let mut token = String::new();
        for choice in choices {
            if let Some(delta) = choice.get("delta") {
                if let Some(c) = delta.get("content").and_then(|v| v.as_str()) {
                    token.push_str(c);
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

    // 3. Cloudflare result wrapper
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

    // 4. Fallback fields
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

/// Extract internal reasoning or thinking tokens for real-time status reporting without user exposure
pub fn extract_reasoning_from_json(val: &serde_json::Value) -> Option<String> {
    if let Some(choices) = val.get("choices").and_then(|v| v.as_array()) {
        for choice in choices {
            if let Some(delta) = choice.get("delta") {
                if let Some(rc) = delta.get("reasoning_content").and_then(|v| v.as_str()) {
                    if !rc.is_empty() {
                        return Some(rc.to_string());
                    }
                }
            }
        }
    }

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

    None
}

/// User visible token extraction function
pub fn extract_token_from_json(val: &serde_json::Value) -> Option<String> {
    extract_visible_token(val)
}

/// Failure classification for failover decision
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureCategory {
    AuthFailure,         // 401/403
    PermissionOrAccess,  // 404 account/model access, 403 restricted access
    RateLimit,           // 429
    QuotaExhausted,      // 402, 429 quota exhaustion
    ServerOrNetwork,     // 5xx, timeout, connect err
    PermanentRequestErr, // 400 with unrecoverable syntax
}

/// Parse and validate comma-separated Account IDs and API Keys.
/// Rules:
/// - Trims whitespace around every entry.
/// - Ignores empty entries caused by leading, trailing, or repeated commas.
/// - Requires at least one valid credential pair.
/// - Supports 1 Account ID with multiple API Keys or N Account IDs with N API Keys.
/// - Deduplicates identical pairs while preserving order.
/// - Rejects internal whitespace inside individual keys or account IDs.
pub fn parse_and_validate_pairs(
    account_ids_raw: &str,
    api_keys_raw: &str,
) -> Result<Vec<CloudflareCredentialPair>, String> {
    let mut accounts: Vec<String> = account_ids_raw
        .split([',', '\n', '\r'])
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let keys: Vec<String> = api_keys_raw
        .split([',', '\n', '\r'])
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if accounts.is_empty() && keys.is_empty() {
        return Err("No credentials provided. Please enter at least one Account ID and API Key.".into());
    }

    if accounts.is_empty() {
        return Err("At least one Account ID is required.".into());
    }

    if keys.is_empty() {
        return Err("At least one API Key is required.".into());
    }

    // Support single Account ID configured with multiple API Keys
    if accounts.len() == 1 && keys.len() > 1 {
        let single_acc = accounts[0].clone();
        accounts = vec![single_acc; keys.len()];
    }

    if accounts.len() != keys.len() {
        return Err(format!(
            "Mismatched count: {} Account ID(s) provided, but {} API Key(s) provided. Each Account ID must have a matching API Key.",
            accounts.len(),
            keys.len()
        ));
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
        // Deduplicate identical pairs while preserving configured order
        if !pairs.iter().any(|p: &CloudflareCredentialPair| p.account_id == acc && p.api_key == key) {
            pairs.push(CloudflareCredentialPair {
                account_id: acc,
                api_key: key,
            });
        }
    }

    if pairs.is_empty() {
        return Err("At least one valid credential pair is required.".into());
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
            if let Ok(pairs) = serde_json::from_str::<Vec<CloudflareCredentialPair>>(&json) {
                if !pairs.is_empty() {
                    return Ok(pairs);
                }
            }
        }
        Err(keyring::Error::NoEntry) => {}
        Err(e) => {
            eprintln!("Keyring lookup warning: {e}");
        }
    }

    // Check environment variables fallback
    let env_acc = std::env::var("CLOUDFLARE_ACCOUNT_ID")
        .or_else(|_| std::env::var("CF_ACCOUNT_ID"))
        .unwrap_or_default();
    let env_token = std::env::var("CLOUDFLARE_API_TOKEN")
        .or_else(|_| std::env::var("CF_API_TOKEN"))
        .or_else(|_| std::env::var("CLOUDFLARE_API_KEY"))
        .or_else(|_| std::env::var("CF_API_KEY"))
        .unwrap_or_default();

    if !env_acc.trim().is_empty() && !env_token.trim().is_empty() {
        if let Ok(pairs) = parse_and_validate_pairs(&env_acc, &env_token) {
            return Ok(pairs);
        }
    }

    Ok(Vec::new())
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
            FailureCategory::PermissionOrAccess => Duration::from_secs(300), // 5 min cooldown for account/model access failure
            FailureCategory::RateLimit => {
                Duration::from_secs(retry_after_secs.unwrap_or(30).clamp(5, 300))
            }
            FailureCategory::QuotaExhausted => {
                Duration::from_secs(3600) // 1 hour cooldown for exhausted account quota
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

/// Scope guard ensuring cursor restoration, overlay hiding, lock release, and cancellation map cleanup on any exit path
struct ChatLifecycleGuard {
    conversation_id: String,
    active_cancellations: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
}

impl Drop for ChatLifecycleGuard {
    fn drop(&mut self) {
        crate::plugins::desktop_control::overlay::stop_desktop_automation();
        crate::plugins::desktop_control::cursor::force_restore_and_cleanup();
        crate::plugins::desktop_control::input::release_all_input_locks();

        let cancel_map = self.active_cancellations.clone();
        let conv_id = self.conversation_id.clone();
        tokio::spawn(async move {
            let mut map = cancel_map.lock().await;
            map.remove(&conv_id);
        });
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
    model: Option<String>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
) {
    let cancel_token = Arc::new(AtomicBool::new(false));
    {
        let mut map = state.active_cancellations.lock().await;
        map.insert(conversation_id.clone(), cancel_token.clone());
    }

    // RAII guard guarantees desktop cursor, overlays, input locks, and cancellation state are cleaned up
    let _lifecycle_guard = ChatLifecycleGuard {
        conversation_id: conversation_id.clone(),
        active_cancellations: state.active_cancellations.clone(),
    };

    let target_model = match model.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(m) => m.to_string(),
        None => DEFAULT_MODEL_ID.to_string(),
    };

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
        .map(|m| m.text_content())
        .unwrap_or_default();

    let is_time_sensitive = is_time_sensitive_request(&latest_user_prompt);
    let is_action = is_action_request(&latest_user_prompt);
    let automation_session_id = format!("{}-{}", conversation_id, message_id);

    if is_action {
        crate::plugins::desktop_control::overlay::start_desktop_automation_session(
            &automation_session_id,
            "Analyzing screen",
        );
    }

    // Normalize messages: Cloudflare requires role: "system" strictly at index 0.
    // If any subsequent messages have role: "system", consolidate them into the initial system prompt.
    let mut effective_messages = Vec::new();
    let mut system_prompt_content = String::new();

    for m in messages {
        if m.role == "system" {
            let tc = m.text_content();
            if system_prompt_content.is_empty() {
                system_prompt_content = tc;
            } else {
                system_prompt_content.push_str("\n\n");
                system_prompt_content.push_str(&tc);
            }
        } else {
            effective_messages.push(m);
        }
    }

    if system_prompt_content.is_empty() {
        system_prompt_content = "You are Aven, a sophisticated engineering AI assistant.".to_string();
    }

    let is_vision = is_vision_model(&target_model);

    system_prompt_content.push_str(
        "\n\nYou are Aven, an autonomous desktop engineering AI assistant. You operate under a VISION-FIRST, UIA-ON-DEMAND, EVIDENCE-VERIFIED architecture:
- Call `desktop_control_capture_screenshot` to observe the desktop or application window. Screenshots are your primary perception method for understanding layout and visible controls. You can request high-resolution crops for small or ambiguous controls.
- Call `desktop_control_mouse_action` to click, move, drag, or scroll. Accepts either raw desktop coordinates (x, y) or image coordinates (image_x, image_y) with screenshot metadata for automatic desktop coordinate transformation. Clicks always default to left click unless right click context menu is explicitly requested.
- Call `desktop_control_inspect_ui` on-demand for targeted semantic properties: `get_focused_control` (active window, focused control, caret position, text), `inspect_window` (actionable window controls with container context and supported patterns), `read_text` (read text/value/selection), `check_state` (enabled/focus/read-only status), `get_element_bounds` (exact bounding box for clicking), `revalidate_element` (verify if element moved or changed), or `scroll`. Do NOT request the full recursive tree unless necessary.
- Call `desktop_control_keyboard_action` to type text, press keys, or trigger shortcuts. Use mode 'replace' to substitute existing text, 'append' to add to existing text, or 'type' at caret. Specify target `element_query` or `window_id` to focus the target before typing.
- Call `desktop_control_launch_app` to launch or focus applications. Verifies that the window opened before returning success.
- Call `desktop_control_window_action` to focus, minimize, maximize, restore, or close windows.
- Call `desktop_control_wait_for_ui` to wait for windows or elements to appear/change.
- Call `desktop_control_get_screen_info` to get virtual desktop bounds, monitor info, cursor pos, and taskbar layout.
- Call `google_search` for live search grounding, `native_open_url` to open links, `native_system_info` for OS/hardware info, and `native_file_access` for project files.

DYNAMIC ACTION METHOD SELECTION HIERARCHY:
1. Application-Specific API / Native Integration: Use specialized tools (native_file_access, native_system_info) when available.
2. Screenshot & AI Vision-First Element Identification (DEFAULT): Always inspect the visual screenshot to locate buttons, inputs, file pickers, and menus. Supply `image_x` and `image_y` with screenshot metadata to `desktop_control_mouse_action` for automatic virtual desktop coordinate transformation. Reject out-of-bounds or implausible coordinates.
3. Native Keyboard Shortcuts & Unicode Input: For typing text into focused fields or standard dialog shortcuts (Ctrl+S, Enter, Esc), use `keyboard_action`.
4. Targeted UIA as On-Demand Fallback & Property Verification: Use UIA (`inspect_ui`) only for specific semantic properties (`read_text`, `check_state`, `get_focused_control`) or when visual identification is ambiguous. Never default to broad recursive UIA tree searches or override a visually identified target.
5. Visual Evidence Verification: After meaningful actions, capture a fresh screenshot with `desktop_control_capture_screenshot` to verify that the expected state change occurred before marking the task complete.
Fall back dynamically between methods when appropriate, always verifying the latest screen state.

OPERATING PRINCIPLES:
1. Vision-First Perception:
   - Interpret screenshots as the default source for understanding visible UI layout, window states, and controls.
   - Request targeted UIA queries on demand when exact text, accessible names, caret offsets, or focus confirmation are needed.
   - Use both vision and UIA when visual location and semantic identity must be matched.
2. Grounded Actions & Coordinates:
   - Identify targets explicitly before interacting.
   - Use current observations and avoid stale coordinates. If a dialog opens, layout changes, or window moves, capture a fresh observation.
   - Verify transformed coordinates fall within valid window or screen bounds.
3. Explicit Expected Outcome:
   - Declare the expected post-action outcome for meaningful actions (e.g. 'expect Notepad window to appear', 'expect text to be entered').
4. Evidence-Verified Execution:
   - After every meaningful action, observe fresh state and verify the expected change using actual evidence.
   - Distinguish input dispatched from control invoked and user outcome achieved. A missing error does not prove success.
   - Never fabricate observations, tool results, or claims of completion.
5. Bounded Recovery & Honest Reporting:
   - When verification fails: diagnose the failure (e.g. window lost focus, target read-only, control missed) using fresh observation, gather targeted UIA if helpful, and attempt a different justified strategy.
   - Do NOT repeat the exact same failed action blindly. Bounded recovery allows a maximum of 2 retry attempts per failed action.
   - Accurately report outcomes: successful (supported by verified evidence), failed (explained with blockers), unverified (outcome uncertain), partially completed, or cancelled.
6. Shortest Reliable Path & Direct Targeting:
   - Identify the shortest direct sequence of actions to accomplish the goal based on the current screenshot.
   - Prefer direct, visible ribbon tabs, buttons, and dialog controls over multi-step menus or indirect search sequences.
   - In Microsoft Word: to add page borders, click directly on the 'Design' tab on the top ribbon, then click the 'Page Borders' button in the 'Page Background' group on the far right, select 'Box' (or desired border) in the 'Borders and Shading' dialog, click 'OK', and capture a screenshot to visually verify the border.
   - Do not wander into backstage menus (File -> Options) or unrelated tabs when direct visible controls exist.
   - If an action or shortcut is unambiguous and reliable (e.g. Enter to accept a dialog, Ctrl+A to select all), use it directly."
    );

    if is_vision {
        system_prompt_content.push_str(
            "\n\n[Vision Perception Active]: The active model supports multimodal vision. Call desktop_control_capture_screenshot as your primary source for visible UI layout and control verification."
        );
    } else {
        system_prompt_content.push_str(&format!(
            "\n\n[Vision Perception Limitation]: The active model '{}' is text-only. Full desktop screenshots can be captured and saved on disk, but direct pixel processing is unsupported by this model. Rely on targeted UI Automation via desktop_control_inspect_ui (inspect_window, get_focused_control, find_element, read_text) for semantic observation, or switch to a vision-capable model for direct screenshot perception.",
            target_model
        ));
    }

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
        ChatMessage::new_text("system", system_prompt_content),
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
                "type": "function",
                "function": {
                    "name": tool_name,
                    "description": cap.description,
                    "parameters": schema,
                }
            }));
        }
    }

    let mut all_errors: Vec<String> = Vec::new();
    let mut full_text = String::new();
    let mut prompt_tokens: Option<u32> = None;
    let mut completion_tokens: Option<u32> = None;
    let mut total_tokens: Option<u32> = None;
    let mut stream_interrupted = false;
    let mut executed_tools: Vec<String> = Vec::new();
    let mut action_failure_counts: HashMap<String, usize> = HashMap::new();

    let mut current_step: usize = 0;
    let mut automation_metrics = AutomationMetrics::default();
    let mut diagnostics_history: Vec<StructuredDiagnostic> = Vec::new();
    let mut consecutive_unproductive_iterations: usize = 0;

    // Continue while the task has pending work and is making progress.
    // Arbitrary total iteration caps are removed in favor of evidence-based safeguards:
    // - User/hotkey cancellation
    // - Verification failure bounded retries (max 2 per subtask)
    // - Consecutive unproductive iteration detection
    // - Provider limits and token budget compaction
    while !cancel_token.load(Ordering::SeqCst) && !crate::plugins::desktop_control::hotkey::is_cancelled() {
        current_step += 1;
        let mut turn_succeeded = false;
        let mut turn_accumulated_tool = AccumulatedToolCall::default();
        let mut turn_text = String::new();
        let mut turn_finish_reason: Option<String> = None;
        let mut fallback_prompt_converted = false;

        let turn_candidates = {
            let mgr = state.manager.lock().await;
            mgr.get_candidate_indices()
        };

        if turn_candidates.is_empty() {
            let combined_err = "No Cloudflare credentials configured. Open Settings → API to configure Account ID and API Key.".to_string();
            all_errors.push(combined_err);
            break;
        }

        let mut attempted_in_turn = std::collections::HashSet::new();

        for &idx in &turn_candidates {
            if attempted_in_turn.contains(&idx) {
                continue;
            }
            attempted_in_turn.insert(idx);
            if cancel_token.load(Ordering::SeqCst) || crate::plugins::desktop_control::hotkey::is_cancelled() {
                if is_action {
                    crate::plugins::desktop_control::overlay::end_desktop_automation_session(
                        &automation_session_id,
                        crate::plugins::desktop_control::overlay::AutomationSessionState::Cancelled,
                        "Cancelled",
                    );
                } else {
                    crate::plugins::desktop_control::overlay::clear_activity_text();
                    crate::plugins::desktop_control::overlay::hide_overlay();
                }
                crate::plugins::desktop_control::input::release_all_input_locks();
                let _ = app.emit(
                    "chat-activity-status",
                    ChatActivityStatusPayload {
                        conversation_id: conversation_id.clone(),
                        message_id: message_id.clone(),
                        status: "cancelled".to_string(),
                        description: "Cancelled".to_string(),
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
                pair.account_id, target_model
            );

            // 1. Context window management: estimate tokens and compact if necessary
            let model_spec = crate::context_manager::get_model_spec(&target_model);
            let tools_tokens = crate::context_manager::estimate_tokens(
                &serde_json::to_string(&tools_list).unwrap_or_default(),
            );

            let mut message_pairs: Vec<(String, String)> = effective_messages
                .iter()
                .map(|m| (m.role.clone(), m.text_content()))
                .collect();

            let compaction = crate::context_manager::compact_context(&mut message_pairs, &model_spec, tools_tokens);
            if compaction.compacted {
                effective_messages = message_pairs
                    .into_iter()
                    .map(|(role, content)| ChatMessage::new_text(role, content))
                    .collect();
            }

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

            // Supply tool definitions whenever tools are available
            if !tools_list.is_empty() {
                req_body_map.insert(
                    "tools".to_string(),
                    serde_json::Value::Array(tools_list.clone()),
                );
            }

            let req_body = serde_json::Value::Object(req_body_map);

            if is_action {
                crate::plugins::desktop_control::cursor::touch_task_heartbeat(
                    &automation_session_id,
                    crate::plugins::desktop_control::overlay::AutomationSessionState::WaitingForModel,
                );
            }

            let send_timeout = tokio::time::timeout(
                Duration::from_secs(45),
                state
                    .http_client
                    .post(&url)
                    .header("Authorization", format!("Bearer {}", pair.api_key))
                    .header("Content-Type", "application/json")
                    .json(&req_body)
                    .send(),
            )
            .await;

            let response = match send_timeout {
                Ok(Ok(r)) => r,
                Ok(Err(e)) => {
                    let err_desc = format!("Pair #{} ({}): network error: {e}", idx + 1, masked);
                    all_errors.push(err_desc);
                    let mut mgr = state.manager.lock().await;
                    mgr.mark_failure(idx, FailureCategory::ServerOrNetwork, None);
                    continue;
                }
                Err(_) => {
                    let err_desc = format!("Pair #{} ({}): request timed out after 45s", idx + 1, masked);
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

                // Context length error recovery: If provider reports context length overflow, compact aggressively and retry
                if crate::context_manager::is_context_length_error(&detail) {
                    let aggressive_spec = crate::context_manager::ModelContextSpec::new(
                        model_spec.max_context_tokens / 2,
                        model_spec.max_completion_tokens,
                        model_spec.safety_margin_tokens,
                    );
                    let mut pairs: Vec<(String, String)> = effective_messages
                        .iter()
                        .map(|m| (m.role.clone(), m.text_content()))
                        .collect();
                    let comp_res = crate::context_manager::compact_context(&mut pairs, &aggressive_spec, tools_tokens);
                    if comp_res.compacted {
                        effective_messages = pairs
                            .into_iter()
                            .map(|(role, content)| ChatMessage::new_text(role, content))
                            .collect();
                        continue;
                    }
                }

                // Resilience fallback: If Cloudflare rejects role: "tool" with HTTP 400 on synthesis,
                // replace the assistant tool call and tool message with a user prompt and retry.
                if current_step > 1 && code == 400 && !fallback_prompt_converted && effective_messages.len() >= 2 {
                    let last_tool = effective_messages.pop();
                    let _last_asst = effective_messages.pop();
                    let tool_name_info = last_tool.as_ref().and_then(|m| m.name.clone()).unwrap_or_else(|| "tool".to_string());
                    let tool_result_info = last_tool.map(|m| m.text_content()).unwrap_or_default();

                    effective_messages.push(ChatMessage::new_text(
                        "user",
                        format!(
                            "[Verified Tool Result for {}]:\n{}\n\nBased on these verified results, please provide a comprehensive and direct answer to the user.",
                            tool_name_info, tool_result_info
                        ),
                    ));
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
                            let mut byte_stream = retry_resp.bytes_stream();
                            let mut buffer = String::new();
                            turn_accumulated_tool = AccumulatedToolCall::default();
                            turn_text.clear();

                            while let Some(chunk_res) = byte_stream.next().await {
                                if cancel_token.load(Ordering::SeqCst) {
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
                                            if let Some(token) = extract_visible_token(&parsed) {
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
                    402 => (
                        FailureCategory::QuotaExhausted,
                        format!("Pair #{} ({}): HTTP 402 Payment Required (Account quota exhausted)", idx + 1, masked),
                    ),
                    404 => (
                        FailureCategory::PermissionOrAccess,
                        format!("Pair #{} ({}): HTTP 404 Not Found (Invalid account or model '{}' unavailable)", idx + 1, masked, target_model),
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

                {
                    let mut mgr = state.manager.lock().await;
                    mgr.mark_failure(idx, cat, retry_after);
                }

                all_errors.push(err_msg.clone());

                if cat == FailureCategory::PermanentRequestErr {
                    crate::plugins::desktop_control::overlay::clear_activity_text();
                    crate::plugins::desktop_control::overlay::hide_overlay();
                    let _ = app.emit(
                        "chat-activity-status",
                        ChatActivityStatusPayload {
                            conversation_id: conversation_id.clone(),
                            message_id: message_id.clone(),
                            status: "failed".to_string(),
                            description: "Failed".to_string(),
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
                    return;
                }

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
            let mut visible_tokens_emitted: usize = 0;
            let mut stream_failed_before_output = false;

            loop {
                if cancel_token.load(Ordering::SeqCst) {
                    crate::plugins::desktop_control::overlay::clear_activity_text();
                    crate::plugins::desktop_control::overlay::hide_overlay();
                    let _ = app.emit(
                        "chat-activity-status",
                        ChatActivityStatusPayload {
                            conversation_id: conversation_id.clone(),
                            message_id: message_id.clone(),
                            status: "cancelled".to_string(),
                            description: "Cancelled".to_string(),
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
                    return;
                }

                let chunk_timeout = tokio::time::timeout(Duration::from_secs(30), byte_stream.next()).await;
                let chunk_res = match chunk_timeout {
                    Ok(Some(res)) => res,
                    Ok(None) => break, // Normal EOF of stream
                    Err(_) => {
                        if visible_tokens_emitted == 0 {
                            let err_desc = format!("Pair #{} ({}): stream timed out waiting for chunk", idx + 1, masked);
                            all_errors.push(err_desc);
                            let mut mgr = state.manager.lock().await;
                            mgr.mark_failure(idx, FailureCategory::ServerOrNetwork, None);
                            stream_failed_before_output = true;
                            break;
                        } else {
                            stream_interrupted = true;
                            break;
                        }
                    }
                };

                let bytes = match chunk_res {
                    Ok(b) => b,
                    Err(e) => {
                        if visible_tokens_emitted == 0 {
                            let err_desc = format!("Pair #{} ({}): network error during initial stream: {e}", idx + 1, masked);
                            all_errors.push(err_desc);
                            let mut mgr = state.manager.lock().await;
                            mgr.mark_failure(idx, FailureCategory::ServerOrNetwork, None);
                            stream_failed_before_output = true;
                            break;
                        } else {
                            stream_interrupted = true;
                            break;
                        }
                    }
                };

                let text_chunk = String::from_utf8_lossy(&bytes);
                buffer.push_str(&text_chunk);
                crate::plugins::desktop_control::cursor::touch_activity();

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

                            // Inspect finish reason
                            if let Some(choices) = parsed.get("choices").and_then(|v| v.as_array()) {
                                for choice in choices {
                                    if let Some(fr) = choice.get("finish_reason").and_then(|v| v.as_str()) {
                                        turn_finish_reason = Some(fr.to_string());
                                    }
                                }
                            }

                            // If reasoning content arrives, report thinking activity status without user-facing text leakage
                            if let Some(reasoning) = extract_reasoning_from_json(&parsed) {
                                if !reasoning.is_empty() {
                                    let reasoning_label = if executed_tools.last().map(|t| t.contains("capture_screenshot")).unwrap_or(false) {
                                        "Analyzing screen"
                                    } else if executed_tools.last().map(|t| t.contains("inspect_ui")).unwrap_or(false) {
                                        "Verifying result"
                                    } else {
                                        "Planning next action"
                                    };
                                    crate::plugins::desktop_control::overlay::set_activity_text(reasoning_label);
                                    let _ = app.emit(
                                        "chat-activity-status",
                                        ChatActivityStatusPayload {
                                            conversation_id: conversation_id.clone(),
                                            message_id: message_id.clone(),
                                            status: if reasoning_label == "Analyzing screen" { "analyzing".to_string() } else { "running".to_string() },
                                            description: reasoning_label.to_string(),
                                            integration_id: None,
                                            tool_name: None,
                                        },
                                    );
                                }
                            }

                            // Extract user-facing visible token
                            if let Some(token) = extract_visible_token(&parsed) {
                                if !token.is_empty() {
                                    turn_text.push_str(&token);
                                    let is_tool_token = turn_accumulated_tool.is_valid()
                                        || turn_text.contains("<tool_call>")
                                        || token.contains("<tool_call>")
                                        || (turn_text.contains("```json") && turn_text.contains("\"name\""));

                                    if !is_tool_token {
                                        visible_tokens_emitted += 1;
                                        full_text.push_str(&token);

                                        // If the assistant announces its next action in conversational speech, update status immediately
                                        if indicates_pending_action(&turn_text) {
                                            let lower_intent = turn_text.to_lowercase();
                                            let action_desc = if lower_intent.contains("border") {
                                                "Applying Page Borders"
                                            } else if lower_intent.contains("design") {
                                                "Opening Design tab"
                                            } else if lower_intent.contains("word") || lower_intent.contains("winword") {
                                                "Opening Microsoft Word"
                                            } else if lower_intent.contains("introduce") || lower_intent.contains("introduction") {
                                                "Entering introduction"
                                            } else if lower_intent.contains("save") {
                                                "Saving document"
                                            } else {
                                                "Executing next step"
                                            };
                                            crate::plugins::desktop_control::overlay::set_activity_text(action_desc);
                                            if is_action {
                                                crate::plugins::desktop_control::overlay::update_automation_session(
                                                    &automation_session_id,
                                                    crate::plugins::desktop_control::overlay::AutomationSessionState::Running,
                                                    action_desc,
                                                    None,
                                                    None,
                                                );
                                            }
                                        }

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
                            if let Some(token) = extract_visible_token(&parsed) {
                                if !token.is_empty() {
                                    turn_text.push_str(&token);
                                    let is_tool_token = turn_accumulated_tool.is_valid()
                                        || turn_text.contains("<tool_call>")
                                        || token.contains("<tool_call>")
                                        || (turn_text.contains("```json") && turn_text.contains("\"name\""));

                                    if !is_tool_token {
                                        visible_tokens_emitted += 1;
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

            if stream_failed_before_output {
                continue;
            }

            if stream_interrupted && visible_tokens_emitted > 0 {
                turn_succeeded = false;
                break;
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
            let tool_args = if let Some(cap) = resolved {
                normalize_tool_arguments(&tool_name, &cap.id, tool_args)
            } else {
                tool_args
            };
            let display_name = resolved.map(|c| c.name.as_str()).unwrap_or(&tool_name);
            let activity_desc = format_tool_activity_description(&tool_name, &tool_args, display_name);

            if is_action {
                crate::plugins::desktop_control::cursor::touch_task_heartbeat(
                    &automation_session_id,
                    crate::plugins::desktop_control::overlay::AutomationSessionState::ExecutingTool,
                );
                crate::plugins::desktop_control::overlay::update_automation_session(
                    &automation_session_id,
                    crate::plugins::desktop_control::overlay::AutomationSessionState::ExecutingTool,
                    &activity_desc,
                    None,
                    None,
                );
            } else {
                crate::plugins::desktop_control::overlay::set_activity_text(&activity_desc);
            }

            let _ = app.emit(
                "chat-activity-status",
                ChatActivityStatusPayload {
                    conversation_id: conversation_id.clone(),
                    message_id: message_id.clone(),
                    status: "tool_running".to_string(),
                    description: activity_desc,
                    integration_id: resolved.map(|c| c.source_id.clone()),
                    tool_name: Some(tool_name.clone()),
                },
            );

            if cancel_token.load(Ordering::SeqCst) || crate::plugins::desktop_control::hotkey::is_cancelled() {
                crate::plugins::desktop_control::overlay::clear_activity_text();
                crate::plugins::desktop_control::overlay::hide_overlay();
                crate::plugins::desktop_control::input::release_all_input_locks();
                let _ = app.emit(
                    "chat-activity-status",
                    ChatActivityStatusPayload {
                        conversation_id: conversation_id.clone(),
                        message_id: message_id.clone(),
                        status: "cancelled".to_string(),
                        description: "Cancelled".to_string(),
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
                return;
            }

            let is_desktop_tool = resolved
                .map(|c| c.source_id == "native_desktop_control")
                .unwrap_or(false)
                || tool_name.starts_with("desktop_control_");

            if is_desktop_tool {
                let factual_label = map_tool_to_factual_label(&tool_name, &tool_args);
                crate::plugins::desktop_control::overlay::update_automation_session(
                    &automation_session_id,
                    crate::plugins::desktop_control::overlay::AutomationSessionState::ExecutingTool,
                    &factual_label,
                    Some(tool_name.clone()),
                    tool_args.get("target_description").and_then(|v| v.as_str()).map(|s| s.to_string()),
                );
                if let Some(x) = tool_args.get("x").and_then(|v| v.as_i64()) {
                    if let Some(y) = tool_args.get("y").and_then(|v| v.as_i64()) {
                        crate::plugins::desktop_control::overlay::update_target_position(x as i32, y as i32);
                    }
                }
            }

            let exec_res = if let Some(cap) = resolved {
                let req = crate::plugins::registry::CapabilityExecutionRequest {
                    capability_id: cap.id.clone(),
                    arguments: tool_args.clone(),
                    user_confirmed: true,
                };
                let r = plugins.execute_capability_internal(req).await;
                crate::plugins::desktop_control::input::release_all_input_locks();
                r
            } else {
                Err(format!("Capability '{}' not found in registry", tool_name))
            };

            executed_tools.push(tool_name.clone());
            let interaction_method = determine_interaction_method(&tool_name, &tool_args);
            automation_metrics.interaction_methods_used.push(interaction_method.clone());

            if tool_name.contains("capture_screenshot") {
                automation_metrics.screenshot_count += 1;
            } else if tool_name.contains("inspect_ui") {
                automation_metrics.uia_query_count += 1;
            }

            let mut screenshot_image_base64: Option<String> = None;
            let mut screenshot_format: String = "jpeg".to_string();
            let mut screenshot_info_obj = serde_json::json!({});

            let tool_content_str = match &exec_res {
                Ok(r) if r.success => {
                    let mut val = r.result.clone().unwrap_or(serde_json::json!({ "status": "ok" }));
                    if tool_name.contains("capture_screenshot") {
                        if let Some(obj) = val.as_object_mut() {
                            if let Some(b64) = obj.remove("image_base64").and_then(|v| v.as_str().map(|s| s.to_string())) {
                                screenshot_image_base64 = Some(b64);
                            }
                            if let Some(fmt) = obj.get("format").and_then(|v| v.as_str()) {
                                screenshot_format = fmt.to_string();
                            }
                            screenshot_info_obj = serde_json::Value::Object(obj.clone());
                            if !is_vision {
                                obj.insert(
                                    "vision_notice".to_string(),
                                    serde_json::json!(format!(
                                        "[Vision Limitation]: Active model '{}' is text-only. Screenshot was saved to disk. To inspect UI semantically, use desktop_control_inspect_ui (inspect_window, get_focused_control, find_element).",
                                        target_model
                                    )),
                                );
                            }
                        }
                    }
                    serde_json::to_string(&val).unwrap_or_default()
                }
                Ok(r) => {
                    serde_json::json!({ "error": r.error.clone().unwrap_or_else(|| "Execution failed".to_string()) }).to_string()
                }
                Err(e) => {
                    serde_json::json!({ "error": e }).to_string()
                }
            };

            // Bound tool output results before appending to context to prevent context-window overflow
            let is_verification_failed = tool_content_str.contains("verification_failed")
                || tool_content_str.contains("\"verified\":false")
                || tool_content_str.contains("\"verified\": false");

            if is_verification_failed {
                automation_metrics.verification_failure_count += 1;
                automation_metrics.retry_count += 1;
                if is_action {
                    crate::plugins::desktop_control::cursor::touch_task_heartbeat(
                        &automation_session_id,
                        crate::plugins::desktop_control::overlay::AutomationSessionState::Recovering,
                    );
                    crate::plugins::desktop_control::overlay::update_automation_session(
                        &automation_session_id,
                        crate::plugins::desktop_control::overlay::AutomationSessionState::Recovering,
                        "Recovering",
                        Some("Action verification failed".to_string()),
                        None,
                    );
                } else {
                    crate::plugins::desktop_control::overlay::set_activity_text("Recovering");
                }
                let _ = app.emit(
                    "chat-activity-status",
                    ChatActivityStatusPayload {
                        conversation_id: conversation_id.clone(),
                        message_id: message_id.clone(),
                        status: "recovering".to_string(),
                        description: "Recovering".to_string(),
                        integration_id: None,
                        tool_name: None,
                    },
                );

                diagnostics_history.push(StructuredDiagnostic {
                    step: current_step,
                    selected_method: interaction_method.clone(),
                    target_id: tool_args.get("element_query").or_else(|| tool_args.get("window_id")).map(|v| v.to_string()),
                    action: tool_name.clone(),
                    result: "failed".to_string(),
                    verification_evidence: Some(tool_content_str.chars().take(200).collect()),
                    recovery_reason: Some(format!("Action verification failed (attempt {} of 2)", action_failure_counts.get(&tool_name).copied().unwrap_or(0) + 1)),
                });
            } else {
                automation_metrics.verification_success_count += 1;
                if is_action {
                    crate::plugins::desktop_control::cursor::touch_task_heartbeat(
                        &automation_session_id,
                        crate::plugins::desktop_control::overlay::AutomationSessionState::Verifying,
                    );
                }
                diagnostics_history.push(StructuredDiagnostic {
                    step: current_step,
                    selected_method: interaction_method.clone(),
                    target_id: tool_args.get("element_query").or_else(|| tool_args.get("window_id")).map(|v| v.to_string()),
                    action: tool_name.clone(),
                    result: "success".to_string(),
                    verification_evidence: Some("Action executed and confirmed".to_string()),
                    recovery_reason: None,
                });
            }

            let bounded_tool_content = if is_verification_failed {
                let count = action_failure_counts.entry(tool_name.clone()).or_insert(0);
                *count += 1;
                let current_retry = *count;
                let bounded = crate::context_manager::bound_tool_result(&tool_name, &tool_content_str);
                if current_retry <= 2 {
                    format!(
                        "{bounded}\n\n[Action Verification Diagnostic]: Action failed verification (recovery attempt {current_retry} of 2). Observe fresh UI state before retrying. Do not blindly repeat the exact same input. Try a different justified strategy (e.g. refocusing window, inspecting element bounds, or clicking center point)."
                    )
                } else {
                    format!(
                        "{bounded}\n\n[Action Verification Failure]: Retry limit reached (2 attempts). Do not retry this action. Explain the blocker with available evidence and report outcome as failed or partially completed."
                    )
                }
            } else {
                if tool_content_str.contains("\"verified\":true") || tool_content_str.contains("\"verified\": true") {
                    action_failure_counts.remove(&tool_name);
                }
                crate::context_manager::bound_tool_result(&tool_name, &tool_content_str)
            };

            let sanitized_call_args = redact_sensitive_tool_args(&tool_name, &tool_args);

            // Append assistant tool call and tool response to conversation history
            effective_messages.push(ChatMessage {
                role: "assistant".to_string(),
                content: serde_json::Value::String(String::new()),
                name: None,
                tool_call_id: None,
                tool_calls: Some(vec![serde_json::json!({
                    "id": tool_call_id,
                    "type": "function",
                    "name": tool_name,
                    "function": {
                        "name": tool_name,
                        "arguments": serde_json::to_string(&sanitized_call_args).unwrap_or_default(),
                    }
                })]),
            });

            effective_messages.push(ChatMessage {
                role: "tool".to_string(),
                content: serde_json::Value::String(bounded_tool_content),
                name: Some(tool_name.clone()),
                tool_call_id: Some(tool_call_id),
                tool_calls: None,
            });

            // If a screenshot was captured and the model supports vision, inject the multimodal image payload
            if is_vision && screenshot_image_base64.is_some() {
                let b64 = screenshot_image_base64.unwrap();
                let title = screenshot_info_obj.get("window_title").and_then(|v| v.as_str()).unwrap_or("Desktop");
                let is_dialog = screenshot_info_obj.get("is_dialog").and_then(|v| v.as_bool()).unwrap_or(false);
                let fg_proc = screenshot_info_obj.get("foreground_process").and_then(|v| v.as_str()).unwrap_or("");
                let w = screenshot_info_obj.get("width").and_then(|v| v.as_u64()).unwrap_or(0);
                let h = screenshot_info_obj.get("height").and_then(|v| v.as_u64()).unwrap_or(0);
                let scale_x = screenshot_info_obj.get("scale_x").and_then(|v| v.as_f64()).unwrap_or(1.0);
                let scale_y = screenshot_info_obj.get("scale_y").and_then(|v| v.as_f64()).unwrap_or(1.0);
                let orig_x = screenshot_info_obj.get("desktop_origin_x").and_then(|v| v.as_i64()).unwrap_or(0);
                let orig_y = screenshot_info_obj.get("desktop_origin_y").and_then(|v| v.as_i64()).unwrap_or(0);
                let dpi = screenshot_info_obj.get("dpi_scale").and_then(|v| v.as_f64()).unwrap_or(1.0);

                let dialog_hint = if is_dialog {
                    format!("\n[Active Native Dialog Detected - {}]: A native dialog or window ({}) is currently active. Visually inspect its controls (buttons, filename input field, file lists) and interact with it directly.", title, fg_proc)
                } else {
                    String::new()
                };

                let visual_prompt = format!(
                    "[Visual Desktop Screenshot - {}]: {}x{} (scale: {:.2}x{:.2}, origin: [{}, {}], DPI: {:.2}). Visually inspect the layout and controls. When clicking, provide image coordinates (image_x, image_y) or desktop coordinates (x, y).{}",
                    title, w, h, scale_x, scale_y, orig_x, orig_y, dpi, dialog_hint
                );

                effective_messages.push(ChatMessage::new_multimodal(
                    "user",
                    &visual_prompt,
                    &b64,
                    &screenshot_format,
                ));
            }

            let next_label = if tool_name.contains("capture_screenshot") {
                "Analyzing screen"
            } else if tool_name.contains("inspect_ui") {
                "Verifying result"
            } else {
                "Planning next action"
            };

            if is_action {
                crate::plugins::desktop_control::overlay::update_automation_session(
                    &automation_session_id,
                    crate::plugins::desktop_control::overlay::AutomationSessionState::WaitingForModel,
                    next_label,
                    None,
                    None,
                );
            } else {
                crate::plugins::desktop_control::overlay::set_activity_text(next_label);
            }

            let _ = app.emit(
                "chat-activity-status",
                ChatActivityStatusPayload {
                    conversation_id: conversation_id.clone(),
                    message_id: message_id.clone(),
                    status: if next_label == "Analyzing screen" { "analyzing".to_string() } else { "running".to_string() },
                    description: next_label.to_string(),
                    integration_id: resolved.map(|c| c.source_id.clone()),
                    tool_name: Some(tool_name.clone()),
                },
            );

            consecutive_unproductive_iterations = 0;
            // Continue loop to synthesize follow-up model response or execute next tool step
            continue;
        }

        // 1. If the model gave introductory speech on step 1 without calling tools:
        if current_step == 1 && is_action_request(&latest_user_prompt) {
            let clean_speech = strip_reasoning_from_text(&strip_tool_calls_from_text(&turn_text));
            let speech_text = if clean_speech.is_empty() {
                "I will execute this desktop task for you.".to_string()
            } else {
                clean_speech
            };

            effective_messages.push(ChatMessage::new_text("assistant", speech_text));
            effective_messages.push(ChatMessage::new_text(
                "user",
                "Please proceed with executing the task now by calling the appropriate desktop tool directly (such as desktop_control_capture_screenshot or desktop_control_launch_app). Call the tool to perform the desktop actions.",
            ));

            let _ = app.emit(
                "chat-activity-status",
                ChatActivityStatusPayload {
                    conversation_id: conversation_id.clone(),
                    message_id: message_id.clone(),
                    status: "tool_running".to_string(),
                    description: "Executing requested task...".to_string(),
                    integration_id: Some("native_desktop_control".to_string()),
                    tool_name: Some("desktop_control_launch_app".to_string()),
                },
            );

            continue;
        }

        // 2. Multi-step execution continuation & Action-by-action completion gate:
        if is_action {
            let gate_outcome = evaluate_task_completion_gate(
                &latest_user_prompt,
                &executed_tools,
                current_step,
                0,
                &turn_text,
            );

            match gate_outcome {
                GateOutcome::RequireContinuation { prompt, reason } => {
                    let is_verifying = reason == "unverified_desktop_state";
                    let cont_label = if is_verifying {
                        "Verifying result"
                    } else if reason == "unexecuted_borders" {
                        "Applying Page Borders"
                    } else if reason == "unexecuted_typing" {
                        "Entering introduction"
                    } else if reason == "unexecuted_launch" {
                        "Opening application"
                    } else {
                        "Executing task"
                    };
                    crate::plugins::desktop_control::overlay::update_automation_session(
                        &automation_session_id,
                        if is_verifying {
                            crate::plugins::desktop_control::overlay::AutomationSessionState::Verifying
                        } else {
                            crate::plugins::desktop_control::overlay::AutomationSessionState::Running
                        },
                        cont_label,
                        Some(reason.to_string()),
                        None,
                    );

                    let clean_assistant_turn = strip_reasoning_from_text(&strip_tool_calls_from_text(&turn_text));
                    let asst_text = if clean_assistant_turn.trim().is_empty() {
                        "Executing task step...".to_string()
                    } else {
                        clean_assistant_turn
                    };
                    effective_messages.push(ChatMessage::new_text("assistant", asst_text));
                    effective_messages.push(ChatMessage::new_text("user", prompt));

                    if !turn_accumulated_tool.is_valid() {
                        consecutive_unproductive_iterations += 1;
                        if consecutive_unproductive_iterations >= 6 {
                            stream_interrupted = true;
                            all_errors.push("Task paused: multiple consecutive iterations yielded no desktop action or tool invocation.".to_string());
                            break;
                        }
                    } else {
                        consecutive_unproductive_iterations = 0;
                    }

                    continue;
                }
                GateOutcome::AllowCompletion => {
                    // All sub-goals executed and outcome verified!
                }
            }
        }

        // 3. Truncated completion recovery:
        if turn_finish_reason.as_deref() == Some("length") {
            effective_messages.push(ChatMessage::new_text("assistant", turn_text.clone()));
            effective_messages.push(ChatMessage::new_text(
                "user",
                "Please continue your response from where you left off.",
            ));

            continue;
        }

        // 4. If this was a synthesis turn after tool execution, but the model returned no text:
        if current_step > 1 && full_text.trim().is_empty() {
            if let Some(tool_msg) = effective_messages.iter().rev().find(|m| m.role == "tool") {
                if let Ok(parsed_res) = serde_json::from_str::<serde_json::Value>(&tool_msg.text_content()) {
                    let is_verified = parsed_res.get("verified").and_then(|v| v.as_bool()).unwrap_or(true);
                    let is_failed = parsed_res.get("status").and_then(|v| v.as_str()) == Some("verification_failed");
                    let diag = parsed_res.get("diagnosis").and_then(|v| v.as_str());

                    let fallback_answer = if is_failed || !is_verified {
                        let action = parsed_res.get("action").and_then(|v| v.as_str()).unwrap_or("desktop action");
                        let reason = diag.unwrap_or("UI state did not change as expected");
                        format!("Action '{action}' failed verification: {reason}. Diagnostic findings recorded.")
                    } else if let Some(msg) = parsed_res.get("message").and_then(|v| v.as_str()) {
                        msg.to_string()
                    } else if let Some(action) = parsed_res.get("action").and_then(|v| v.as_str()) {
                        if action == "type_text" {
                            let count = parsed_res.get("chars_typed").and_then(|v| v.as_u64()).unwrap_or(0);
                            format!("Successfully typed {count} characters and verified state change.")
                        } else {
                            format!("Successfully executed action '{action}' and verified outcome.")
                        }
                    } else if let Some(dev_name) = parsed_res.get("device_name").and_then(|v| v.as_str()) {
                        let os = parsed_res.get("os").and_then(|v| v.as_str()).unwrap_or("Windows");
                        let user = parsed_res.get("user_name").and_then(|v| v.as_str()).unwrap_or("");
                        if !user.is_empty() {
                            format!("Your device name is **{}** (logged in as `{}` on {}).", dev_name, user, os)
                        } else {
                            format!("Your device name is **{}** (running {}).", dev_name, os)
                        }
                    } else {
                        "Task action completed and verified with evidence.".to_string()
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

        // Final response completed
        break;
    }

    if is_action {
        let final_state = if stream_interrupted || (full_text.is_empty() && !all_errors.is_empty()) {
            crate::plugins::desktop_control::overlay::AutomationSessionState::Failed
        } else {
            crate::plugins::desktop_control::overlay::AutomationSessionState::Completed
        };
        let final_label = if final_state == crate::plugins::desktop_control::overlay::AutomationSessionState::Completed {
            "Completed"
        } else {
            "Failed"
        };
        crate::plugins::desktop_control::overlay::end_desktop_automation_session(
            &automation_session_id,
            final_state,
            final_label,
        );
    } else {
        crate::plugins::desktop_control::overlay::clear_activity_text();
        crate::plugins::desktop_control::overlay::hide_overlay();
    }
    let final_status_label = if stream_interrupted || (full_text.is_empty() && !all_errors.is_empty()) {
        "failed"
    } else {
        "completed"
    };
    let _ = app.emit(
        "chat-activity-status",
        ChatActivityStatusPayload {
            conversation_id: conversation_id.clone(),
            message_id: message_id.clone(),
            status: final_status_label.to_string(),
            description: if final_status_label == "completed" { "Completed".to_string() } else { "Failed".to_string() },
            integration_id: None,
            tool_name: None,
        },
    );

    let clean_full_text = strip_reasoning_from_text(&strip_tool_calls_from_text(&full_text));

    if stream_interrupted {
        let _ = app.emit(
            "chat-stream-done",
            StreamDonePayload {
                conversation_id: conversation_id.clone(),
                message_id: message_id.clone(),
                full_text: clean_full_text,
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
            "Model '{}' request failed across all configured credentials:\n• {}",
            target_model,
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
                full_text: clean_full_text,
                cancelled: false,
                error: None,
                prompt_tokens,
                completion_tokens,
                total_tokens,
            },
        );
    }
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
    model: Option<String>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
) -> Result<(), String> {
    crate::plugins::desktop_control::hotkey::reset_cancel_flag();
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
            model,
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
    crate::plugins::desktop_control::overlay::stop_desktop_automation();
    crate::plugins::desktop_control::cursor::force_restore_and_cleanup();
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
    fn test_parse_and_validate_pairs_with_newlines() {
        let accs = "acc_1\nacc_2\r\nacc_3";
        let keys = "key_1\nkey_2\r\nkey_3";
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
    fn test_single_account_multiple_api_keys_pairing() {
        let accs = "shared_account_id_123456789";
        let keys = "key_alpha, key_beta, key_gamma";
        let res = parse_and_validate_pairs(accs, keys).unwrap();
        assert_eq!(res.len(), 3);
        assert_eq!(res[0].account_id, "shared_account_id_123456789");
        assert_eq!(res[0].api_key, "key_alpha");
        assert_eq!(res[1].account_id, "shared_account_id_123456789");
        assert_eq!(res[1].api_key, "key_beta");
        assert_eq!(res[2].account_id, "shared_account_id_123456789");
        assert_eq!(res[2].api_key, "key_gamma");
    }

    #[test]
    fn test_parse_and_validate_pairs_deduplication() {
        let accs = "acc_1, acc_1, acc_2";
        let keys = "key_1, key_1, key_2";
        let res = parse_and_validate_pairs(accs, keys).unwrap();
        // Duplicate pair (acc_1, key_1) must be deduplicated
        assert_eq!(res.len(), 2);
        assert_eq!(res[0].account_id, "acc_1");
        assert_eq!(res[0].api_key, "key_1");
        assert_eq!(res[1].account_id, "acc_2");
        assert_eq!(res[1].api_key, "key_2");
    }

    #[test]
    fn test_failover_cooldown_and_candidate_exhaustion() {
        let pairs = vec![
            CloudflareCredentialPair { account_id: "acc_1".into(), api_key: "key_1".into() },
            CloudflareCredentialPair { account_id: "acc_2".into(), api_key: "key_2".into() },
            CloudflareCredentialPair { account_id: "acc_3".into(), api_key: "key_3".into() },
        ];
        let mut mgr = CredentialManager::new(pairs);

        // Turn 1: Initial candidate order
        let c1 = mgr.get_candidate_indices();
        assert_eq!(c1, vec![0, 1, 2]);

        // Key 0 encounters PermissionOrAccess (e.g. 404 model access error)
        mgr.mark_failure(0, FailureCategory::PermissionOrAccess, None);
        let c2 = mgr.get_candidate_indices();
        assert_eq!(c2, vec![1, 2, 0]); // Key 1 & 2 are prioritized, 0 is cooled down

        // Key 1 encounters QuotaExhausted (e.g. 402 quota error)
        mgr.mark_failure(1, FailureCategory::QuotaExhausted, None);
        let c3 = mgr.get_candidate_indices();
        assert_eq!(c3, vec![2, 0, 1]); // Key 2 is remaining fresh candidate

        // Key 2 succeeds!
        mgr.mark_success(2);
        assert_eq!(mgr.preferred_index, 2);

        // Subsequent turn prioritizes preferred successful Key 2
        let c4 = mgr.get_candidate_indices();
        assert_eq!(c4[0], 2);
    }

    #[test]
    fn test_keys_never_exposed_in_masked_output() {
        let raw_secret_key = "cf_super_secret_api_key_123456789";
        let raw_account_id = "cf_secret_account_8899aabbccdd";
        let pairs = vec![
            CloudflareCredentialPair {
                account_id: raw_account_id.into(),
                api_key: raw_secret_key.into(),
            },
        ];
        let mgr = CredentialManager::new(pairs);
        let meta = mgr.get_metadata();
        assert_eq!(meta.len(), 1);

        // Account ID is masked, and secret API key is never in metadata
        assert!(!meta[0].account_id_masked.contains("secret_account_8899"));
        assert!(meta[0].account_id_masked.contains("..."));

        // Serialized metadata sent across IPC must never leak secret API key or full account
        let serialized = serde_json::to_string(&meta).unwrap();
        assert!(!serialized.contains(raw_secret_key));
        assert!(!serialized.contains("secret_account_8899"));
    }

    #[test]
    fn test_format_tool_activity_description_concise_states() {
        let args_find = serde_json::json!({ "action": "find_element", "query": "Save Button" });
        assert_eq!(format_tool_activity_description("desktop_control_inspect_ui", &args_find, "Inspect UI"), "Searching UIA");

        let args_bounds = serde_json::json!({ "action": "get_element_bounds", "query": "Save Button" });
        assert_eq!(format_tool_activity_description("desktop_control_inspect_ui", &args_bounds, "Inspect UI"), "Finding target");

        let args_click = serde_json::json!({ "action": "click" });
        assert_eq!(format_tool_activity_description("desktop_control_mouse_action", &args_click, "Mouse Click"), "Clicking");

        let args_type = serde_json::json!({ "action": "type_text", "text": "hello" });
        assert_eq!(format_tool_activity_description("desktop_control_keyboard_action", &args_type, "Type Text"), "Typing");

        let args_screen = serde_json::json!({});
        assert_eq!(format_tool_activity_description("desktop_control_capture_screenshot", &args_screen, "Capture Screenshot"), "Capturing screen");

        let args_wait = serde_json::json!({ "condition": "window_opened" });
        assert_eq!(format_tool_activity_description("desktop_control_wait_for_ui", &args_wait, "Wait For UI"), "Waiting for application");
    }

    #[test]
    fn test_multimodal_vision_payload_construction() {
        let msg = ChatMessage::new_multimodal(
            "user",
            "Observed desktop layout",
            "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==",
            "png",
        );
        assert_eq!(msg.role, "user");
        assert!(msg.content.is_array());
        let arr = msg.content.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["type"], "text");
        assert_eq!(arr[0]["text"], "Observed desktop layout");
        assert_eq!(arr[1]["type"], "image_url");
        assert!(arr[1]["image_url"]["url"].as_str().unwrap().starts_with("data:image/png;base64,"));
        assert_eq!(msg.text_content(), "Observed desktop layout");

        let text_msg = ChatMessage::new_text("assistant", "Ready to execute");
        assert_eq!(text_msg.text_content(), "Ready to execute");
    }

    #[test]
    fn test_redact_sensitive_tool_args() {
        let sensitive = serde_json::json!({
            "password": "MySuperSecretPassword123!",
            "token": "ghp_xxxxxxxxxxxx",
            "image_base64": "very_large_base64_string_here_1234567890",
            "normal_field": "hello world"
        });

        let redacted = redact_sensitive_tool_args("desktop_control_mouse_action", &sensitive);
        assert_eq!(redacted["password"], "***REDACTED***");
        assert_eq!(redacted["token"], "***REDACTED***");
        assert_eq!(redacted["image_base64"], "[image_base64_omitted]");
        assert_eq!(redacted["normal_field"], "hello world");

        let keyboard_pwd = serde_json::json!({
            "action": "type_text",
            "element_query": "Password input",
            "text": "SecretP@ssw0rd!"
        });
        let redacted_kb = redact_sensitive_tool_args("desktop_control_keyboard_action", &keyboard_pwd);
        assert_eq!(redacted_kb["text"], "***REDACTED***");
    }

    #[test]
    fn test_determine_interaction_method() {
        assert_eq!(
            determine_interaction_method("desktop_control_inspect_ui", &serde_json::json!({ "action": "invoke" })),
            "uia_semantic"
        );
        assert_eq!(
            determine_interaction_method("desktop_control_inspect_ui", &serde_json::json!({ "action": "find_element" })),
            "uia_query"
        );
        assert_eq!(
            determine_interaction_method("desktop_control_mouse_action", &serde_json::json!({ "image_x": 100, "image_y": 200 })),
            "vision_coordinate_mouse"
        );
        assert_eq!(
            determine_interaction_method("desktop_control_mouse_action", &serde_json::json!({ "x": 100, "y": 200 })),
            "desktop_coordinate_mouse"
        );
        assert_eq!(
            determine_interaction_method("desktop_control_keyboard_action", &serde_json::json!({ "action": "type_text" })),
            "native_keyboard"
        );
        assert_eq!(
            determine_interaction_method("desktop_control_capture_screenshot", &serde_json::json!({})),
            "vision_perception"
        );
    }

    #[test]
    fn test_automation_metrics_and_diagnostics() {
        let mut metrics = AutomationMetrics::default();
        metrics.screenshot_count += 1;
        metrics.uia_query_count += 2;
        metrics.model_requests_count += 3;
        metrics.retry_count += 1;
        metrics.verification_success_count += 1;
        metrics.interaction_methods_used.push("vision_perception".to_string());
        metrics.interaction_methods_used.push("uia_semantic".to_string());

        assert_eq!(metrics.screenshot_count, 1);
        assert_eq!(metrics.uia_query_count, 2);
        assert_eq!(metrics.model_requests_count, 3);
        assert_eq!(metrics.retry_count, 1);
        assert_eq!(metrics.verification_success_count, 1);
        assert_eq!(metrics.interaction_methods_used.len(), 2);

        let diag = StructuredDiagnostic {
            step: 1,
            selected_method: "uia_semantic".to_string(),
            target_id: Some("Save Button".to_string()),
            action: "desktop_control_inspect_ui".to_string(),
            result: "success".to_string(),
            verification_evidence: Some("InvokePattern confirmed".to_string()),
            recovery_reason: None,
        };
        assert_eq!(diag.result, "success");
        assert_eq!(diag.selected_method, "uia_semantic");
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
        // Visible tokens must NOT expose reasoning content
        assert_eq!(extract_visible_token(&val), None);
        assert_eq!(extract_token_from_json(&val), None);
        // Reasoning extractor captures internal thinking
        assert_eq!(extract_reasoning_from_json(&val), Some("Thinking step".to_string()));
    }

    #[test]
    fn test_default_model_id_is_gemma_4() {
        assert_eq!(DEFAULT_MODEL_ID, "@cf/google/gemma-4-26b-a4b-it");
        assert_eq!(MODEL_ID, "@cf/google/gemma-4-26b-a4b-it");
    }

    #[test]
    fn test_repair_json_string() {
        // 1. Unclosed braces
        let unclosed = r#"{"app_name": "notepad""#;
        assert_eq!(repair_json_string(unclosed), serde_json::json!({"app_name": "notepad"}));

        // 2. Trailing comma before missing brace
        let trailing_comma = r#"{"app_name": "notepad","#;
        assert_eq!(repair_json_string(trailing_comma), serde_json::json!({"app_name": "notepad"}));

        // 3. Unclosed quote and braces
        let unclosed_quote = r#"{"text": "Hello world"#;
        assert_eq!(repair_json_string(unclosed_quote), serde_json::json!({"text": "Hello world"}));

        // 4. Markdown code fenced JSON
        let fenced = "```json\n{\"app_name\": \"calc\"}\n```";
        assert_eq!(repair_json_string(fenced), serde_json::json!({"app_name": "calc"}));

        // 5. Empty string
        assert_eq!(repair_json_string(""), serde_json::json!({}));
    }

    #[test]
    fn test_accumulated_tool_call_no_duplicate_name() {
        let mut acc = AccumulatedToolCall::default();
        let chunk1: serde_json::Value = serde_json::from_str(
            r#"{"choices":[{"delta":{"tool_calls":[{"id":"call_1","function":{"name":"desktop_control_launch_app","arguments":"{\"app"}}]}}]}"#
        ).unwrap();
        let chunk2: serde_json::Value = serde_json::from_str(
            r#"{"choices":[{"delta":{"tool_calls":[{"function":{"name":"desktop_control_launch_app","arguments":"_name\":\"notepad\"}"}}]}}]}"#
        ).unwrap();

        acc.update_from_chunk(&chunk1);
        acc.update_from_chunk(&chunk2);

        // Name must not duplicate into "desktop_control_launch_appdesktop_control_launch_app"
        assert_eq!(acc.name, "desktop_control_launch_app");
        assert_eq!(acc.parse_arguments(), serde_json::json!({"app_name": "notepad"}));
    }

    #[test]
    fn test_indicates_pending_action() {
        assert!(indicates_pending_action("Notepad is open. Next, I will type Hello into the window."));
        assert!(indicates_pending_action("I will now focus the application."));
        assert!(indicates_pending_action("Proceeding to click the submit button."));
        assert!(indicates_pending_action("Now I'll add page borders via the Design tab."));
        assert!(indicates_pending_action("I'll now add the borders."));
        assert!(indicates_pending_action("Now let's apply the page border."));
        assert!(indicates_pending_action("Now I'm going to format the document."));
        assert!(!indicates_pending_action("I have typed Hello into Notepad. The task is completed."));
        assert!(!indicates_pending_action("The current time in Tokyo is 10:30 AM."));
    }

    #[test]
    fn test_strip_reasoning_from_text() {
        let text_with_thought = "<thought>Thinking about what app to open</thought>I will open Notepad for you.";
        assert_eq!(strip_reasoning_from_text(text_with_thought), "I will open Notepad for you.");

        let text_with_thinking = "<thinking>Analyzing user request</thinking>Here is your answer.";
        assert_eq!(strip_reasoning_from_text(text_with_thinking), "Here is your answer.");
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

    #[test]
    fn test_is_action_request_classification() {
        assert!(is_action_request("Open Notepad and write Hi"));
        assert!(is_action_request("Open WhatsApp and send Hi to Dileep"));
        assert!(is_action_request("Launch calc and calculate 42 * 9"));
        assert!(is_action_request("Click on the submit button"));
        assert!(is_action_request("Type hello in the search bar"));
        assert!(is_action_request("Please close the browser window"));
        assert!(is_action_request("Take a screenshot of the desktop"));

        // Conversational/informational requests should NOT be classified as action requests
        assert!(!is_action_request("What is the capital of France?"));
        assert!(!is_action_request("Explain the difference between TCP and UDP"));
        assert!(!is_action_request("Write a poem about the sunrise"));
    }

    #[test]
    fn test_strip_tool_calls_from_text() {
        let input_with_tags = "I will check that for you.\n<tool_call>{\"name\": \"desktop_control_list_windows\", \"arguments\": {}}</tool_call>\nAll done!";
        let stripped = strip_tool_calls_from_text(input_with_tags);
        assert!(!stripped.contains("<tool_call>"));
        assert!(!stripped.contains("desktop_control_list_windows"));
        assert!(stripped.contains("I will check that for you."));
        assert!(stripped.contains("All done!"));

        let input_with_fenced_json = "Here are the details:\n```json\n{\"name\": \"desktop_control_launch_app\", \"arguments\": {\"app_name\": \"notepad\"}}\n```\nEnjoy!";
        let stripped_fence = strip_tool_calls_from_text(input_with_fenced_json);
        assert!(!stripped_fence.contains("desktop_control_launch_app"));
        assert!(stripped_fence.contains("Here are the details:"));
        assert!(stripped_fence.contains("Enjoy!"));
    }

    #[test]
    fn test_resolve_desktop_control_aliases() {
        let caps = crate::plugins::registry::get_default_native_capabilities("d:\\test");

        // computer.readUI -> desktop_control_inspect_ui
        let read_ui = resolve_capability("computer.readUI", &caps);
        assert!(read_ui.is_some());
        assert_eq!(read_ui.unwrap().id, "desktop_control_inspect_ui");

        // computer.findElement -> desktop_control_inspect_ui
        let find_el = resolve_capability("computer.findElement", &caps);
        assert!(find_el.is_some());
        assert_eq!(find_el.unwrap().id, "desktop_control_inspect_ui");

        // get_focused_control -> desktop_control_inspect_ui
        let focused = resolve_capability("get_focused_control", &caps);
        assert!(focused.is_some());
        assert_eq!(focused.unwrap().id, "desktop_control_inspect_ui");

        // computer.click -> desktop_control_mouse_action
        let click = resolve_capability("computer.click", &caps);
        assert!(click.is_some());
        assert_eq!(click.unwrap().id, "desktop_control_mouse_action");

        // computer.scroll -> desktop_control_mouse_action
        let scroll = resolve_capability("computer.scroll", &caps);
        assert!(scroll.is_some());
        assert_eq!(scroll.unwrap().id, "desktop_control_mouse_action");

        // computer.type -> desktop_control_keyboard_action
        let type_tool = resolve_capability("computer.type", &caps);
        assert!(type_tool.is_some());
        assert_eq!(type_tool.unwrap().id, "desktop_control_keyboard_action");

        // keyboard -> desktop_control_keyboard_action
        let kb = resolve_capability("keyboard", &caps);
        assert!(kb.is_some());
        assert_eq!(kb.unwrap().id, "desktop_control_keyboard_action");
    }

    #[test]
    fn test_normalize_tool_arguments() {
        // readUI shorthand normalization
        let read_args = normalize_tool_arguments("computer.readUI", "desktop_control_inspect_ui", serde_json::json!({}));
        assert_eq!(read_args["action"], "read_tree");

        // findElement shorthand normalization
        let find_args = normalize_tool_arguments("computer.findElement", "desktop_control_inspect_ui", serde_json::json!({ "query": "Editor" }));
        assert_eq!(find_args["action"], "find_element");
        assert_eq!(find_args["query"], "Editor");

        // type shorthand normalization
        let type_args = normalize_tool_arguments("computer.type", "desktop_control_keyboard_action", serde_json::json!({
            "text": "Hello World",
            "mode": "replace"
        }));
        assert_eq!(type_args["action"], "type_text");
        assert_eq!(type_args["text"], "Hello World");
        assert_eq!(type_args["mode"], "replace");

        // scroll shorthand normalization
        let scroll_args = normalize_tool_arguments("computer.scroll", "desktop_control_mouse_action", serde_json::json!({
            "scroll_y": -5
        }));
        assert_eq!(scroll_args["action"], "scroll");
        assert_eq!(scroll_args["scroll_y"], -5);

        // targeted UIA shorthand normalizations
        let read_text_args = normalize_tool_arguments("read_text", "desktop_control_inspect_ui", serde_json::json!({ "query": "Editor" }));
        assert_eq!(read_text_args["action"], "read_text");

        let inspect_win_args = normalize_tool_arguments("inspect_window", "desktop_control_inspect_ui", serde_json::json!({}));
        assert_eq!(inspect_win_args["action"], "inspect_window");

        let bounds_args = normalize_tool_arguments("get_element_bounds", "desktop_control_inspect_ui", serde_json::json!({ "query": "Save" }));
        assert_eq!(bounds_args["action"], "get_element_bounds");

        let reval_args = normalize_tool_arguments("revalidate_element", "desktop_control_inspect_ui", serde_json::json!({ "query": "Save" }));
        assert_eq!(reval_args["action"], "revalidate_element");
    }

    #[test]
    fn test_is_vision_model_compatibility() {
        assert!(is_vision_model("@cf/meta/llama-3.2-11b-vision-instruct"));
        assert!(is_vision_model("@cf/unum/uform-gen2-qwen-500m"));
        assert!(is_vision_model("gpt-4o"));
        assert!(is_vision_model("gemini-2.0-flash"));

        // Text-only models
        assert!(!is_vision_model("@cf/google/gemma-4-26b-a4b-it"));
        assert!(!is_vision_model("@cf/qwen/qwen3.8-27b"));
    }

    #[test]
    fn test_evaluate_task_completion_gate_prevents_premature_termination() {
        let prompt = "Open Notepad and type 'Aven Desktop Agent'";

        // Step 1: Nothing executed yet -> must require launch
        let outcome1 = evaluate_task_completion_gate(prompt, &[], 1, 25, "I will help you open Notepad.");
        assert_eq!(outcome1, GateOutcome::RequireContinuation {
            prompt: "The requested application has not been launched yet. Please call desktop_control_launch_app with app_name: 'notepad' now.".to_string(),
            reason: "unexecuted_launch",
        });

        // Step 2: App launched, but text not typed yet -> must require typing
        let tools2 = vec!["desktop_control_launch_app".to_string()];
        let outcome2 = evaluate_task_completion_gate(prompt, &tools2, 2, 25, "Notepad is open.");
        assert_eq!(outcome2, GateOutcome::RequireContinuation {
            prompt: "The requested text has not been typed yet. Please call desktop_control_keyboard_action with action: 'type_text' to type the requested text into the active target now.".to_string(),
            reason: "unexecuted_typing",
        });

        // Step 3: Text typed, but no screenshot verification -> must require screenshot verification
        let tools3 = vec![
            "desktop_control_launch_app".to_string(),
            "desktop_control_keyboard_action".to_string(),
        ];
        let outcome3 = evaluate_task_completion_gate(prompt, &tools3, 3, 25, "I have typed the text.");
        assert_eq!(outcome3, GateOutcome::RequireContinuation {
            prompt: "Desktop action was executed. Please call desktop_control_capture_screenshot now to observe the screen, verify the outcome with visual evidence, and confirm the task is complete.".to_string(),
            reason: "unverified_desktop_state",
        });

        // Step 4: Verification screenshot taken -> outcome confirmed -> allow completion!
        let tools4 = vec![
            "desktop_control_launch_app".to_string(),
            "desktop_control_keyboard_action".to_string(),
            "desktop_control_capture_screenshot".to_string(),
        ];
        let outcome4 = evaluate_task_completion_gate(prompt, &tools4, 4, 25, "I confirmed 'Aven Desktop Agent' is typed in Notepad.");
        assert_eq!(outcome4, GateOutcome::AllowCompletion);
    }

    #[test]
    fn test_evaluate_task_completion_gate_non_action_allows_completion() {
        let prompt = "What is the capital of Italy?";
        let outcome = evaluate_task_completion_gate(prompt, &[], 1, 25, "The capital of Italy is Rome.");
        assert_eq!(outcome, GateOutcome::AllowCompletion);
    }

    #[test]
    fn test_evaluate_task_completion_gate_pending_intent_forces_continuation() {
        let prompt = "Click on the Save button";
        let tools = vec!["desktop_control_capture_screenshot".to_string()];
        let outcome = evaluate_task_completion_gate(prompt, &tools, 2, 25, "I will now click on the Save button.");
        assert_eq!(outcome, GateOutcome::RequireContinuation {
            prompt: "Please proceed immediately with executing the next planned step by calling the required desktop tool directly. Do not stop until all actions are executed.".to_string(),
            reason: "model_indicated_pending_action",
        });
    }

    #[test]
    fn test_map_tool_to_factual_label() {
        assert_eq!(
            map_tool_to_factual_label("desktop_control_capture_screenshot", &serde_json::json!({})),
            "Capturing screen"
        );
        assert_eq!(
            map_tool_to_factual_label("desktop_control_launch_app", &serde_json::json!({ "app_name": "notepad" })),
            "Opening Notepad"
        );
        assert_eq!(
            map_tool_to_factual_label("desktop_control_launch_app", &serde_json::json!({ "app_name": "winword" })),
            "Opening Microsoft Word"
        );
        assert_eq!(
            map_tool_to_factual_label("desktop_control_keyboard_action", &serde_json::json!({ "action": "type_text" })),
            "Typing"
        );
        assert_eq!(
            map_tool_to_factual_label("desktop_control_mouse_action", &serde_json::json!({ "target_description": "Save file dialog button" })),
            "Opening file picker"
        );
        assert_eq!(
            map_tool_to_factual_label("desktop_control_mouse_action", &serde_json::json!({ "target_description": "report.pdf file item" })),
            "Selecting file"
        );
        assert_eq!(
            map_tool_to_factual_label("desktop_control_mouse_action", &serde_json::json!({ "target_description": "Design tab" })),
            "Opening Design tab"
        );
        assert_eq!(
            map_tool_to_factual_label("desktop_control_mouse_action", &serde_json::json!({ "target_description": "Page Borders button" })),
            "Applying Page Borders"
        );
        assert_eq!(
            map_tool_to_factual_label("desktop_control_wait_for_ui", &serde_json::json!({})),
            "Waiting for application"
        );
        assert_eq!(
            map_tool_to_factual_label("desktop_control_inspect_ui", &serde_json::json!({})),
            "Verifying result"
        );
        assert_eq!(
            map_tool_to_factual_label("native_file_access", &serde_json::json!({ "action": "read", "path": "public/index.html" })),
            "Reading file"
        );
    }

    #[test]
    fn test_sanitize_activity_label_prevents_raw_html_and_payload_leaks() {
        // Raw HTML or DOCTYPE must never leak as an activity label
        assert_eq!(
            sanitize_activity_label("<!DOCTYPE html><html><head><title>Aven Logo</title></head></html>"),
            "Analyzing screen"
        );
        assert_eq!(
            sanitize_activity_label("<div class=\"content\">Some file text</div>"),
            "Analyzing screen"
        );
        assert_eq!(
            sanitize_activity_label("{\"status\":\"ok\",\"content\":\"very long json payload\"}"),
            "Analyzing screen"
        );

        // Clean action descriptions must be preserved
        assert_eq!(sanitize_activity_label("Opening Microsoft Word"), "Opening Microsoft Word");
        assert_eq!(sanitize_activity_label("Applying Page Borders"), "Applying Page Borders");
        assert_eq!(sanitize_activity_label("Typing self-introduction"), "Typing self-introduction");
    }

    #[test]
    fn test_word_task_completion_gate_requires_borders_and_verification() {
        let prompt = "Open Word and introduce yourself and make sure the page have borders.";

        // Step 1: Nothing executed yet -> must require launch
        let outcome1 = evaluate_task_completion_gate(prompt, &[], 1, 25, "I will help you with Word.");
        assert_eq!(outcome1, GateOutcome::RequireContinuation {
            prompt: "The requested application has not been launched yet. Please call desktop_control_launch_app with app_name: 'winword' now.".to_string(),
            reason: "unexecuted_launch",
        });

        // Step 2: App launched, introduction not typed yet -> must require typing
        let tools2 = vec!["desktop_control_launch_app".to_string()];
        let outcome2 = evaluate_task_completion_gate(prompt, &tools2, 2, 25, "Word is now open.");
        assert_eq!(outcome2, GateOutcome::RequireContinuation {
            prompt: "The requested text has not been typed yet. Please call desktop_control_keyboard_action with action: 'type_text' to type the requested text into the active target now.".to_string(),
            reason: "unexecuted_typing",
        });

        // Step 3: Word launched and text typed, model announces border intention -> must require continuation
        let tools3 = vec![
            "desktop_control_launch_app".to_string(),
            "desktop_control_keyboard_action".to_string(),
        ];
        let outcome3 = evaluate_task_completion_gate(prompt, &tools3, 3, 25, "Now I'll add page borders via the Design tab.");
        assert_eq!(outcome3, GateOutcome::RequireContinuation {
            prompt: "Please proceed immediately with executing the next planned step by calling the required desktop tool directly. Do not stop until all actions are executed.".to_string(),
            reason: "model_indicated_pending_action",
        });

        // Step 4: Text typed, but border actions not performed yet -> must require border application
        let outcome4 = evaluate_task_completion_gate(prompt, &tools3, 4, 25, "Introduction is written.");
        assert_eq!(outcome4, GateOutcome::RequireContinuation {
            prompt: "The user requested page borders ('make sure the page have borders'). Please navigate to the Design tab or Page Borders control in Word, apply the page border, and capture a screenshot to visually verify that the page border is applied before completing.".to_string(),
            reason: "unexecuted_borders",
        });

        // Step 5: Border navigated and applied, followed by screenshot verification -> outcome confirmed!
        let tools5 = vec![
            "desktop_control_launch_app".to_string(),
            "desktop_control_keyboard_action".to_string(),
            "desktop_control_mouse_action".to_string(),
            "desktop_control_mouse_action".to_string(),
            "desktop_control_capture_screenshot".to_string(),
        ];
        let outcome5 = evaluate_task_completion_gate(prompt, &tools5, 5, 25, "I have applied page borders in Word and verified the document visually.");
        assert_eq!(outcome5, GateOutcome::AllowCompletion);
    }

    #[test]
    fn test_watchdog_heartbeat_tracking() {
        use crate::plugins::desktop_control::cursor;
        use crate::plugins::desktop_control::overlay::AutomationSessionState;

        let test_session = "test-session-heartbeat-123";
        cursor::touch_task_heartbeat(test_session, AutomationSessionState::WaitingForModel);
        cursor::touch_activity();
        cursor::touch_task_heartbeat(test_session, AutomationSessionState::ExecutingTool);
        cursor::touch_task_heartbeat(test_session, AutomationSessionState::Verifying);
    }
}

