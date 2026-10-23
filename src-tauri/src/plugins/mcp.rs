use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{oneshot, Mutex};

/// Protocol version supported
pub const MCP_PROTOCOL_VERSION: &str = "2024-11-05";
pub const DEFAULT_TIMEOUT_SECS: u64 = 15;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    pub jsonrpc: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerInfo {
    pub name: String,
    pub version: Option<String>,
}

/// Active MCP Client Session managing a running local child process or remote HTTP endpoint
pub struct McpClientSession {
    pub server_id: String,
    pub server_info: Option<McpServerInfo>,
    pub tools: Vec<McpTool>,
    pub approved_paths: Vec<PathBuf>,
    pub is_connected: bool,
    next_id: AtomicU64,
    stdin_tx: Option<tokio::sync::mpsc::Sender<String>>,
    pending_requests: Arc<Mutex<HashMap<u64, oneshot::Sender<Result<serde_json::Value, String>>>>>,
    child_handle: Option<tokio::process::Child>,
    remote_url: Option<String>,
    auth_header: Option<String>,
    http_client: reqwest::Client,
}

impl McpClientSession {
    /// Create a new session configured for stdio local process
    pub fn new_stdio(
        server_id: String,
        approved_paths: Vec<PathBuf>,
    ) -> Self {
        Self {
            server_id,
            server_info: None,
            tools: Vec::new(),
            approved_paths,
            is_connected: false,
            next_id: AtomicU64::new(1),
            stdin_tx: None,
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
            child_handle: None,
            remote_url: None,
            auth_header: None,
            http_client: reqwest::Client::new(),
        }
    }

    /// Create a new session configured for remote HTTP / SSE
    pub fn new_remote(
        server_id: String,
        url: String,
        auth_header: Option<String>,
        approved_paths: Vec<PathBuf>,
    ) -> Self {
        Self {
            server_id,
            server_info: None,
            tools: Vec::new(),
            approved_paths,
            is_connected: false,
            next_id: AtomicU64::new(1),
            stdin_tx: None,
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
            child_handle: None,
            remote_url: Some(url),
            auth_header,
            http_client: reqwest::Client::new(),
        }
    }

    /// Start and perform protocol handshake over stdio
    pub async fn start_stdio(
        &mut self,
        command: &str,
        args: &[String],
        cwd: Option<&Path>,
        env: &HashMap<String, String>,
    ) -> Result<(), String> {
        if command.trim().is_empty() {
            return Err("MCP executable command cannot be empty".into());
        }

        // On Windows, resolve npx/npm/python aliases safely
        let mut cmd = tokio::process::Command::new(command);
        cmd.args(args);

        if let Some(dir) = cwd {
            if dir.exists() {
                cmd.current_dir(dir);
            }
        }

        for (k, v) in env {
            cmd.env(k, v);
        }

        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let mut child = cmd.spawn()
            .map_err(|e| format!("Failed to spawn MCP server process '{}': {}", command, e))?;

        let stdin = child.stdin.take().ok_or("Failed to open child process stdin")?;
        let stdout = child.stdout.take().ok_or("Failed to open child process stdout")?;
        let stderr = child.stderr.take();

        // Spawn async reader for stderr
        if let Some(err) = stderr {
            let s_id = self.server_id.clone();
            tokio::spawn(async move {
                let mut reader = BufReader::new(err).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    eprintln!("[MCP:{s_id}:stderr] {line}");
                }
            });
        }

        // Channel for writing JSON-RPC lines to child stdin
        let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(32);
        self.stdin_tx = Some(tx);

        tokio::spawn(async move {
            let mut writer = stdin;
            while let Some(msg) = rx.recv().await {
                if writer.write_all(msg.as_bytes()).await.is_err() {
                    break;
                }
                if writer.write_all(b"\n").await.is_err() {
                    break;
                }
                if writer.flush().await.is_err() {
                    break;
                }
            }
        });

