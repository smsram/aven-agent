use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{AppHandle, Manager};
use tokio::sync::Mutex;

const STORAGE_FILE_NAME: &str = "aven_conversations.json";
const STORAGE_TMP_FILE_NAME: &str = "aven_conversations.json.tmp";
const CURRENT_STORAGE_VERSION: u32 = 1;

/// A single process/activity step executed during assistant response generation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PersistedProcessStep {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integration_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    pub label: String,
    pub status: String, // "pending", "running", "completed", "failed", "cancelled"
    pub timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Generation metadata captured from actual provider usage and monotonic timing
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct PersistedMessageMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_formatted: Option<String>,
}

/// Individual persisted message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PersistedChatMessage {
    pub id: String,
    pub sender: String, // "user" | "assistant"
    pub text: String,
    pub timestamp: String,
    pub status: String, // "thinking", "done", "error", "interrupted"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<PersistedMessageMetadata>,
    #[serde(default)]
    pub process_steps: Vec<PersistedProcessStep>,
    #[serde(default)]
    pub logs: Vec<serde_json::Value>,
}

/// Full persisted conversation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PersistedConversation {
    pub id: String,
    pub title: String,
    pub prompt: String,
    pub status: String, // "running", "completed", "failed", "paused", "queued"
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<String>,
    pub model: String,
    pub provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    #[serde(default)]
    pub token_usage: Option<serde_json::Value>,
    #[serde(default)]
    pub logs: Vec<serde_json::Value>,
    #[serde(default)]
    pub messages: Vec<PersistedChatMessage>,
    pub updated_at: u64, // epoch ms for stable sorting
}

/// Overall storage envelope with schema version and currently selected conversation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct PersistedStorageFile {
    pub version: u32,
    pub active_conversation_id: Option<String>,
    pub conversations: Vec<PersistedConversation>,
}

/// Global synchronized state for conversation storage
pub struct ChatStorageState {
    pub lock: Arc<Mutex<()>>,
}

impl ChatStorageState {
    pub fn new() -> Self {
        Self {
            lock: Arc::new(Mutex::new(())),
        }
    }
}

/// Resolve storage path in the user's OS application data directory
pub fn get_storage_path(app: &AppHandle) -> Result<PathBuf, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .or_else(|_| app.path().app_config_dir())
        .map_err(|e| format!("Failed to resolve application data directory: {e}"))?;

    if !data_dir.exists() {
        fs::create_dir_all(&data_dir)
            .map_err(|e| format!("Failed to create storage directory {}: {e}", data_dir.display()))?;
    }

    Ok(data_dir.join(STORAGE_FILE_NAME))
}

/// Normalize interrupted generation state across application restarts:
/// Any message left in 'thinking' status is marked 'interrupted' / 'error'
/// Any running process step is marked 'cancelled'
pub fn sanitize_interrupted_state(file_data: &mut PersistedStorageFile) -> bool {
    let mut modified = false;

    for conv in &mut file_data.conversations {
        let mut conv_had_interrupted = false;

        for msg in &mut conv.messages {
            if msg.sender == "assistant" && (msg.status == "thinking" || msg.status == "running") {
                msg.status = "error".to_string();
                conv_had_interrupted = true;
                modified = true;

                // Mark any unfinished process steps as cancelled
                for step in &mut msg.process_steps {
                    if step.status == "running" || step.status == "pending" {
                        step.status = "cancelled".to_string();
                    }
                }

                // Append an explicit interrupted log so the user understands the state
                let log_entry = serde_json::json!({
                    "timestamp": "Restored",
                    "level": "warn",
                    "message": "Generation was interrupted when the application was closed."
                });
                msg.logs.push(log_entry);
            }
        }

        if conv_had_interrupted && conv.status == "running" {
            conv.status = "completed".to_string();
            modified = true;
        }
    }

    modified
}

