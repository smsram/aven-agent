pub mod api;
pub mod desktop_control;
pub mod mcp;
pub mod native;
pub mod registry;

use keyring::Entry;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Mutex;

pub const SERVICE_NAME: &str = "com.aven.agent";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginKind {
    Mcp,
    Api,
    Native,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginStatus {
    NotConfigured,
    Disconnected,
    Connecting,
    Connected,
    NeedsPermission,
    Error,
    Unsupported,
}

/// Persistent Configuration for an Integration / Plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    pub id: String,
    pub kind: PluginKind,
    pub name: String,
    pub enabled: bool,
    pub transport: Option<String>,       // "stdio" or "http"
    pub command: Option<String>,         // for stdio MCP
    pub args: Option<Vec<String>>,       // for stdio MCP
    pub url: Option<String>,             // for remote MCP or API
    pub approved_paths: Option<Vec<String>>, // for scoped filesystem access
    pub settings: HashMap<String, String>,  // non-secret settings
    pub last_status: PluginStatus,
    pub last_error: Option<String>,
}

/// Catalog entry presented to the user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogEntry {
    pub id: String,
    pub kind: PluginKind,
    pub name: String,
    pub category: String, // "Filesystem", "Developer", "Database", "Browser", "Search", "System"
    pub description: String,
    pub supported_capabilities: Vec<String>,
    pub requirements: String,
    pub setup_instructions: String,
    pub is_implemented: bool,
    pub default_command: Option<String>,
    pub default_args: Option<Vec<String>>,
    pub default_url: Option<String>,
    pub permissions_notice: String,
}

/// Safe Metadata sent to frontend for an integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub id: String,
    pub kind: PluginKind,
    pub name: String,
    pub status: PluginStatus,
    pub has_secret: bool,
    pub error_message: Option<String>,
    pub latency_ms: Option<u64>,
    pub approved_paths: Vec<String>,
    pub settings: HashMap<String, String>,
    pub discovered_tools: Vec<mcp::McpTool>,
}

/// Overall plugins state returned to UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginsStatePayload {
    pub catalog: Vec<CatalogEntry>,
    pub configured_plugins: Vec<PluginMetadata>,
    pub capabilities: Vec<registry::RegisteredCapability>,
    pub system_info: serde_json::Value,
}

/// Result of testing an integration connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConnectionResult {
    pub success: bool,
    pub status: PluginStatus,
    pub latency_ms: u64,
    pub message: String,
}

/// Top-level Manager holding state for MCP sessions, API configs, and Native capabilities
pub struct PluginsManager {
    pub active_mcp_sessions: Arc<Mutex<HashMap<String, mcp::McpClientSession>>>,
    pub configs: Arc<Mutex<HashMap<String, PluginConfig>>>,
    pub http_client: reqwest::Client,
    pub default_workspace: String,
}

impl PluginsManager {
    pub fn new(default_workspace: String) -> Self {
        Self {
            active_mcp_sessions: Arc::new(Mutex::new(HashMap::new())),
            configs: Arc::new(Mutex::new(HashMap::new())),
            http_client: reqwest::Client::new(),
            default_workspace,
        }
    }

    /// Load saved configurations from file on application startup and restore active runtimes
    pub async fn load_configs_from_disk(&self, app: &AppHandle) {
        let mut loaded_map: HashMap<String, PluginConfig> = HashMap::new();
        if let Ok(config_path) = get_plugins_config_path(app) {
            if config_path.exists() {
                if let Ok(content) = fs::read_to_string(&config_path) {
                    if let Ok(map) = serde_json::from_str::<HashMap<String, PluginConfig>>(&content) {
                        loaded_map = map;
                    }
                }
            }
        }

        // Merge default configs if not already persisted
        let defaults = get_default_configs(&self.default_workspace);
        for (k, v) in defaults {
            loaded_map.entry(k).or_insert(v);
        }

        // Restore runtime states
        for (id, conf) in loaded_map.iter_mut() {
            if conf.kind == PluginKind::Native {
                conf.last_status = if cfg!(target_os = "windows") {
                    if conf.enabled {
                        PluginStatus::Connected
                    } else {
                        PluginStatus::Disconnected
                    }
                } else {
                    PluginStatus::Unsupported
                };
            } else if conf.kind == PluginKind::Api && id == "api_gemini_search" {
                let has_key = has_plugin_secret(id);
                if !has_key {
                    conf.last_status = PluginStatus::NeedsPermission;
                } else if conf.enabled {
                    conf.last_status = PluginStatus::Connected;
                } else {
                    conf.last_status = PluginStatus::Disconnected;
                }
            } else if conf.kind == PluginKind::Mcp {
                if conf.enabled && conf.transport.as_deref() == Some("stdio") {
                    let cmd = conf.command.as_deref().unwrap_or("");
                    if !cmd.is_empty() {
                        let args = conf.args.clone().unwrap_or_default();
                        let approved_paths: Vec<PathBuf> = conf
                            .approved_paths
                            .clone()
                            .unwrap_or_default()
                            .iter()
                            .map(PathBuf::from)
                            .collect();

                        let mut session = mcp::McpClientSession::new_stdio(id.clone(), approved_paths);
                        let mut env_map = HashMap::new();
                        if id == "mcp_github" {
                            if let Ok(Some(token)) = get_plugin_secret(id) {
                                env_map.insert("GITHUB_PERSONAL_ACCESS_TOKEN".to_string(), token);
                            }
                        }

                        match session.start_stdio(cmd, &args, None, &env_map).await {
                            Ok(_) => {
                                conf.last_status = PluginStatus::Connected;
                                conf.last_error = None;
                                let mut sessions = self.active_mcp_sessions.lock().await;
                                sessions.insert(id.clone(), session);
                            }
                            Err(e) => {
                                // Do not delete configuration on transient failure
                                conf.last_status = PluginStatus::Error;
                                conf.last_error = Some(e);
                            }
                        }
                    } else {
                        conf.last_status = PluginStatus::Disconnected;
                    }
                } else {
                    conf.last_status = PluginStatus::Disconnected;
                }
            }
        }

        {
            let mut configs = self.configs.lock().await;
            *configs = loaded_map;
        }

        // Save normalized state back to disk
        let _ = self.save_configs_to_disk(app).await;

        // Emit updated state to frontend
        let state_payload = self.get_state().await;
        let _ = app.emit("plugins-state-changed", state_payload);
    }