        // Spawn async reader for stdout responses
        let pending = Arc::clone(&self.pending_requests);
        let s_id = self.server_id.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                if let Ok(resp) = serde_json::from_str::<JsonRpcResponse>(trimmed) {
                    if let Some(id) = resp.id {
                        let mut map = pending.lock().await;
                        if let Some(sender) = map.remove(&id) {
                            if let Some(err) = resp.error {
                                let _ = sender.send(Err(format!("MCP Error {}: {}", err.code, err.message)));
                            } else {
                                let _ = sender.send(Ok(resp.result.unwrap_or(serde_json::Value::Null)));
                            }
                        }
                    }
                } else {
                    eprintln!("[MCP:{s_id}:unparsed_stdout] {trimmed}");
                }
            }
        });

        self.child_handle = Some(child);

        // Perform MCP Protocol Handshake
        self.perform_handshake().await?;

        // Discover Tools
        self.discover_tools().await?;

        self.is_connected = true;
        Ok(())
    }

    /// Perform handshake for remote HTTP server
    pub async fn start_remote(&mut self) -> Result<(), String> {
        if self.remote_url.is_none() {
            return Err("Missing remote MCP endpoint URL".into());
        }

        self.perform_handshake().await?;
        self.discover_tools().await?;
        self.is_connected = true;
        Ok(())
    }

    /// Send a JSON-RPC request and wait for correlated response with timeout
    pub async fn send_request(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, String> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.to_string(),
            params,
        };

        let req_json = serde_json::to_string(&req)
            .map_err(|e| format!("Failed to serialize JSON-RPC request: {e}"))?;

        // If local stdio
        if let Some(tx) = &self.stdin_tx {
            let (resp_tx, resp_rx) = oneshot::channel();
            {
                let mut map = self.pending_requests.lock().await;
                map.insert(id, resp_tx);
            }

            tx.send(req_json)
                .await
                .map_err(|e| format!("Failed to send request to MCP child process: {e}"))?;

            match tokio::time::timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS), resp_rx).await {
                Ok(Ok(res)) => res,
                Ok(Err(_)) => Err("MCP server closed response channel unexpectedly".into()),
                Err(_) => {
                    let mut map = self.pending_requests.lock().await;
                    map.remove(&id);
                    Err(format!("MCP request '{method}' timed out after {DEFAULT_TIMEOUT_SECS}s"))
                }
            }
        } else if let Some(url) = &self.remote_url {
            // Remote HTTP request
            let mut builder = self.http_client
                .post(url)
                .header("Content-Type", "application/json");

            if let Some(auth) = &self.auth_header {
                builder = builder.header("Authorization", auth);
            }

            let resp = builder
                .body(req_json)
                .send()
                .await
                .map_err(|e| format!("Remote MCP HTTP request failed: {e}"))?;

            if !resp.status().is_success() {
                return Err(format!("Remote MCP server returned HTTP status {}", resp.status()));
            }

            let rpc_resp: JsonRpcResponse = resp
                .json()
                .await
                .map_err(|e| format!("Failed to parse remote MCP JSON-RPC response: {e}"))?;

            if let Some(err) = rpc_resp.error {
                return Err(format!("MCP Error {}: {}", err.code, err.message));
            }

            Ok(rpc_resp.result.unwrap_or(serde_json::Value::Null))
        } else {
            Err("No active MCP transport channel".into())
        }
    }

    /// Send notification without expecting response
    pub async fn send_notification(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<(), String> {
        let notif = JsonRpcNotification {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
        };

        let json = serde_json::to_string(&notif)
            .map_err(|e| format!("Failed to serialize notification: {e}"))?;

        if let Some(tx) = &self.stdin_tx {
            tx.send(json)
                .await
                .map_err(|e| format!("Failed to send notification: {e}"))?;
            Ok(())
        } else if let Some(url) = &self.remote_url {
            let mut builder = self.http_client
                .post(url)
                .header("Content-Type", "application/json");

            if let Some(auth) = &self.auth_header {
                builder = builder.header("Authorization", auth);
            }

            let _ = builder.body(json).send().await;
            Ok(())
        } else {
            Err("No active MCP transport".into())
        }
    }

    /// Perform MCP handshake: initialize -> response -> notifications/initialized
    async fn perform_handshake(&mut self) -> Result<(), String> {
        let init_params = serde_json::json!({
            "protocolVersion": MCP_PROTOCOL_VERSION,
            "capabilities": {
                "roots": { "listChanged": true },
                "sampling": {}
            },
            "clientInfo": {
                "name": "Aven",
                "version": "0.1.0"
            }
        });

        let init_res = self.send_request("initialize", Some(init_params)).await?;

        if let Some(server_info) = init_res.get("serverInfo") {
            let name = server_info.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string();
            let version = server_info.get("version").and_then(|v| v.as_str()).map(|s| s.to_string());
            self.server_info = Some(McpServerInfo { name, version });
        }

        // Send required initialized notification
        let _ = self.send_notification("notifications/initialized", Some(serde_json::json!({}))).await;

        Ok(())
    }

    /// Discover available tools from MCP server with cursor pagination
    pub async fn discover_tools(&mut self) -> Result<Vec<McpTool>, String> {
        let mut all_tools = Vec::new();
        let mut cursor: Option<String> = None;
        let mut pages = 0;

        loop {
            pages += 1;
            if pages > 10 {
                break; // Safety limit on pagination
            }

            let mut params = serde_json::Map::new();
            if let Some(c) = &cursor {
                params.insert("cursor".to_string(), serde_json::Value::String(c.clone()));
            }

            let res = self.send_request("tools/list", Some(serde_json::Value::Object(params))).await?;

            if let Some(tools_arr) = res.get("tools").and_then(|v| v.as_array()) {
                for t in tools_arr {
                    if let Some(name) = t.get("name").and_then(|v| v.as_str()) {
                        let desc = t.get("description").and_then(|v| v.as_str()).map(|s| s.to_string());
                        let schema = t.get("inputSchema").cloned().unwrap_or(serde_json::json!({
                            "type": "object",
                            "properties": {}
                        }));

                        all_tools.push(McpTool {
                            name: name.to_string(),
                            description: desc,
                            input_schema: schema,
                        });
                    }
                }
            }

            cursor = res.get("nextCursor").and_then(|v| v.as_str()).map(|s| s.to_string());
            if cursor.is_none() {
                break;
            }
        }

        self.tools = all_tools.clone();
        Ok(all_tools)
    }

    /// Invoke an MCP tool with schema validation and strict containment enforcement
    pub async fn call_tool(
        &self,
        tool_name: &str,
        arguments: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        if !self.is_connected {
            return Err("MCP server is disconnected".into());
        }

        // Verify tool exists in discovered list
        let tool = self.tools.iter().find(|t| t.name == tool_name)
            .ok_or_else(|| format!("Tool '{tool_name}' not discovered on this MCP server"))?;

        // Validate arguments against schema
        validate_arguments_against_schema(&tool.input_schema, arguments)?;

        // Enforce Security Containment for Filesystem operations
        self.enforce_tool_security(tool_name, arguments)?;

        let call_params = serde_json::json!({
            "name": tool_name,
            "arguments": arguments
        });

        let res = self.send_request("tools/call", Some(call_params)).await?;

        // Extract content array or text
        if let Some(content) = res.get("content").and_then(|v| v.as_array()) {
            let mut texts = Vec::new();
            for item in content {
                if let Some(t) = item.get("text").and_then(|v| v.as_str()) {
                    texts.push(t.to_string());
                }
            }
            if !texts.is_empty() {
                return Ok(serde_json::json!({
                    "success": true,
                    "text": texts.join("\n"),
                    "raw": res
                }));
            }
        }

        Ok(serde_json::json!({
            "success": true,
            "raw": res
        }))
    }

    /// Enforce backend filesystem and sqlite boundary confinement
    fn enforce_tool_security(&self, tool_name: &str, arguments: &serde_json::Value) -> Result<(), String> {
        let lower = tool_name.to_lowercase();
        let is_fs_tool = lower.contains("file") || lower.contains("directory") || lower.contains("read") || lower.contains("write");

        if is_fs_tool && !self.approved_paths.is_empty() {
            // Check 'path' or 'filePath' argument
            let path_val = arguments.get("path")
                .or_else(|| arguments.get("filePath"))
                .or_else(|| arguments.get("directory"))
                .and_then(|v| v.as_str());

            if let Some(p_str) = path_val {
                let req_path = Path::new(p_str);
                let mut allowed = false;

                for approved in &self.approved_paths {
                    if let Ok(contained) = super::native::validate_path_containment(approved, req_path) {
                        if contained.starts_with(approved) {
                            allowed = true;
                            break;
                        }
                    }
                }

                if !allowed {
                    return Err(format!(
                        "Security violation: path '{}' is not within any approved workspace directory for MCP server '{}'.",
                        p_str, self.server_id
                    ));
                }
            }
        }

        Ok(())
    }

    /// Graceful shutdown
    pub async fn shutdown(&mut self) {
        self.is_connected = false;
        self.stdin_tx = None;

        if let Some(mut child) = self.child_handle.take() {
            let _ = child.kill().await;
        }

        let mut map = self.pending_requests.lock().await;
        for (_, sender) in map.drain() {
            let _ = sender.send(Err("MCP server shutdown".into()));
        }
    }
}

