use serde::{Deserialize, Serialize};

/// Context specifications for supported AI models
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelContextSpec {
    pub max_context_tokens: usize,
    pub max_completion_tokens: usize,
    pub safety_margin_tokens: usize,
}

impl ModelContextSpec {
    pub const fn new(max_context: usize, max_completion: usize, safety_margin: usize) -> Self {
        Self {
            max_context_tokens: max_context,
            max_completion_tokens: max_completion,
            safety_margin_tokens: safety_margin,
        }
    }

    /// Available input budget before compaction is required
    pub fn max_input_budget(&self) -> usize {
        self.max_context_tokens
            .saturating_sub(self.max_completion_tokens)
            .saturating_sub(self.safety_margin_tokens)
    }
}

/// Retrieve model-specific context window specification
pub fn get_model_spec(model_id: &str) -> ModelContextSpec {
    let lower = model_id.to_lowercase();
    if lower.contains("gemma-4") || lower.contains("gemma") {
        // @cf/google/gemma-4-26b-a4b-it: MoE 26B/4B
        // Documented up to 256,000 tokens. Deployed platform budget: 131,072 tokens with 4096 output reserve.
        ModelContextSpec::new(131072, 4096, 2048)
    } else if lower.contains("qwen") {
        // @cf/qwen/qwen3.8-27b: 32,768 context limit
        ModelContextSpec::new(32768, 4096, 1024)
    } else if lower.contains("gemini-2") || lower.contains("gemini-1.5") {
        // Gemini 1.5 / 2.0 / 2.5 Flash / Pro: 1M tokens
        ModelContextSpec::new(1048576, 8192, 4096)
    } else if lower.contains("gpt-4o") {
        // OpenAI GPT-4o / mini: 128k
        ModelContextSpec::new(128000, 4096, 2048)
    } else if lower.contains("claude-3") || lower.contains("claude-3.5") {
        // Anthropic Claude 3.5: 200k
        ModelContextSpec::new(200000, 4096, 2048)
    } else if lower.contains("llama-3-8b") || lower.contains("llama-3.1-8b") {
        // Llama 3 8B: 8k-128k depending on provider
        ModelContextSpec::new(8192, 2048, 512)
    } else if lower.contains("mistral") {
        ModelContextSpec::new(32768, 4096, 1024)
    } else {
        // Conservative default for unknown models
        ModelContextSpec::new(8192, 2048, 512)
    }
}

/// Estimate token count for a text string using standard subword heuristic
/// (~3.5 to 4 characters per token for English text; ~2 to 2.5 chars per token for code/JSON)
pub fn estimate_tokens(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }

    let char_count = text.chars().count();
    let word_count = text.split_whitespace().count();

    // Mixed heuristic: base on word count + character count
    // Average English token is ~0.75 words, or ~4 chars. Code/JSON has more tokens per char.
    let by_chars = (char_count + 3) / 4;
    let by_words = (word_count * 4) / 3;

    std::cmp::max(by_chars, by_words).max(1)
}

/// Estimate total tokens for a list of chat messages
pub fn estimate_messages_tokens<M: AsRef<str>>(messages: &[(M, M)]) -> usize {
    let mut total = 0;
    for (role, content) in messages {
        // Fixed overhead per message envelope: ~4 tokens (<|im_start|>role\n ... <|im_end|>)
        total += 4;
        total += estimate_tokens(role.as_ref());
        total += estimate_tokens(content.as_ref());
    }
    total
}

/// Bounded maximum length for tool output results to prevent any single tool result from blowing up context
pub const MAX_TOOL_OUTPUT_CHARS: usize = 3500;

/// Bound oversized tool output results before feeding into conversation context
pub fn bound_tool_result(tool_name: &str, output: &str) -> String {
    let trimmed = output.trim();
    let lower_tool = tool_name.to_lowercase();

    // Screenshots should ALWAYS omit raw base64 data regardless of length
    if lower_tool.contains("screenshot") {
        if let Ok(mut val) = serde_json::from_str::<serde_json::Value>(trimmed) {
            if val.get("image_base64").is_some() {
                val["image_base64"] = serde_json::json!("[base64 image data omitted from context; saved on disk]");
                let processed = serde_json::to_string_pretty(&val).unwrap_or_else(|_| trimmed.to_string());
                if processed.len() <= MAX_TOOL_OUTPUT_CHARS {
                    return processed;
                }
                return bound_tool_result("tool", &processed);
            }
        }
    }

    if trimmed.len() <= MAX_TOOL_OUTPUT_CHARS {
        return trimmed.to_string();
    }

    // Truncate cleanly at a line boundary with informative footer
    let mut truncated: String = trimmed.chars().take(MAX_TOOL_OUTPUT_CHARS).collect();
    if let Some(last_newline) = truncated.rfind('\n') {
        if last_newline > MAX_TOOL_OUTPUT_CHARS / 2 {
            truncated.truncate(last_newline);
        }
    }

    format!(
        "{}\n\n[... Tool output truncated: showing first {} characters out of {} total to fit context ...]",
        truncated,
        truncated.len(),
        trimmed.len()
    )
}