/// Atomically write storage payload to disk
fn write_storage_atomic(storage_path: &Path, payload: &PersistedStorageFile) -> Result<(), String> {
    let parent_dir = storage_path
        .parent()
        .ok_or_else(|| "Invalid storage path parent".to_string())?;

    if !parent_dir.exists() {
        fs::create_dir_all(parent_dir)
            .map_err(|e| format!("Failed to create storage parent directory: {e}"))?;
    }

    let tmp_path = parent_dir.join(STORAGE_TMP_FILE_NAME);
    let serialized = serde_json::to_string_pretty(payload)
        .map_err(|e| format!("Failed to serialize conversations: {e}"))?;

    fs::write(&tmp_path, serialized.as_bytes())
        .map_err(|e| format!("Failed to write temporary conversations file {}: {e}", tmp_path.display()))?;

    if let Err(e) = fs::rename(&tmp_path, storage_path) {
        // Fallback copy if filesystem boundary prevents atomic rename
        if let Err(copy_err) = fs::copy(&tmp_path, storage_path) {
            let _ = fs::remove_file(&tmp_path);
            return Err(format!("Failed to commit conversations file atomically: {e}; copy failed: {copy_err}"));
        }
        let _ = fs::remove_file(&tmp_path);
    }

    Ok(())
}

/// Load all persisted conversations, normalizing any interrupted generation states
pub async fn load_conversations_internal(
    app: &AppHandle,
    state: &ChatStorageState,
) -> Result<PersistedStorageFile, String> {
    let _guard = state.lock.lock().await;
    let path = get_storage_path(app)?;

    if !path.exists() {
        return Ok(PersistedStorageFile {
            version: CURRENT_STORAGE_VERSION,
            active_conversation_id: None,
            conversations: Vec::new(),
        });
    }

    let contents = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read chat history from {}: {e}", path.display()))?;

    if contents.trim().is_empty() {
        return Ok(PersistedStorageFile {
            version: CURRENT_STORAGE_VERSION,
            active_conversation_id: None,
            conversations: Vec::new(),
        });
    }

    let mut storage_file: PersistedStorageFile = match serde_json::from_str(&contents) {
        Ok(data) => data,
        Err(e) => {
            // Attempt migration from legacy raw array if format is an array of tasks
            if let Ok(conv_list) = serde_json::from_str::<Vec<PersistedConversation>>(&contents) {
                PersistedStorageFile {
                    version: CURRENT_STORAGE_VERSION,
                    active_conversation_id: conv_list.first().map(|c| c.id.clone()),
                    conversations: conv_list,
                }
            } else {
                return Err(format!("Failed to deserialize conversations database: {e}"));
            }
        }
    };

    // Sanitize interrupted generation state
    let was_sanitized = sanitize_interrupted_state(&mut storage_file);
    if was_sanitized {
        let _ = write_storage_atomic(&path, &storage_file);
    }

    // Sort conversations by updated_at descending
    storage_file.conversations.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    Ok(storage_file)
}

/// Save or update a single conversation atomically
pub async fn save_conversation_internal(
    app: &AppHandle,
    state: &ChatStorageState,
    mut conversation: PersistedConversation,
) -> Result<(), String> {
    let _guard = state.lock.lock().await;
    let path = get_storage_path(app)?;

    let mut storage_file = if path.exists() {
        let contents = fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&contents).unwrap_or_else(|_| PersistedStorageFile {
            version: CURRENT_STORAGE_VERSION,
            active_conversation_id: None,
            conversations: Vec::new(),
        })
    } else {
        PersistedStorageFile {
            version: CURRENT_STORAGE_VERSION,
            active_conversation_id: None,
            conversations: Vec::new(),
        }
    };

    // Update timestamp if not set
    if conversation.updated_at == 0 {
        conversation.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
    }

    // Upsert conversation
    if let Some(pos) = storage_file.conversations.iter().position(|c| c.id == conversation.id) {
        storage_file.conversations[pos] = conversation;
    } else {
        storage_file.conversations.insert(0, conversation);
    }

    // Keep active_conversation_id updated
    if storage_file.active_conversation_id.is_none() {
        storage_file.active_conversation_id = storage_file.conversations.first().map(|c| c.id.clone());
    }

    // Sort by updated_at descending
    storage_file.conversations.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    write_storage_atomic(&path, &storage_file)
}