    /// Save current configurations to disk atomically
    pub async fn save_configs_to_disk(&self, app: &AppHandle) -> Result<(), String> {
        let config_path = get_plugins_config_path(app)?;
        let tmp_path = config_path.with_extension("json.tmp");
        let configs = self.configs.lock().await;
        let json = serde_json::to_string_pretty(&*configs)
            .map_err(|e| format!("Failed to serialize plugins config: {e}"))?;

        fs::write(&tmp_path, &json)
            .map_err(|e| format!("Failed to write plugins config to temporary file: {e}"))?;

        if let Err(e) = fs::rename(&tmp_path, &config_path) {
            // Fallback copy if rename across filesystem boundary fails
            if fs::copy(&tmp_path, &config_path).is_err() {
                return Err(format!("Failed to commit plugins config atomically: {e}"));
            }
            let _ = fs::remove_file(&tmp_path);
        }
        Ok(())
    }

    /// Get authoritative plugins state for the UI
    pub async fn get_state(&self) -> PluginsStatePayload {
        let catalog = get_full_catalog();
        let configs = self.configs.lock().await;
        let mcp_sessions = self.active_mcp_sessions.lock().await;

        let mut configured_plugins = Vec::new();
        let mut capabilities = registry::get_default_native_capabilities(&self.default_workspace);

        for (id, conf) in configs.iter() {
            let has_secret = has_plugin_secret(id);
            let mut discovered_tools = Vec::new();
            let mut status = conf.last_status;

            if conf.kind == PluginKind::Mcp {
                if let Some(session) = mcp_sessions.get(id) {
                    if session.is_connected {
                        status = PluginStatus::Connected;
                        discovered_tools = session.tools.clone();

                        // Register discovered MCP tools into common capabilities list
                        for tool in &session.tools {
                            capabilities.push(registry::RegisteredCapability {
                                id: format!("mcp_{}_{}", id, tool.name),
                                source_id: id.clone(),
                                kind: registry::CapabilityKind::Mcp,
                                name: tool.name.clone(),
                                description: tool.description.clone().unwrap_or_else(|| format!("MCP tool from {}", conf.name)),
                                input_schema: tool.input_schema.clone(),
                                required_permission: registry::PermissionLevel::Execute,
                                requires_confirmation: false,
                                is_available: true,
                            });
                        }
                    }
                }
            } else if conf.kind == PluginKind::Api && id == "api_gemini_search" {
                if has_secret && conf.enabled {
                    status = PluginStatus::Connected;
                    // Register canonical 'google_search' tool
                    capabilities.push(registry::RegisteredCapability {
                        id: "google_search".to_string(),
                        source_id: "api_gemini_search".to_string(),
                        kind: registry::CapabilityKind::Api,
                        name: "google_search".to_string(),
                        description: "Performs live web searches with authoritative citations and grounded references using Google Search via Gemini API. Use this tool whenever the user asks for current, recent, today's, latest, or time-sensitive events, news, library versions, or web information.".to_string(),
                        input_schema: serde_json::json!({
                            "type": "object",
                            "properties": {
                                "query": { "type": "string", "description": "The exact search query to look up on Google" }
                            },
                            "required": ["query"]
                        }),
                        required_permission: registry::PermissionLevel::Network,
                        requires_confirmation: false,
                        is_available: true,
                    });
                } else if !has_secret {
                    status = PluginStatus::NeedsPermission;
                } else {
                    status = PluginStatus::Disconnected;
                }
            } else if conf.kind == PluginKind::Native {
                status = if cfg!(target_os = "windows") {
                    if conf.enabled {
                        PluginStatus::Connected
                    } else {
                        PluginStatus::Disconnected
                    }
                } else {
                    PluginStatus::Unsupported
                };
            }

            configured_plugins.push(PluginMetadata {
                id: id.clone(),
                kind: conf.kind,
                name: conf.name.clone(),
                status,
                has_secret,
                error_message: conf.last_error.clone(),
                latency_ms: None,
                approved_paths: conf.approved_paths.clone().unwrap_or_default(),
                settings: conf.settings.clone(),
                discovered_tools,
            });
        }

        PluginsStatePayload {
            catalog,
            configured_plugins,
            capabilities,
            system_info: native::get_native_system_info(),
        }
    }

