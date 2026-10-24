use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CapabilityKind {
    Mcp,
    Api,
    Native,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PermissionLevel {
    Read,
    Write,
    Execute,
    Network,
    Full,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredCapability {
    pub id: String,
    pub source_id: String,
    pub kind: CapabilityKind,
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub required_permission: PermissionLevel,
    pub requires_confirmation: bool,
    pub is_available: bool,
}

/// Execution request payload passed from frontend or agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityExecutionRequest {
    pub capability_id: String,
    pub arguments: serde_json::Value,
    pub user_confirmed: bool,
}

/// Execution response payload returned to caller
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityExecutionResult {
    pub success: bool,
    pub capability_id: String,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
}

/// Get native capabilities registered by default
pub fn get_default_native_capabilities(workspace_path: &str) -> Vec<RegisteredCapability> {
    let mut caps = vec![
        RegisteredCapability {
            id: "native::open_url".to_string(),
            source_id: "native_browser".to_string(),
            kind: CapabilityKind::Native,
            name: "Open URL in Browser".to_string(),
            description: "Opens validated HTTP or HTTPS links in the default system web browser safely.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "Web URL starting with http:// or https://" }
                },
                "required": ["url"]
            }),
            required_permission: PermissionLevel::Network,
            requires_confirmation: false,
            is_available: true,
        },

        RegisteredCapability {
            id: "native::file_access".to_string(),
            source_id: "native_fs".to_string(),
            kind: CapabilityKind::Native,
            name: "Scoped File Access".to_string(),
            description: format!("Safely reads or lists files strictly contained within the user-approved directory scope ('{}').", workspace_path),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": { "type": "string", "enum": ["read", "list"], "description": "File operation: 'read' or 'list'" },
                    "path": { "type": "string", "description": "Relative path within the approved workspace" }
                },
                "required": ["action", "path"]
            }),
            required_permission: PermissionLevel::Read,
            requires_confirmation: false,
            is_available: true,
        },
        RegisteredCapability {
            id: "native::system_info".to_string(),
            source_id: "native_system".to_string(),
            kind: CapabilityKind::Native,
            name: "System Diagnostic Info".to_string(),
            description: "Retrieves safe system diagnostic metrics including device name (computer name), logged-in user name, operating system version, and system architecture.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
            required_permission: PermissionLevel::Read,
            requires_confirmation: false,
            is_available: true,
        },
    ];

    #[cfg(target_os = "windows")]
    {
        let desktop_caps = crate::plugins::desktop_control::get_desktop_control_capabilities();
        caps.extend(desktop_caps);
    }

    caps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_native_capabilities_list() {
        let caps = get_default_native_capabilities("d:\\Projects\\Desktop\\Aven Agent");
        assert!(caps.iter().any(|c| c.id == "native::open_url"));
        assert!(!caps.iter().any(|c| c.id == "native::launch_app"), "native::launch_app duplicate must be removed");
        assert!(caps.iter().any(|c| c.id == "desktop_control_launch_app"));
        assert!(caps.iter().any(|c| c.id == "native::file_access"));
        assert!(caps.iter().any(|c| c.id == "native::system_info"));
        assert!(caps.iter().any(|c| c.id == "desktop_control_list_windows"));
        assert!(caps.iter().any(|c| c.id == "desktop_control_mouse_action"));
        assert!(caps.iter().any(|c| c.id == "desktop_control_keyboard_action"));
        assert!(caps.iter().any(|c| c.id == "desktop_control_inspect_ui"));
        assert!(caps.iter().any(|c| c.id == "desktop_control_capture_screenshot"));
        assert!(caps.iter().any(|c| c.id == "desktop_control_get_screen_info"));
    }
}