/// Delete a conversation by ID
pub async fn delete_conversation_internal(
    app: &AppHandle,
    state: &ChatStorageState,
    conversation_id: String,
) -> Result<(), String> {
    let _guard = state.lock.lock().await;
    let path = get_storage_path(app)?;

    if !path.exists() {
        return Ok(());
    }

    let contents = fs::read_to_string(&path).unwrap_or_default();
    let mut storage_file: PersistedStorageFile =
        serde_json::from_str(&contents).unwrap_or_default();

    storage_file.conversations.retain(|c| c.id != conversation_id);

    if storage_file.active_conversation_id.as_deref() == Some(&conversation_id) {
        storage_file.active_conversation_id = storage_file.conversations.first().map(|c| c.id.clone());
    }

    write_storage_atomic(&path, &storage_file)
}

/// Rename a conversation by ID
pub async fn rename_conversation_internal(
    app: &AppHandle,
    state: &ChatStorageState,
    conversation_id: String,
    new_title: String,
) -> Result<(), String> {
    let _guard = state.lock.lock().await;
    let path = get_storage_path(app)?;

    if !path.exists() {
        return Err(format!("Conversation '{conversation_id}' not found"));
    }

    let contents = fs::read_to_string(&path).unwrap_or_default();
    let mut storage_file: PersistedStorageFile =
        serde_json::from_str(&contents).map_err(|e| format!("Failed to read database: {e}"))?;

    if let Some(conv) = storage_file.conversations.iter_mut().find(|c| c.id == conversation_id) {
        conv.title = new_title;
        conv.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        write_storage_atomic(&path, &storage_file)
    } else {
        Err(format!("Conversation '{conversation_id}' not found"))
    }
}

/// Save the current active conversation ID for seamless session restoration
pub async fn save_active_conversation_id_internal(
    app: &AppHandle,
    state: &ChatStorageState,
    conversation_id: Option<String>,
) -> Result<(), String> {
    let _guard = state.lock.lock().await;
    let path = get_storage_path(app)?;

    if !path.exists() {
        let storage_file = PersistedStorageFile {
            version: CURRENT_STORAGE_VERSION,
            active_conversation_id: conversation_id,
            conversations: Vec::new(),
        };
        return write_storage_atomic(&path, &storage_file);
    }

    let contents = fs::read_to_string(&path).unwrap_or_default();
    let mut storage_file: PersistedStorageFile =
        serde_json::from_str(&contents).unwrap_or_default();

    storage_file.active_conversation_id = conversation_id;
    write_storage_atomic(&path, &storage_file)
}

/// Clear all persisted conversations
pub async fn clear_all_conversations_internal(
    app: &AppHandle,
    state: &ChatStorageState,
) -> Result<(), String> {
    let _guard = state.lock.lock().await;
    let path = get_storage_path(app)?;

    let storage_file = PersistedStorageFile {
        version: CURRENT_STORAGE_VERSION,
        active_conversation_id: None,
        conversations: Vec::new(),
    };

    write_storage_atomic(&path, &storage_file)
}

// -------------------------------------------------------------------------------------------------
// Tauri Command Handlers
// -------------------------------------------------------------------------------------------------

#[tauri::command]
pub async fn load_conversations(
    app: AppHandle,
    state: tauri::State<'_, Arc<ChatStorageState>>,
) -> Result<PersistedStorageFile, String> {
    load_conversations_internal(&app, &state).await
}

#[tauri::command]
pub async fn save_conversation(
    app: AppHandle,
    state: tauri::State<'_, Arc<ChatStorageState>>,
    conversation: PersistedConversation,
) -> Result<(), String> {
    save_conversation_internal(&app, &state, conversation).await
}

#[tauri::command]
pub async fn delete_conversation(
    app: AppHandle,
    state: tauri::State<'_, Arc<ChatStorageState>>,
    conversation_id: String,
) -> Result<(), String> {
    delete_conversation_internal(&app, &state, conversation_id).await
}

#[tauri::command]
pub async fn rename_conversation(
    app: AppHandle,
    state: tauri::State<'_, Arc<ChatStorageState>>,
    conversation_id: String,
    new_title: String,
) -> Result<(), String> {
    rename_conversation_internal(&app, &state, conversation_id, new_title).await
}

#[tauri::command]
pub async fn save_active_conversation_id(
    app: AppHandle,
    state: tauri::State<'_, Arc<ChatStorageState>>,
    conversation_id: Option<String>,
) -> Result<(), String> {
    save_active_conversation_id_internal(&app, &state, conversation_id).await
}