impl Drop for McpClientSession {
    fn drop(&mut self) {
        if let Some(mut child) = self.child_handle.take() {
            let _ = child.start_kill();
        }
    }
}

/// Validate arguments against JSON Schema required fields
pub fn validate_arguments_against_schema(
    schema: &serde_json::Value,
    arguments: &serde_json::Value,
) -> Result<(), String> {
    if !arguments.is_object() {
        return Err("Arguments must be a JSON object".into());
    }

    let args_obj = arguments.as_object().unwrap();

    // Check required fields
    if let Some(required) = schema.get("required").and_then(|v| v.as_array()) {
        for req_field in required {
            if let Some(field_name) = req_field.as_str() {
                if !args_obj.contains_key(field_name) {
                    return Err(format!("Missing required parameter: '{field_name}'"));
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_rpc_request_serialization() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 42,
            method: "tools/list".to_string(),
            params: Some(serde_json::json!({ "cursor": "abc" })),
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"id\":42"));
        assert!(json.contains("\"method\":\"tools/list\""));
    }

    #[test]
    fn test_json_rpc_response_deserialization() {
        let json_data = r#"{"jsonrpc":"2.0","id":1,"result":{"tools":[{"name":"test_tool","inputSchema":{}}]}}"#;
        let resp: JsonRpcResponse = serde_json::from_str(json_data).unwrap();
        assert_eq!(resp.id, Some(1));
        assert!(resp.error.is_none());
        assert!(resp.result.is_some());
    }

    #[test]
    fn test_argument_schema_validation_required_fields() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" },
                "limit": { "type": "integer" }
            },
            "required": ["path"]
        });

        // Valid
        let valid_args = serde_json::json!({ "path": "test.txt", "limit": 10 });
        assert!(validate_arguments_against_schema(&schema, &valid_args).is_ok());

        // Invalid: missing required 'path'
        let invalid_args = serde_json::json!({ "limit": 10 });
        assert!(validate_arguments_against_schema(&schema, &invalid_args).is_err());
    }
}