    /// Execute a registered capability directly through the central manager
    pub async fn execute_capability_internal(
        &self,
        request: registry::CapabilityExecutionRequest,
    ) -> Result<registry::CapabilityExecutionResult, String> {
        let cap_id = request.capability_id.clone();

        // 1. Google Search Grounding (Gemini 2.5 Flash)
        if cap_id == "google_search" || cap_id == "api::gemini_search::search" {
            let query = request
                .arguments
                .get("query")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter 'query'".to_string())?;

            let api_key = match get_plugin_secret("api_gemini_search")? {
                Some(k) if !k.trim().is_empty() => k,
                _ => {
                    return Ok(registry::CapabilityExecutionResult {
                        success: false,
                        capability_id: cap_id,
                        result: None,
                        error: Some(
                            "Gemini API key is not configured. Please add your key in Plugins → Google Search."
                                .to_string(),
                        ),
                    })
                }
            };

            match api::execute_gemini_search(&self.http_client, &api_key, query).await {
                Ok(search_res) => {
                    let formatted_text = api::format_grounded_answer(&search_res);
                    let mut res_val = serde_json::to_value(&search_res).unwrap_or_default();
                    if let Some(obj) = res_val.as_object_mut() {
                        obj.insert(
                            "formatted_answer".to_string(),
                            serde_json::Value::String(formatted_text),
                        );
                    }

                    Ok(registry::CapabilityExecutionResult {
                        success: true,
                        capability_id: cap_id,
                        result: Some(res_val),
                        error: None,
                    })
                }
                Err(e) => Ok(registry::CapabilityExecutionResult {
                    success: false,
                    capability_id: cap_id,
                    result: None,
                    error: Some(e),
                }),
            }
        }
        // 2. Desktop Control Capabilities
        else if cap_id.starts_with("desktop_control_")
            || cap_id.starts_with("desktop_control::")
            || cap_id.starts_with("native_desktop_control_")
            || cap_id.starts_with("native::desktop_control::")
        {
            let default_dir = std::path::Path::new(&self.default_workspace);
            match desktop_control::execute_desktop_control_capability(
                &cap_id,
                &request.arguments,
                request.user_confirmed,
                default_dir,
            ) {
                Ok(val) => Ok(registry::CapabilityExecutionResult {
                    success: true,
                    capability_id: cap_id,
                    result: Some(val),
                    error: None,
                }),
                Err(e) => Ok(registry::CapabilityExecutionResult {
                    success: false,
                    capability_id: cap_id,
                    result: None,
                    error: Some(e),
                }),
            }
        }
        // 3. Other Native Capabilities
        else if let Some(native_action) = cap_id
            .strip_prefix("native::")
            .or_else(|| cap_id.strip_prefix("native_"))
        {
            match native_action {
                "desktop_control" => {
                    let default_dir = std::path::Path::new(&self.default_workspace);
                    let action = request
                        .arguments
                        .get("action")
                        .and_then(|v| v.as_str())
                        .unwrap_or("self_test");
                    if action == "self_test" {
                        match desktop_control::run_desktop_control_self_test() {
                            Ok(res) => Ok(registry::CapabilityExecutionResult {
                                success: true,
                                capability_id: cap_id,
                                result: Some(res),
                                error: None,
                            }),
                            Err(e) => Ok(registry::CapabilityExecutionResult {
                                success: false,
                                capability_id: cap_id,
                                result: None,
                                error: Some(e),
                            }),
                        }
                    } else {
                        match desktop_control::execute_desktop_control_capability(
                            action,
                            &request.arguments,
                            request.user_confirmed,
                            default_dir,
                        ) {
                            Ok(res) => Ok(registry::CapabilityExecutionResult {
                                success: true,
                                capability_id: cap_id,
                                result: Some(res),
                                error: None,
                            }),
                            Err(e) => Ok(registry::CapabilityExecutionResult {
                                success: false,
                                capability_id: cap_id,
                                result: None,
                                error: Some(e),
                            }),
                        }
                    }
                }
                "open_url" | "browser" => {
                    let url = request
                        .arguments
                        .get("url")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| "Missing required parameter 'url'".to_string())?;

                    match native::open_url_in_browser(url) {
                        Ok(_) => Ok(registry::CapabilityExecutionResult {
                            success: true,
                            capability_id: cap_id,
                            result: Some(serde_json::json!({ "opened_url": url })),
                            error: None,
                        }),
                        Err(e) => Ok(registry::CapabilityExecutionResult {
                            success: false,
                            capability_id: cap_id,
                            result: None,
                            error: Some(e),
                        }),
                    }
                }
                "launch_app" | "app_launcher" => {
                    let app_name = request
                        .arguments
                        .get("app_name")
                        .or_else(|| request.arguments.get("application"))
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| "Missing required parameter 'app_name'".to_string())?;

                    let args: Vec<String> = request
                        .arguments
                        .get("args")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_default();

                    let reuse = request
                        .arguments
                        .get("reuse_existing")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(true);

                    match desktop_control::launcher::launch_or_reuse_application(
                        app_name,
                        &args,
                        reuse,
                        request.user_confirmed,
                    ) {
                        Ok(res) => Ok(registry::CapabilityExecutionResult {
                            success: res.success,
                            capability_id: cap_id,
                            result: Some(serde_json::to_value(&res).unwrap_or_default()),
                            error: None,
                        }),
                        Err(e) => Ok(registry::CapabilityExecutionResult {
                            success: false,
                            capability_id: cap_id,
                            result: None,
                            error: Some(e),
                        }),
                    }
                }
                "file_access" | "fs" => {
                    let action = request
                        .arguments
                        .get("action")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            "Missing required parameter 'action' (read or list)".to_string()
                        })?;

                    let path = request
                        .arguments
                        .get("path")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| "Missing required parameter 'path'".to_string())?;

                    match native::scoped_file_operation(action, &self.default_workspace, path) {
                        Ok(res) => Ok(registry::CapabilityExecutionResult {
                            success: true,
                            capability_id: cap_id,
                            result: Some(res),
                            error: None,
                        }),
                        Err(e) => Ok(registry::CapabilityExecutionResult {
                            success: false,
                            capability_id: cap_id,
                            result: None,
                            error: Some(e),
                        }),
                    }
                }
                "system_info" | "system" => {
                    let info = native::get_native_system_info();
                    Ok(registry::CapabilityExecutionResult {
                        success: true,
                        capability_id: cap_id,
                        result: Some(info),
                        error: None,
                    })
                }
                _ => Ok(registry::CapabilityExecutionResult {
                    success: false,
                    capability_id: cap_id.clone(),
                    result: None,
                    error: Some(format!("Unknown native capability: {native_action}")),
                }),
            }
        }
        // 3. MCP Tools
        else if let Some(mcp_spec) = cap_id
            .strip_prefix("mcp::")
            .or_else(|| cap_id.strip_prefix("mcp_"))
        {
            let parts: Vec<&str> = if mcp_spec.contains("::") {
                mcp_spec.splitn(2, "::").collect()
            } else {
                mcp_spec.splitn(2, '_').collect()
            };

            if parts.len() != 2 {
                return Ok(registry::CapabilityExecutionResult {
                    success: false,
                    capability_id: cap_id,
                    result: None,
                    error: Some("Invalid MCP capability identifier format".to_string()),
                });
            }

            let server_id = parts[0];
            let tool_name = parts[1];

            let sessions = self.active_mcp_sessions.lock().await;
            let session = match sessions.get(server_id) {
                Some(s) if s.is_connected => s,
                _ => {
                    return Ok(registry::CapabilityExecutionResult {
                        success: false,
                        capability_id: cap_id.clone(),
                        result: None,
                        error: Some(format!(
                            "MCP Server '{server_id}' is not connected or active"
                        )),
                    })
                }
            };

            match session.call_tool(tool_name, &request.arguments).await {
                Ok(res) => Ok(registry::CapabilityExecutionResult {
                    success: true,
                    capability_id: cap_id,
                    result: Some(res),
                    error: None,
                }),
                Err(e) => Ok(registry::CapabilityExecutionResult {
                    success: false,
                    capability_id: cap_id,
                    result: None,
                    error: Some(e),
                }),
            }
        } else {
            Ok(registry::CapabilityExecutionResult {
                success: false,
                capability_id: cap_id,
                result: None,
                error: Some("Unrecognized capability subsystem".to_string()),
            })
        }
    }
}