/// Context compaction options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionResult {
    pub compacted: bool,
    pub original_estimated_tokens: usize,
    pub final_estimated_tokens: usize,
    pub omitted_turn_count: usize,
    pub summary_generated: bool,
}

/// Compact a list of message pairs (role, content) to fit within max input token budget.
/// Guarantees:
/// 1. System prompt (if first message is "system") is ALWAYS preserved intact.
/// 2. Latest user message (and recent turns) is ALWAYS preserved intact.
/// 3. Older completed tool calls and conversations are condensed into structured summaries.
/// 4. Repetitive consecutive tool calls are deduplicated.
pub fn compact_context(
    messages: &mut Vec<(String, String)>,
    spec: &ModelContextSpec,
    tools_tokens: usize,
) -> CompactionResult {
    let budget = spec.max_input_budget().saturating_sub(tools_tokens);
    let original_estimate = estimate_messages_tokens(messages);

    // Step 1: Deduplicate consecutive repetitive tool outputs (e.g. repeated waits or screenshots)
    let mut deduped: Vec<(String, String)> = Vec::with_capacity(messages.len());
    let mut had_duplicates = false;
    for msg in messages.iter() {
        if let Some(last) = deduped.last() {
            if last.0 == msg.0 && last.1 == msg.1 && msg.0 == "tool" {
                had_duplicates = true;
                continue; // Skip exact duplicate consecutive tool result
            }
        }
        deduped.push(msg.clone());
    }

    if had_duplicates {
        *messages = deduped;
    }

    let mut current_estimate = estimate_messages_tokens(messages);
    if current_estimate <= budget || messages.len() <= 2 {
        return CompactionResult {
            compacted: had_duplicates,
            original_estimated_tokens: original_estimate,
            final_estimated_tokens: current_estimate,
            omitted_turn_count: 0,
            summary_generated: false,
        };
    }

    // Step 2: Separate system prompt and recent turns
    let has_system = !messages.is_empty() && messages[0].0 == "system";
    let system_msg = if has_system {
        Some(messages[0].clone())
    } else {
        None
    };

    let start_idx = if has_system { 1 } else { 0 };
    let non_system = &messages[start_idx..];

    // Always preserve at least the last 2 messages (e.g. latest user message and active tool step)
    let preserve_count = 3.min(non_system.len());
    let split_point = non_system.len().saturating_sub(preserve_count);

    let older_turns = &non_system[..split_point];
    let recent_turns = &non_system[split_point..];

    // Step 3: Generate a structured summary of older turns
    let mut goals = Vec::new();
    let mut completed_actions = Vec::new();
    let mut facts = Vec::new();

    for (role, content) in older_turns {
        if role == "user" {
            let first_line = content.lines().next().unwrap_or(content).trim();
            if !first_line.is_empty() {
                goals.push(first_line.to_string());
            }
        } else if role == "tool" || role == "assistant" {
            if content.contains("[Verified Tool Result") || content.contains("desktop_control_") || content.contains("status") {
                let first_line = content.lines().next().unwrap_or(content).trim();
                let summary_item = if first_line.len() > 100 {
                    format!("{}...", &first_line[..100])
                } else {
                    first_line.to_string()
                };
                completed_actions.push(summary_item);
            } else if content.len() < 200 {
                facts.push(content.clone());
            }
        }
    }

    let mut summary_lines = Vec::new();
    summary_lines.push("[Context Summary of Earlier Conversation]:".to_string());
    if !goals.is_empty() {
        summary_lines.push(format!("- User objectives: {}", goals.join(" -> ")));
    }
    if !completed_actions.is_empty() {
        // Keep up to 5 most recent completed actions
        let start = completed_actions.len().saturating_sub(5);
        summary_lines.push(format!("- Completed actions: {}", completed_actions[start..].join("; ")));
    }
    if !facts.is_empty() {
        summary_lines.push(format!("- Established facts: {}", facts.join("; ")));
    }

    let summary_text = summary_lines.join("\n");

    // Reconstruct compacted messages
    let mut compacted_messages = Vec::new();
    if let Some(sys) = system_msg {
        compacted_messages.push(sys);
    }
    compacted_messages.push(("system".to_string(), summary_text));
    compacted_messages.extend_from_slice(recent_turns);

    *messages = compacted_messages;
    current_estimate = estimate_messages_tokens(messages);

    CompactionResult {
        compacted: true,
        original_estimated_tokens: original_estimate,
        final_estimated_tokens: current_estimate,
        omitted_turn_count: split_point,
        summary_generated: true,
    }
}