#[tauri::command]
pub async fn clear_all_conversations(
    app: AppHandle,
    state: tauri::State<'_, Arc<ChatStorageState>>,
) -> Result<(), String> {
    clear_all_conversations_internal(&app, &state).await
}

// -------------------------------------------------------------------------------------------------
// Unit Tests
// -------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_interrupted_state() {
        let mut file = PersistedStorageFile {
            version: 1,
            active_conversation_id: Some("conv_1".into()),
            conversations: vec![PersistedConversation {
                id: "conv_1".into(),
                title: "Test Chat".into(),
                prompt: "Hello".into(),
                status: "running".into(),
                created_at: "10:00 AM".into(),
                completed_at: None,
                duration: None,
                model: "@cf/qwen/qwen3.8-27b".into(),
                provider: "Cloudflare".into(),
                output: None,
                token_usage: None,
                logs: vec![],
                messages: vec![
                    PersistedChatMessage {
                        id: "m1".into(),
                        sender: "user".into(),
                        text: "Hello".into(),
                        timestamp: "10:00 AM".into(),
                        status: "done".into(),
                        metadata: None,
                        process_steps: vec![],
                        logs: vec![],
                    },
                    PersistedChatMessage {
                        id: "m2".into(),
                        sender: "assistant".into(),
                        text: "Partial text...".into(),
                        timestamp: "10:00 AM".into(),
                        status: "thinking".into(), // Was interrupted!
                        metadata: None,
                        process_steps: vec![PersistedProcessStep {
                            id: "s1".into(),
                            integration_id: Some("google_search".into()),
                            tool_name: Some("search".into()),
                            label: "Searching the web".into(),
                            status: "running".into(), // Was running!
                            timestamp: "10:00 AM".into(),
                            error: None,
                        }],
                        logs: vec![],
                    },
                ],
                updated_at: 100,
            }],
        };

        let modified = sanitize_interrupted_state(&mut file);
        assert!(modified);

        let conv = &file.conversations[0];
        assert_eq!(conv.status, "completed");
        let asst_msg = &conv.messages[1];
        assert_eq!(asst_msg.status, "error");
        assert_eq!(asst_msg.process_steps[0].status, "cancelled");
        assert!(asst_msg.logs.iter().any(|l| {
            l.get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("")
                .contains("interrupted")
        }));
    }

    #[test]
    fn test_serialization_roundtrip() {
        let file = PersistedStorageFile {
            version: 1,
            active_conversation_id: Some("conv_abc".into()),
            conversations: vec![PersistedConversation {
                id: "conv_abc".into(),
                title: "My System Info".into(),
                prompt: "What is my device name?".into(),
                status: "completed".into(),
                created_at: "10:15 AM".into(),
                completed_at: Some("10:15 AM".into()),
                duration: Some("1.25s".into()),
                model: "@cf/qwen/qwen3.8-27b".into(),
                provider: "Cloudflare Workers AI".into(),
                output: Some("Your device name is DESKTOP-123".into()),
                token_usage: Some(serde_json::json!({ "totalTokens": 150 })),
                logs: vec![],
                messages: vec![PersistedChatMessage {
                    id: "msg_1".into(),
                    sender: "assistant".into(),
                    text: "Your device name is DESKTOP-123".into(),
                    timestamp: "10:15 AM".into(),
                    status: "done".into(),
                    metadata: Some(PersistedMessageMetadata {
                        model: Some("@cf/qwen/qwen3.8-27b".into()),
                        prompt_tokens: Some(40),
                        completion_tokens: Some(110),
                        total_tokens: Some(150),
                        duration_ms: Some(1250),
                        duration_formatted: Some("1.25s".into()),
                    }),
                    process_steps: vec![PersistedProcessStep {
                        id: "step_1".into(),
                        integration_id: Some("native_system".into()),
                        tool_name: Some("native_system_info".into()),
                        label: "System Diagnostic Info".into(),
                        status: "completed".into(),
                        timestamp: "10:15 AM".into(),
                        error: None,
                    }],
                    logs: vec![],
                }],
                updated_at: 1700000000000,
            }],
        };

        let json = serde_json::to_string(&file).unwrap();
        let decoded: PersistedStorageFile = serde_json::from_str(&json).unwrap();
        assert_eq!(file, decoded);
    }
}