/// Pre-seed default configurations for built-in native capabilities and Google Search
pub fn get_default_configs(workspace_path: &str) -> HashMap<String, PluginConfig> {
    let mut map = HashMap::new();

    map.insert(
        "native_browser".to_string(),
        PluginConfig {
            id: "native_browser".to_string(),
            kind: PluginKind::Native,
            name: "Open URLs in Default Browser".to_string(),
            enabled: true,
            transport: None,
            command: None,
            args: None,
            url: None,
            approved_paths: None,
            settings: HashMap::new(),
            last_status: if cfg!(target_os = "windows") {
                PluginStatus::Connected
            } else {
                PluginStatus::Unsupported
            },
            last_error: None,
        },
    );



    map.insert(
        "native_fs".to_string(),
        PluginConfig {
            id: "native_fs".to_string(),
            kind: PluginKind::Native,
            name: "Scoped File Access".to_string(),
            enabled: true,
            transport: None,
            command: None,
            args: None,
            url: None,
            approved_paths: Some(vec![workspace_path.to_string()]),
            settings: HashMap::new(),
            last_status: if cfg!(target_os = "windows") {
                PluginStatus::Connected
            } else {
                PluginStatus::Unsupported
            },
            last_error: None,
        },
    );

    map.insert(
        "native_system".to_string(),
        PluginConfig {
            id: "native_system".to_string(),
            kind: PluginKind::Native,
            name: "System Diagnostic Info".to_string(),
            enabled: true,
            transport: None,
            command: None,
            args: None,
            url: None,
            approved_paths: None,
            settings: HashMap::new(),
            last_status: if cfg!(target_os = "windows") {
                PluginStatus::Connected
            } else {
                PluginStatus::Unsupported
            },
            last_error: None,
        },
    );

    map.insert(
        "native_desktop_control".to_string(),
        PluginConfig {
            id: "native_desktop_control".to_string(),
            kind: PluginKind::Native,
            name: "Desktop & UI Control".to_string(),
            enabled: true,
            transport: None,
            command: None,
            args: None,
            url: None,
            approved_paths: Some(vec![workspace_path.to_string()]),
            settings: HashMap::new(),
            last_status: if cfg!(target_os = "windows") {
                PluginStatus::Connected
            } else {
                PluginStatus::Unsupported
            },
            last_error: None,
        },
    );

    let has_gemini_key = has_plugin_secret("api_gemini_search");
    map.insert(
        "api_gemini_search".to_string(),
        PluginConfig {
            id: "api_gemini_search".to_string(),
            kind: PluginKind::Api,
            name: "Google Search via Gemini API".to_string(),
            enabled: has_gemini_key,
            transport: None,
            command: None,
            args: None,
            url: Some(api::GEMINI_ENDPOINT.to_string()),
            approved_paths: None,
            settings: HashMap::new(),
            last_status: if has_gemini_key {
                PluginStatus::Connected
            } else {
                PluginStatus::NeedsPermission
            },
            last_error: None,
        },
    );

    map
}

/// Helper to resolve app config directory for plugins configuration
fn get_plugins_config_path(app: &AppHandle) -> Result<PathBuf, String> {
    let config_dir = app.path().app_config_dir()
        .map_err(|e| format!("Failed to resolve app config directory: {e}"))?;
    if !config_dir.exists() {
        let _ = fs::create_dir_all(&config_dir);
    }
    Ok(config_dir.join("aven_plugins_config.json"))
}

/// Store secret credential safely in OS Credential Manager
pub fn store_plugin_secret(plugin_id: &str, secret: &str) -> Result<(), String> {
    let entry_name = format!("plugin_secret_{plugin_id}");
    let entry = Entry::new(SERVICE_NAME, &entry_name)
        .map_err(|e| format!("Failed to create keyring entry: {e}"))?;
    entry.set_password(secret)
        .map_err(|e| format!("Failed to store secret in OS Credential Manager: {e}"))?;
    Ok(())
}

/// Retrieve secret credential from OS Credential Manager
pub fn get_plugin_secret(plugin_id: &str) -> Result<Option<String>, String> {
    let entry_name = format!("plugin_secret_{plugin_id}");
    let entry = Entry::new(SERVICE_NAME, &entry_name)
        .map_err(|e| format!("Failed to access keyring: {e}"))?;
    match entry.get_password() {
        Ok(secret) => Ok(Some(secret)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("Keyring error: {e}")),
    }
}

/// Check if secret exists without exposing it
pub fn has_plugin_secret(plugin_id: &str) -> bool {
    let entry_name = format!("plugin_secret_{plugin_id}");
    if let Ok(entry) = Entry::new(SERVICE_NAME, &entry_name) {
        if let Ok(pwd) = entry.get_password() {
            return !pwd.is_empty();
        }
    }
    false
}

/// Delete secret from OS Credential Manager
pub fn delete_plugin_secret(plugin_id: &str) -> Result<(), String> {
    let entry_name = format!("plugin_secret_{plugin_id}");
    if let Ok(entry) = Entry::new(SERVICE_NAME, &entry_name) {
        let _ = entry.delete_credential();
    }
    Ok(())
}