/// Check if an error message from an AI provider represents a context length exceeded error
pub fn is_context_length_error(err_text: &str) -> bool {
    let lower = err_text.to_lowercase();
    lower.contains("context length")
        || lower.contains("maximum context")
        || lower.contains("token limit")
        || lower.contains("too many tokens")
        || lower.contains("prompt is too long")
        || lower.contains("context window")
        || lower.contains("max_tokens exceeded")
        || lower.contains("exceeds the limit")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_spec_limits() {
        let qwen = get_model_spec("@cf/qwen/qwen3.8-27b");
        assert_eq!(qwen.max_context_tokens, 32768);
        assert_eq!(qwen.max_completion_tokens, 4096);
        assert_eq!(qwen.safety_margin_tokens, 1024);
        assert_eq!(qwen.max_input_budget(), 32768 - 4096 - 1024);

        let gemini = get_model_spec("gemini-2.5-flash");
        assert_eq!(gemini.max_context_tokens, 1048576);

        let gpt4o = get_model_spec("gpt-4o");
        assert_eq!(gpt4o.max_context_tokens, 128000);
    }

    #[test]
    fn test_bound_tool_result() {
        let short_out = "Window list: Notepad (HWND 1234)";
        assert_eq!(bound_tool_result("list_windows", short_out), short_out);

        let long_out = "A".repeat(10000);
        let bounded = bound_tool_result("read_tree", &long_out);
        assert!(bounded.len() < 4000);
        assert!(bounded.contains("Tool output truncated"));
    }

    #[test]
    fn test_bound_screenshot_omits_base64() {
        let shot_json = serde_json::json!({
            "file_path": "C:/shots/1.png",
            "width": 1920,
            "height": 1080,
            "image_base64": "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAA..."
        });
        let bounded = bound_tool_result("desktop_control_capture_screenshot", &shot_json.to_string());
        assert!(!bounded.contains("iVBORw0KGgoAAAANSUhEUgAA"));
        assert!(bounded.contains("base64 image data omitted from context"));
        assert!(bounded.contains("1.png"));
    }

    #[test]
    fn test_compact_long_conversation() {
        let spec = ModelContextSpec::new(70, 20, 10); // Available input budget: 40 tokens
        let mut messages = vec![
            ("system".to_string(), "You are Aven AI Agent.".to_string()),
            ("user".to_string(), "First objective: Open Notepad and write text".to_string()),
            ("assistant".to_string(), "I will launch Notepad for you".to_string()),
            ("tool".to_string(), "Notepad launched successfully with HWND 12345".to_string()),
            ("user".to_string(), "Second objective: Type Hi into Notepad".to_string()),
            ("assistant".to_string(), "Typing Hi into Notepad window".to_string()),
            ("tool".to_string(), "Typed 2 characters into active Notepad window".to_string()),
            ("user".to_string(), "What is the third step?".to_string()),
        ];

        let res = compact_context(&mut messages, &spec, 10);
        assert!(res.compacted);
        assert!(res.summary_generated);
        assert_eq!(messages[0].0, "system"); // System prompt preserved
        assert_eq!(messages[0].1, "You are Aven AI Agent.");
        assert!(messages[1].1.contains("[Context Summary of Earlier Conversation]:"));
        // Latest user message preserved
        assert_eq!(messages.last().unwrap().1, "What is the third step?");
    }

    #[test]
    fn test_is_context_length_error() {
        assert!(is_context_length_error("Error 400: Maximum context length exceeded"));
        assert!(is_context_length_error("Prompt is too long: token limit 8192 exceeded"));
        assert!(is_context_length_error("Too many tokens in request"));
        assert!(!is_context_length_error("Error 401: Unauthorized"));
        assert!(!is_context_length_error("Error 500: Internal server error"));
    }

    #[test]
    fn test_deduplicate_consecutive_repeated_tool_calls() {
        let spec = ModelContextSpec::new(5000, 1000, 500);
        let mut messages = vec![
            ("system".to_string(), "You are Aven.".to_string()),
            ("user".to_string(), "Wait for window".to_string()),
            ("tool".to_string(), "{\"status\": \"still waiting\"}".to_string()),
            ("tool".to_string(), "{\"status\": \"still waiting\"}".to_string()), // duplicate
            ("tool".to_string(), "{\"status\": \"still waiting\"}".to_string()), // duplicate
            ("tool".to_string(), "{\"status\": \"window found\"}".to_string()),
            ("user".to_string(), "Now click button".to_string()),
        ];

        let res = compact_context(&mut messages, &spec, 100);
        assert!(res.compacted);
        // Repeated identical tool outputs should be deduplicated
        let waiting_count = messages.iter().filter(|m| m.1 == "{\"status\": \"still waiting\"}").count();
        assert_eq!(waiting_count, 1);
    }

    #[test]
    fn test_bound_extremely_large_tool_output() {
        let huge_tree = "{\"element\": \"Button\", \"id\": 12345}\n".repeat(2000); // ~74KB
        let bounded = bound_tool_result("desktop_control_inspect_ui", &huge_tree);
        assert!(bounded.len() <= MAX_TOOL_OUTPUT_CHARS + 200);
        assert!(bounded.contains("Tool output truncated"));
    }

    #[test]
    fn test_gemma_4_model_spec() {
        let spec = get_model_spec("@cf/google/gemma-4-26b-a4b-it");
        assert_eq!(spec.max_context_tokens, 131072);
        assert_eq!(spec.max_completion_tokens, 4096);
        assert_eq!(spec.safety_margin_tokens, 2048);
        assert!(spec.max_input_budget() > 120000);
    }
}