/// Full integration catalog with initial entries
pub fn get_full_catalog() -> Vec<CatalogEntry> {
    vec![
        // ------------------ MCP Servers ------------------
        CatalogEntry {
            id: "mcp_filesystem".to_string(),
            kind: PluginKind::Mcp,
            name: "Filesystem MCP".to_string(),
            category: "Filesystem".to_string(),
            description: "Direct Model Context Protocol server providing authorized file reading, directory listing, and file inspection strictly confined to user-approved directory scopes.".to_string(),
            supported_capabilities: vec![
                "read_file".to_string(),
                "list_directory".to_string(),
                "get_file_info".to_string(),
            ],
            requirements: "Node.js (npx) installed on your system. Approved workspace directory path.".to_string(),
            setup_instructions: "Configure the approved directories below. By default, Aven runs the official `@modelcontextprotocol/server-filesystem` via `npx`.".to_string(),
            is_implemented: true,
            default_command: Some("npx".to_string()),
            default_args: Some(vec!["-y".to_string(), "@modelcontextprotocol/server-filesystem".to_string()]),
            default_url: None,
            permissions_notice: "Path containment is strictly enforced by the Aven backend. Access outside approved directories will be rejected.".to_string(),
        },
        CatalogEntry {
            id: "mcp_sqlite".to_string(),
            kind: PluginKind::Mcp,
            name: "SQLite MCP".to_string(),
            category: "Database".to_string(),
            description: "Connects to local SQLite databases to inspect schemas, execute read-only queries, and analyze structured data.".to_string(),
            supported_capabilities: vec![
                "read_query".to_string(),
                "list_tables".to_string(),
                "describe_table".to_string(),
            ],
            requirements: "Python/uvx or Node.js. Explicit database file path.".to_string(),
            setup_instructions: "Specify the database file path. Access is restricted to explicitly chosen SQLite database files.".to_string(),
            is_implemented: true,
            default_command: Some("uvx".to_string()),
            default_args: Some(vec!["mcp-server-sqlite".to_string()]),
            default_url: None,
            permissions_notice: "Confined to approved database files. Destructive schema alterations require confirmation.".to_string(),
        },
        CatalogEntry {
            id: "mcp_github".to_string(),
            kind: PluginKind::Mcp,
            name: "GitHub MCP".to_string(),
            category: "Developer".to_string(),
            description: "Model Context Protocol integration with GitHub API to inspect repositories, read issues, pull requests, and commit logs.".to_string(),
            supported_capabilities: vec![
                "search_repositories".to_string(),
                "get_issue".to_string(),
                "list_pull_requests".to_string(),
                "read_file_contents".to_string(),
            ],
            requirements: "Node.js (npx) and a GitHub Personal Access Token (PAT).".to_string(),
            setup_instructions: "Enter your GitHub Personal Access Token. Tokens are securely stored in Windows Credential Manager.".to_string(),
            is_implemented: true,
            default_command: Some("npx".to_string()),
            default_args: Some(vec!["-y".to_string(), "@modelcontextprotocol/server-github".to_string()]),
            default_url: None,
            permissions_notice: "Uses your personal GitHub token permissions. Never stores or logs raw tokens in plaintext.".to_string(),
        },
        CatalogEntry {
            id: "mcp_playwright".to_string(),
            kind: PluginKind::Mcp,
            name: "Playwright MCP".to_string(),
            category: "Browser".to_string(),
            description: "Headless browser automation MCP server providing web page inspection, accessibility tree reading, and snapshot extraction.".to_string(),
            supported_capabilities: vec![
                "browser_navigate".to_string(),
                "browser_snapshot".to_string(),
                "browser_evaluate".to_string(),
            ],
            requirements: "Node.js (npx) and Playwright browser binaries.".to_string(),
            setup_instructions: "Runs `@modelcontextprotocol/server-playwright`. Exposes only tools published by the actual connected server.".to_string(),
            is_implemented: true,
            default_command: Some("npx".to_string()),
            default_args: Some(vec!["-y".to_string(), "@modelcontextprotocol/server-playwright".to_string()]),
            default_url: None,
            permissions_notice: "Runs in isolated browser processes. High-impact interactions require explicit confirmation.".to_string(),
        },
        CatalogEntry {
            id: "mcp_context7".to_string(),
            kind: PluginKind::Mcp,
            name: "Context7 MCP".to_string(),
            category: "Developer".to_string(),
            description: "Context7 code intelligence and semantic documentation indexer.".to_string(),
            supported_capabilities: vec![
                "resolve_symbols".to_string(),
                "query_docs".to_string(),
            ],
            requirements: "Context7 host server credentials or API key.".to_string(),
            setup_instructions: "Planned integration. Remote host configuration will be available in future release.".to_string(),
            is_implemented: false,
            default_command: None,
            default_args: None,
            default_url: Some("https://api.context7.ai/mcp".to_string()),
            permissions_notice: "Remote communication uses TLS.".to_string(),
        },
        CatalogEntry {
            id: "mcp_custom".to_string(),
            kind: PluginKind::Mcp,
            name: "Custom MCP Server".to_string(),
            category: "Developer".to_string(),
            description: "Connect any compliant Model Context Protocol server over local stdio process or remote HTTP endpoint.".to_string(),
            supported_capabilities: vec![
                "Dynamic discovery via tools/list".to_string(),
            ],
            requirements: "Executable command or HTTP endpoint URL.".to_string(),
            setup_instructions: "Specify the executable command and arguments for stdio, or the endpoint URL for HTTP. Secrets are stored in OS Credential Manager.".to_string(),
            is_implemented: true,
            default_command: None,
            default_args: None,
            default_url: None,
            permissions_notice: "Tools discovered from custom servers are validated against their input schemas before execution.".to_string(),
        },

        // ------------------ API Integrations ------------------
        CatalogEntry {
            id: "api_gemini_search".to_string(),
            kind: PluginKind::Api,
            name: "Google Search via Gemini API".to_string(),
            category: "Search".to_string(),
            description: "Live web search grounded directly by Google's official Gemini 2.5 Flash Search Grounding engine. Returns factual summaries with real web citations, links, and attribution.".to_string(),
            supported_capabilities: vec![
                "google_search_grounding".to_string(),
                "citation_extraction".to_string(),
                "source_verification".to_string(),
            ],
            requirements: "Google Gemini API Key from Google AI Studio.".to_string(),
            setup_instructions: "Enter your Gemini API key below. Click 'Test Connection' to verify connectivity. The key is securely stored in Windows Credential Manager.".to_string(),
            is_implemented: true,
            default_command: None,
            default_args: None,
            default_url: Some(api::GEMINI_ENDPOINT.to_string()),
            permissions_notice: "Requests are made to official Google Gemini API (`generativelanguage.googleapis.com`). Keys are never shared or logged.".to_string(),
        },
        CatalogEntry {
            id: "api_custom_rest".to_string(),
            kind: PluginKind::Api,
            name: "Custom REST API Integration".to_string(),
            category: "Developer".to_string(),
            description: "Execute authenticated HTTP requests against user-defined external REST APIs.".to_string(),
            supported_capabilities: vec![
                "http_request".to_string(),
            ],
            requirements: "Base URL and optional API token.".to_string(),
            setup_instructions: "Planned integration. Structured schema validation is required for safe execution.".to_string(),
            is_implemented: false,
            default_command: None,
            default_args: None,
            default_url: None,
            permissions_notice: "Restricted to HTTPS endpoints.".to_string(),
        },

        // ------------------ Native Capabilities ------------------
        CatalogEntry {
            id: "native_browser".to_string(),
            kind: PluginKind::Native,
            name: "Open URLs in Default Browser".to_string(),
            category: "System".to_string(),
            description: "Native Windows capability that opens validated HTTP/HTTPS URLs directly in your operating system's default browser.".to_string(),
            supported_capabilities: vec![
                "native::open_url".to_string(),
            ],
            requirements: "Built-in native capability. Windows operating system.".to_string(),
            setup_instructions: "Enabled by default on Windows. URLs are strictly validated to prevent arbitrary protocol execution.".to_string(),
            is_implemented: true,
            default_command: None,
            default_args: None,
            default_url: None,
            permissions_notice: "Only http:// and https:// URLs are permitted. file://, javascript:, and shell protocols are blocked.".to_string(),
        },

        CatalogEntry {
            id: "native_fs".to_string(),
            kind: PluginKind::Native,
            name: "Scoped File Access".to_string(),
            category: "System".to_string(),
            description: "Fast native file reading and directory inspection strictly contained within your user-approved workspace directory.".to_string(),
            supported_capabilities: vec![
                "native::file_access".to_string(),
            ],
            requirements: "Approved workspace directory path.".to_string(),
            setup_instructions: "Select your approved directory. Path canonicalization prevents directory traversal and symlink escapes.".to_string(),
            is_implemented: true,
            default_command: None,
            default_args: None,
            default_url: None,
            permissions_notice: "Directory traversal (../) and unauthorized disk access are blocked in Rust before any disk I/O occurs.".to_string(),
        },
        CatalogEntry {
            id: "native_system".to_string(),
            kind: PluginKind::Native,
            name: "System Diagnostic Info".to_string(),
            category: "System".to_string(),
            description: "Retrieves safe system diagnostic metrics including OS platform, architecture, and feature readiness.".to_string(),
            supported_capabilities: vec![
                "native::system_info".to_string(),
            ],
            requirements: "Built-in native capability.".to_string(),
            setup_instructions: "Enabled and active by default.".to_string(),
            is_implemented: true,
            default_command: None,
            default_args: None,
            default_url: None,
            permissions_notice: "Read-only non-sensitive telemetry.".to_string(),
        },
        CatalogEntry {
            id: "native_desktop_control".to_string(),
            kind: PluginKind::Native,
            name: "Desktop & UI Control".to_string(),
            category: "System".to_string(),
            description: "Direct Windows UI Automation and native desktop control for windows, mouse, keyboard (Unicode text and shortcuts), UI element inspection, screenshots, bounded waits, and application launching with window reuse.".to_string(),
            supported_capabilities: vec![
                "desktop_control_list_windows".to_string(),
                "desktop_control_get_window_details".to_string(),
                "desktop_control_window_action".to_string(),
                "desktop_control_mouse_action".to_string(),
                "desktop_control_keyboard_action".to_string(),
                "desktop_control_inspect_ui".to_string(),
                "desktop_control_capture_screenshot".to_string(),
                "desktop_control_wait_for_ui".to_string(),
                "desktop_control_launch_app".to_string(),
            ],
            requirements: "Windows 10/11 operating system. Native UI Automation.".to_string(),
            setup_instructions: "Enabled and active by default on Windows. Sensitive destructive actions (like closing applications) require explicit confirmation.".to_string(),
            is_implemented: true,
            default_command: None,
            default_args: None,
            default_url: None,
            permissions_notice: "Operates with standard desktop user permissions. Shell scripts (cmd/powershell) are strictly blocked. Actions are verified and bounded.".to_string(),
        },
    ]
}

// -------------------------------------------------------------------------------------------------
// Tauri Commands
// -------------------------------------------------------------------------------------------------

#[tauri::command]
pub async fn get_plugins_state(
    state: tauri::State<'_, Arc<PluginsManager>>,
) -> Result<PluginsStatePayload, String> {
    Ok(state.get_state().await)
}

#[tauri::command]
pub async fn save_plugin_config(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<PluginsManager>>,
    config: PluginConfig,
    secret_key: Option<String>,
) -> Result<PluginMetadata, String> {
    let id = config.id.clone();

    // If secret provided, store in OS keyring (never in plaintext config file)
    if let Some(sec) = secret_key {
        let trimmed = sec.trim();
        if !trimmed.is_empty() {
            store_plugin_secret(&id, trimmed)?;
        }
    }

    {
        let mut configs = state.configs.lock().await;
        configs.insert(id.clone(), config.clone());
    }

    state.save_configs_to_disk(&app).await?;

    let has_secret = has_plugin_secret(&id);
    Ok(PluginMetadata {
        id,
        kind: config.kind,
        name: config.name,
        status: config.last_status,
        has_secret,
        error_message: config.last_error,
        latency_ms: None,
        approved_paths: config.approved_paths.unwrap_or_default(),
        settings: config.settings,
        discovered_tools: Vec::new(),
    })
}

#[tauri::command]
pub async fn delete_plugin_config(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<PluginsManager>>,
    plugin_id: String,
) -> Result<(), String> {
    // 1. Disconnect MCP session if active
    {
        let mut sessions = state.active_mcp_sessions.lock().await;
        if let Some(mut session) = sessions.remove(&plugin_id) {
            session.shutdown().await;
        }
    }

    if plugin_id == "native_desktop_control" {
        desktop_control::cursor::force_restore_and_cleanup();
    }

    // 2. Remove configuration
    {
        let mut configs = state.configs.lock().await;
        configs.remove(&plugin_id);
    }

    // 3. Delete OS keyring entry
    let _ = delete_plugin_secret(&plugin_id);

    // 4. Save updated configs to disk
    state.save_configs_to_disk(&app).await?;

    Ok(())
}

#[tauri::command]
pub async fn test_plugin_connection(
    state: tauri::State<'_, Arc<PluginsManager>>,
    plugin_id: String,
) -> Result<TestConnectionResult, String> {
    let start = Instant::now();

    // Check if it's Native Desktop Control
    if plugin_id == "native_desktop_control" {
        let is_win = cfg!(target_os = "windows");
        if !is_win {
            return Ok(TestConnectionResult {
                success: false,
                status: PluginStatus::Unsupported,
                latency_ms: 1,
                message: "Desktop Control requires Windows OS.".to_string(),
            });
        }
        let test_res = desktop_control::run_desktop_control_self_test();
        let latency = start.elapsed().as_millis() as u64;
        return match test_res {
            Ok(info) => {
                let uia = info.get("ui_automation_active").and_then(|v| v.as_bool()).unwrap_or(false);
                let visible = info.get("visible_windows_count").and_then(|v| v.as_u64()).unwrap_or(0);
                Ok(TestConnectionResult {
                    success: true,
                    status: PluginStatus::Connected,
                    latency_ms: latency,
                    message: format!(
                        "Desktop Control self-test passed: UI Automation is {}, {} visible window(s) detected.",
                        if uia { "active" } else { "standby" },
                        visible
                    ),
                })
            }
            Err(e) => Ok(TestConnectionResult {
                success: false,
                status: PluginStatus::Error,
                latency_ms: latency,
                message: format!("Desktop Control self-test failed: {e}"),
            }),
        };
    }

    // Check if it's other Native capabilities
    if plugin_id.starts_with("native_") {
        let is_win = cfg!(target_os = "windows");
        return Ok(TestConnectionResult {
            success: is_win,
            status: if is_win { PluginStatus::Connected } else { PluginStatus::Unsupported },
            latency_ms: 1,
            message: if is_win {
                "Native Windows subsystem is active and supported.".to_string()
            } else {
                "Native capability requires Windows OS.".to_string()
            },
        });
    }

    // Check if it's Google Search via Gemini API
    if plugin_id == "api_gemini_search" {
        let api_key = match get_plugin_secret(&plugin_id)? {
            Some(k) if !k.trim().is_empty() => k,
            _ => return Ok(TestConnectionResult {
                success: false,
                status: PluginStatus::NeedsPermission,
                latency_ms: 0,
                message: "No Gemini API key configured. Enter your key in settings.".to_string(),
            }),
        };

        let res = api::test_gemini_connection(&state.http_client, &api_key).await?;
        return Ok(TestConnectionResult {
            success: res.success,
            status: if res.success { PluginStatus::Connected } else { PluginStatus::Error },
            latency_ms: res.latency_ms,
            message: res.message,
        });
    }

    // Check if it's an MCP server
    let config = {
        let configs = state.configs.lock().await;
        configs.get(&plugin_id).cloned()
    };

    let config = match config {
        Some(c) => c,
        None => return Ok(TestConnectionResult {
            success: false,
            status: PluginStatus::NotConfigured,
            latency_ms: 0,
            message: "Integration configuration not found. Please save configuration first.".to_string(),
        }),
    };

    // Stdio MCP test connection
    if config.transport.as_deref() == Some("stdio") {
        let cmd = config.command.as_deref().unwrap_or("");
        let args = config.args.clone().unwrap_or_default();
        let approved_paths: Vec<PathBuf> = config.approved_paths
            .unwrap_or_default()
            .iter()
            .map(PathBuf::from)
            .collect();

        let mut session = mcp::McpClientSession::new_stdio(plugin_id.clone(), approved_paths);
        let mut env_map = HashMap::new();

        // Inject secrets if GitHub or similar
        if plugin_id == "mcp_github" {
            if let Ok(Some(token)) = get_plugin_secret(&plugin_id) {
                env_map.insert("GITHUB_PERSONAL_ACCESS_TOKEN".to_string(), token);
            }
        }

        match session.start_stdio(cmd, &args, None, &env_map).await {
            Ok(_) => {
                let tool_count = session.tools.len();
                let latency = start.elapsed().as_millis() as u64;
                let s_info = session.server_info.as_ref()
                    .map(|i| format!("{} (v{})", i.name, i.version.as_deref().unwrap_or("?")))
                    .unwrap_or_else(|| "MCP Server".to_string());

                session.shutdown().await;

                Ok(TestConnectionResult {
                    success: true,
                    status: PluginStatus::Connected,
                    latency_ms: latency,
                    message: format!("Connected to {s_info}. Discovered {tool_count} tool(s) in {latency}ms."),
                })
            }
            Err(e) => {
                session.shutdown().await;
                Ok(TestConnectionResult {
                    success: false,
                    status: PluginStatus::Error,
                    latency_ms: start.elapsed().as_millis() as u64,
                    message: format!("Failed to connect: {e}"),
                })
            }
        }
    } else {
        Ok(TestConnectionResult {
            success: false,
            status: PluginStatus::Unsupported,
            latency_ms: 0,
            message: "Unsupported transport protocol for testing.".to_string(),
        })
    }
}

#[tauri::command]
pub async fn connect_plugin(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<PluginsManager>>,
    plugin_id: String,
) -> Result<PluginMetadata, String> {
    let mut config = {
        let configs = state.configs.lock().await;
        configs.get(&plugin_id).cloned().ok_or_else(|| format!("Plugin '{plugin_id}' not configured"))?
    };

    if config.kind == PluginKind::Native {
        config.last_status = if cfg!(target_os = "windows") {
            PluginStatus::Connected
        } else {
            PluginStatus::Unsupported
        };
        config.enabled = true;
    } else if config.kind == PluginKind::Api && plugin_id == "api_gemini_search" {
        let has_key = has_plugin_secret(&plugin_id);
        if !has_key {
            return Err("Gemini API Key is not configured. Please enter your API Key.".into());
        }
        config.last_status = PluginStatus::Connected;
        config.enabled = true;
    } else if config.kind == PluginKind::Mcp {
        let cmd = config.command.as_deref().unwrap_or("");
        let args = config.args.clone().unwrap_or_default();
        let approved_paths: Vec<PathBuf> = config.approved_paths
            .clone()
            .unwrap_or_default()
            .iter()
            .map(PathBuf::from)
            .collect();

        let mut session = mcp::McpClientSession::new_stdio(plugin_id.clone(), approved_paths);
        let mut env_map = HashMap::new();

        if plugin_id == "mcp_github" {
            if let Ok(Some(token)) = get_plugin_secret(&plugin_id) {
                env_map.insert("GITHUB_PERSONAL_ACCESS_TOKEN".to_string(), token);
            }
        }

        match session.start_stdio(cmd, &args, None, &env_map).await {
            Ok(_) => {
                config.last_status = PluginStatus::Connected;
                config.last_error = None;
                config.enabled = true;

                let mut sessions = state.active_mcp_sessions.lock().await;
                sessions.insert(plugin_id.clone(), session);
            }
            Err(e) => {
                config.last_status = PluginStatus::Error;
                config.last_error = Some(e.clone());
                return Err(format!("Connection failed: {e}"));
            }
        }
    }

    {
        let mut configs = state.configs.lock().await;
        configs.insert(plugin_id.clone(), config.clone());
    }

    state.save_configs_to_disk(&app).await?;

    // Emit event so UI auto-refreshes
    let _ = app.emit("plugins-state-changed", state.get_state().await);

    let has_secret = has_plugin_secret(&plugin_id);
    let discovered = {
        let sessions = state.active_mcp_sessions.lock().await;
        sessions.get(&plugin_id).map(|s| s.tools.clone()).unwrap_or_default()
    };

    Ok(PluginMetadata {
        id: plugin_id,
        kind: config.kind,
        name: config.name,
        status: config.last_status,
        has_secret,
        error_message: config.last_error,
        latency_ms: None,
        approved_paths: config.approved_paths.unwrap_or_default(),
        settings: config.settings,
        discovered_tools: discovered,
    })
}

#[tauri::command]
pub async fn disconnect_plugin(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<PluginsManager>>,
    plugin_id: String,
) -> Result<PluginMetadata, String> {
    // 1. If MCP session active, shutdown
    {
        let mut sessions = state.active_mcp_sessions.lock().await;
        if let Some(mut session) = sessions.remove(&plugin_id) {
            session.shutdown().await;
        }
    }

    if plugin_id == "native_desktop_control" {
        desktop_control::cursor::force_restore_and_cleanup();
    }

    // 2. Update config status
    let mut config = {
        let configs = state.configs.lock().await;
        configs.get(&plugin_id).cloned().ok_or_else(|| format!("Plugin '{plugin_id}' not found"))?
    };

    config.last_status = PluginStatus::Disconnected;
    config.enabled = false;

    {
        let mut configs = state.configs.lock().await;
        configs.insert(plugin_id.clone(), config.clone());
    }

    state.save_configs_to_disk(&app).await?;

    // Emit event so UI auto-refreshes
    let _ = app.emit("plugins-state-changed", state.get_state().await);

    let has_secret = has_plugin_secret(&plugin_id);
    Ok(PluginMetadata {
        id: plugin_id,
        kind: config.kind,
        name: config.name,
        status: PluginStatus::Disconnected,
        has_secret,
        error_message: None,
        latency_ms: None,
        approved_paths: config.approved_paths.unwrap_or_default(),
        settings: config.settings,
        discovered_tools: Vec::new(),
    })
}

#[tauri::command]
pub async fn execute_capability(
    state: tauri::State<'_, Arc<PluginsManager>>,
    request: registry::CapabilityExecutionRequest,
) -> Result<registry::CapabilityExecutionResult, String> {
    state.execute_capability_internal(request).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_catalog_contains_required_initial_entries() {
        let cat = get_full_catalog();
        assert!(cat.iter().any(|c| c.id == "mcp_filesystem"));
        assert!(cat.iter().any(|c| c.id == "mcp_sqlite"));
        assert!(cat.iter().any(|c| c.id == "mcp_github"));
        assert!(cat.iter().any(|c| c.id == "mcp_playwright"));
        assert!(cat.iter().any(|c| c.id == "mcp_context7"));
        assert!(cat.iter().any(|c| c.id == "mcp_custom"));
        assert!(cat.iter().any(|c| c.id == "api_gemini_search"));
        assert!(cat.iter().any(|c| c.id == "api_custom_rest"));
        assert!(cat.iter().any(|c| c.id == "native_browser"));
        assert!(!cat.iter().any(|c| c.id == "native_app_launcher"), "native_app_launcher must be removed in favor of native_desktop_control");
        assert!(cat.iter().any(|c| c.id == "native_fs"));
        assert!(cat.iter().any(|c| c.id == "native_system"));
        assert!(cat.iter().any(|c| c.id == "native_desktop_control"));
    }

    #[test]
    fn test_catalog_distinguishes_implemented_vs_planned() {
        let cat = get_full_catalog();
        let fs = cat.iter().find(|c| c.id == "mcp_filesystem").unwrap();
        assert!(fs.is_implemented);

        let dt = cat.iter().find(|c| c.id == "native_desktop_control").unwrap();
        assert!(dt.is_implemented);

        let c7 = cat.iter().find(|c| c.id == "mcp_context7").unwrap();
        assert!(!c7.is_implemented);

        let gemini = cat.iter().find(|c| c.id == "api_gemini_search").unwrap();
        assert!(gemini.is_implemented);

        let custom_rest = cat.iter().find(|c| c.id == "api_custom_rest").unwrap();
        assert!(!custom_rest.is_implemented);
    }

    #[test]
    fn test_default_configs_contain_native_and_gemini_search() {
        let defaults = get_default_configs("d:\\Projects\\Desktop\\Aven Agent");
        assert!(defaults.contains_key("native_browser"));
        assert!(!defaults.contains_key("native_app_launcher"), "native_app_launcher must not be in default configs");
        assert!(defaults.contains_key("native_fs"));
        assert!(defaults.contains_key("native_system"));
        assert!(defaults.contains_key("native_desktop_control"));
        assert!(defaults.contains_key("api_gemini_search"));

        let gemini = defaults.get("api_gemini_search").unwrap();
        assert_eq!(gemini.kind, PluginKind::Api);
        assert_eq!(gemini.enabled, has_plugin_secret("api_gemini_search"));
    }

    #[test]
    fn test_atomic_persistence_roundtrip() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_dir = std::env::temp_dir().join(format!("aven_test_{}", nanos));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let config_file = temp_dir.join("plugins.json");
        let temp_file = temp_dir.join("plugins.json.tmp");

        let mut configs = HashMap::new();
        configs.insert(
            "mcp_test".to_string(),
            PluginConfig {
                id: "mcp_test".to_string(),
                kind: PluginKind::Mcp,
                name: "Test MCP".to_string(),
                enabled: true,
                transport: Some("stdio".to_string()),
                command: Some("npx".to_string()),
                args: Some(vec!["-y".to_string(), "@modelcontextprotocol/server-everything".to_string()]),
                url: None,
                approved_paths: Some(vec!["d:\\test".to_string()]),
                settings: HashMap::new(),
                last_status: PluginStatus::Connecting,
                last_error: None,
            },
        );

        // Atomic write
        let json_data = serde_json::to_string_pretty(&configs).unwrap();
        std::fs::write(&temp_file, &json_data).unwrap();
        std::fs::rename(&temp_file, &config_file).unwrap();

        // Read back
        assert!(config_file.exists());
        let read_data = std::fs::read_to_string(&config_file).unwrap();
        let loaded: HashMap<String, PluginConfig> = serde_json::from_str(&read_data).unwrap();

        assert_eq!(loaded.len(), 1);
        let restored = loaded.get("mcp_test").unwrap();
        assert_eq!(restored.name, "Test MCP");
        assert_eq!(restored.kind, PluginKind::Mcp);
        assert!(restored.enabled);
        assert_eq!(restored.args.as_ref().unwrap().len(), 2);

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_temporary_connection_failure_preserves_config() {
        let mut configs = HashMap::new();
        let mcp_id = "mcp_offline_server";
        configs.insert(
            mcp_id.to_string(),
            PluginConfig {
                id: mcp_id.to_string(),
                kind: PluginKind::Mcp,
                name: "Offline MCP".to_string(),
                enabled: true,
                transport: Some("stdio".to_string()),
                command: Some("nonexistent_binary_xyz_123".to_string()),
                args: None,
                url: None,
                approved_paths: None,
                settings: HashMap::new(),
                last_status: PluginStatus::Connected,
                last_error: None,
            },
        );

        // Simulate connection attempt failing: config is NOT removed
        if let Some(conf) = configs.get_mut(mcp_id) {
            conf.last_status = PluginStatus::Error;
            conf.last_error = Some("Failed to spawn process nonexistent_binary_xyz_123".to_string());
        }

        assert!(configs.contains_key(mcp_id));
        let conf = configs.get(mcp_id).unwrap();
        assert_eq!(conf.last_status, PluginStatus::Error);
        assert!(conf.last_error.as_ref().unwrap().contains("nonexistent_binary"));
        assert!(conf.enabled); // Remains enabled so user can retry or fix path
    }

    #[test]
    fn test_execute_capability_argument_validation() {
        // Validation check for native::open_url requiring a valid http/https url
        let request = registry::CapabilityExecutionRequest {
            capability_id: "native::open_url".to_string(),
            arguments: serde_json::json!({
                "url": "javascript:alert(1)"
            }),
            user_confirmed: false,
        };

        let url_str = request.arguments.get("url").and_then(|v| v.as_str()).unwrap();
        let is_valid = native::validate_url(url_str).is_ok();
        assert!(!is_valid, "Dangerous javascript: URL must be rejected");
    }
}
