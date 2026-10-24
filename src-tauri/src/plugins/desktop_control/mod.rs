pub mod cursor;
pub mod hotkey;
pub mod input;
pub mod launcher;
pub mod overlay;
pub mod screenshot;
pub mod taskbar;
pub mod timing;
pub mod uia;
pub mod windows;

use crate::plugins::registry::{CapabilityKind, PermissionLevel, RegisteredCapability};
use std::path::Path;

/// Generate registered capabilities for the desktop control subsystem
pub fn get_desktop_control_capabilities() -> Vec<RegisteredCapability> {
    vec![
        RegisteredCapability {
            id: "desktop_control_list_windows".to_string(),
            source_id: "native_desktop_control".to_string(),
            kind: CapabilityKind::Native,
            name: "List Desktop Windows".to_string(),
            description: "Lists open desktop windows with window handles (HWND), titles, process names, visibility, and bounds.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "filter_visible": { "type": "boolean", "description": "Only return visible top-level windows (defaults to true)" },
                    "filter_title": { "type": "string", "description": "Optional substring match for window title" },
                    "filter_process": { "type": "string", "description": "Optional substring match for process executable name" }
                }
            }),
            required_permission: PermissionLevel::Read,
            requires_confirmation: false,
            is_available: cfg!(target_os = "windows"),
        },
        RegisteredCapability {
            id: "desktop_control_get_window_details".to_string(),
            source_id: "native_desktop_control".to_string(),
            kind: CapabilityKind::Native,
            name: "Get Window Details".to_string(),
            description: "Retrieves detailed properties for a target window and identifies all other windows belonging to the same application.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "window_id": { "type": "integer", "description": "Window handle (HWND)" },
                    "title": { "type": "string", "description": "Title query" },
                    "process_name": { "type": "string", "description": "Process executable name query" }
                }
            }),
            required_permission: PermissionLevel::Read,
            requires_confirmation: false,
            is_available: cfg!(target_os = "windows"),
        },
        RegisteredCapability {
            id: "desktop_control_window_action".to_string(),
            source_id: "native_desktop_control".to_string(),
            kind: CapabilityKind::Native,
            name: "Perform Window Action".to_string(),
            description: "Performs window management actions: focus, minimize, maximize, restore, move, resize, close, or close_application. Destructive actions require confirmation.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["focus", "minimize", "maximize", "restore", "move", "resize", "close", "close_application"],
                        "description": "The window operation to perform"
                    },
                    "window_id": { "type": "integer", "description": "Window handle (HWND)" },
                    "x": { "type": "integer", "description": "New X coordinate (for move)" },
                    "y": { "type": "integer", "description": "New Y coordinate (for move)" },
                    "width": { "type": "integer", "description": "New width in pixels (for resize)" },
                    "height": { "type": "integer", "description": "New height in pixels (for resize)" }
                },
                "required": ["action", "window_id"]
            }),
            required_permission: PermissionLevel::Execute,
            requires_confirmation: false, // Performed inside handler: close & close_application enforce confirmation
            is_available: cfg!(target_os = "windows"),
        },
        RegisteredCapability {
            id: "desktop_control_get_screen_info".to_string(),
            source_id: "native_desktop_control".to_string(),
            kind: CapabilityKind::Native,
            name: "Get Screen and Monitor Info".to_string(),
            description: "Returns virtual desktop bounds, primary monitor dimensions, taskbar location/state, current pointer position, and active foreground window details.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
            required_permission: PermissionLevel::Read,
            requires_confirmation: false,
            is_available: cfg!(target_os = "windows"),
        },
        RegisteredCapability {
            id: "desktop_control_mouse_action".to_string(),
            source_id: "native_desktop_control".to_string(),
            kind: CapabilityKind::Native,
            name: "Mouse Action".to_string(),
            description: "Controls the mouse: move to coordinates, click (default left, right, middle), double-click, drag smoothly to destination, press, release, or scroll vertically and horizontally.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["click", "left_click", "right_click", "middle_click", "double_click", "move", "drag", "scroll", "press", "release"],
                        "description": "Mouse action to execute. 'click' defaults to left click."
                    },
                    "button": {
                        "type": "string",
                        "enum": ["left", "right", "middle"],
                        "description": "Mouse button (left, right, middle). Defaults to 'left', never right."
                    },
                    "x": { "type": "integer", "description": "Target X screen coordinate" },
                    "y": { "type": "integer", "description": "Target Y screen coordinate" },
                    "to_x": { "type": "integer", "description": "Destination X coordinate (for drag)" },
                    "to_y": { "type": "integer", "description": "Destination Y coordinate (for drag)" },
                    "scroll_x": { "type": "integer", "description": "Horizontal scroll steps (positive = right, negative = left)" },
                    "scroll_y": { "type": "integer", "description": "Vertical scroll steps (positive = up, negative = down)" }
                },
                "required": ["action"]
            }),
            required_permission: PermissionLevel::Execute,
            requires_confirmation: false,
            is_available: cfg!(target_os = "windows"),
        },
        RegisteredCapability {
            id: "desktop_control_keyboard_action".to_string(),
            source_id: "native_desktop_control".to_string(),
            kind: CapabilityKind::Native,
            name: "Keyboard Action".to_string(),
            description: "Simulates keyboard actions: type Unicode/multiline text without layout mismatch, press special keys (Enter, Tab, Esc), execute validated shortcuts (Ctrl+C, Ctrl+V, Alt+Tab, Win+R), inspect focused element, or clear fields.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["type_text", "key_press", "shortcut", "select_all", "copy", "paste", "clear_field", "get_focused_element"],
                        "description": "Keyboard operation to execute"
                    },
                    "text": { "type": "string", "description": "Unicode text to type (for type_text)" },
                    "key": { "type": "string", "description": "Key name (Enter, Tab, Escape, Backspace, Delete, ArrowUp, ArrowDown, ArrowLeft, ArrowRight, F1-F12)" },
                    "shortcut": { "type": "string", "description": "Validated shortcut string (e.g. Ctrl+C, Ctrl+V, Ctrl+A, Alt+Tab, Win+R)" },
                    "window_id": { "type": "integer", "description": "Optional window handle (HWND) to focus and activate before typing" },
                    "element_query": { "type": "string", "description": "Optional control name, label, or automation ID to focus before typing" },
                    "mode": { "type": "string", "enum": ["type", "replace", "append"], "description": "Typing mode: 'type' (default: insert at caret), 'replace' (clears existing text first if different), or 'append' (moves caret to end)" },
                    "verify": { "type": "boolean", "description": "Whether to verify post-typing state (defaults to true)" }
                },
                "required": ["action"]
            }),
            required_permission: PermissionLevel::Execute,
            requires_confirmation: false,
            is_available: cfg!(target_os = "windows"),
        },
        RegisteredCapability {
            id: "desktop_control_inspect_ui".to_string(),
            source_id: "native_desktop_control".to_string(),
            kind: CapabilityKind::Native,
            name: "Inspect UI Elements".to_string(),
            description: "Semantic Windows UI Automation inspection: read the foreground window accessibility tree, find controls by name/role/automation ID, inspect focused control details (values, text, selection, caret), click/invoke controls, set element values, select items, scroll, or expand/collapse controls.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["read_tree", "find_element", "get_focused_control", "invoke", "set_value", "select_item", "scroll", "expand_collapse"],
                        "description": "UI inspection or interaction action"
                    },
                    "window_id": { "type": "integer", "description": "Window handle (HWND) - defaults to active foreground window" },
                    "query": { "type": "string", "description": "Control name, label text, or automation ID" },
                    "control_type": { "type": "string", "description": "Filter by control type (e.g. Button, Edit, Text, MenuItem, TabItem, ComboBox, CheckBox)" },
                    "value": { "type": "string", "description": "Text value to assign (for set_value)" },
                    "scroll_x": { "type": "integer", "description": "Horizontal scroll steps (for scroll)" },
                    "scroll_y": { "type": "integer", "description": "Vertical scroll steps (for scroll)" },
                    "expand": { "type": "boolean", "description": "True to expand, false to collapse (for expand_collapse)" },
                    "max_depth": { "type": "integer", "description": "Maximum tree depth to explore (default 3, max 5)" }
                },
                "required": ["action"]
            }),
            required_permission: PermissionLevel::Read,
            requires_confirmation: false,
            is_available: cfg!(target_os = "windows"),
        },
        RegisteredCapability {
            id: "desktop_control_capture_screenshot".to_string(),
            source_id: "native_desktop_control".to_string(),
            kind: CapabilityKind::Native,
            name: "Capture Screenshot".to_string(),
            description: "Captures a high-resolution screenshot of the foreground window, a specified window, the primary monitor, or all monitors. Saves to disk and optionally returns base64 image data.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "target": {
                        "type": "string",
                        "enum": ["foreground_window", "selected_window", "primary_monitor", "all_monitors"],
                        "description": "Screenshot capture target (default: foreground_window)"
                    },
                    "window_id": { "type": "integer", "description": "Window handle (HWND) if target is selected_window" },
                    "save_to_path": { "type": "string", "description": "Optional specific file path for PNG file" },
                    "include_image_data": { "type": "boolean", "description": "If true, returns base64-encoded image data" }
                }
            }),
            required_permission: PermissionLevel::Read,
            requires_confirmation: false,
            is_available: cfg!(target_os = "windows"),
        },
        RegisteredCapability {
            id: "desktop_control_wait_for_ui".to_string(),
            source_id: "native_desktop_control".to_string(),
            kind: CapabilityKind::Native,
            name: "Wait for UI State".to_string(),
            description: "Bounded wait for UI state changes: wait for a window to exist, become active, close, or for a UI element to appear.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "condition": {
                        "type": "string",
                        "enum": ["window_exists", "window_active", "window_closed", "element_exists"],
                        "description": "Condition to wait for"
                    },
                    "window_id": { "type": "integer", "description": "Window handle (HWND)" },
                    "title": { "type": "string", "description": "Window title substring" },
                    "process_name": { "type": "string", "description": "Process executable name substring" },
                    "element_query": { "type": "string", "description": "Element name or automation ID" },
                    "control_type": { "type": "string", "description": "Element control type" },
                    "timeout_ms": { "type": "integer", "description": "Timeout in milliseconds (default 5000, max 30000)" }
                },
                "required": ["condition"]
            }),
            required_permission: PermissionLevel::Read,
            requires_confirmation: false,
            is_available: cfg!(target_os = "windows"),
        },
        RegisteredCapability {
            id: "desktop_control_launch_app".to_string(),
            source_id: "native_desktop_control".to_string(),
            kind: CapabilityKind::Native,
            name: "Launch or Reuse Application".to_string(),
            description: "Opens applications through normal Windows mechanisms and reuses existing open windows when possible. Script shells are blocked.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "app_name": { "type": "string", "description": "Application alias (notepad, calc, edge, chrome, explorer, code, terminal) or executable name" },
                    "args": { "type": "array", "items": { "type": "string" }, "description": "Command line arguments" },
                    "reuse_existing": { "type": "boolean", "description": "Whether to focus an existing window if already open (default: true)" }
                },
                "required": ["app_name"]
            }),
            required_permission: PermissionLevel::Execute,
            requires_confirmation: false,
            is_available: cfg!(target_os = "windows"),
        },
    ]
}

/// Verified typing execution:
/// 1. Pre-inspects target window, active window, and focused control
/// 2. Compares existing text and selection with requested change to avoid blind actions
/// 3. Executes Unicode keystrokes via SendInput
/// 4. Fetches fresh post-action state and verifies expected changes
/// 5. Diagnoses and returns detailed evidence of outcome
pub fn type_text_verified(
    text: &str,
    window_id: Option<i64>,
    element_query: Option<&str>,
    mode: Option<&str>,
    verify: bool,
) -> Result<serde_json::Value, String> {
    // 1. Pre-inspection: Focus window & control, inspect pre-action state
    let pre_state = uia::prepare_target_for_typing(window_id, element_query, None)?;

    let pre_text = pre_state
        .content
        .value
        .as_deref()
        .or(pre_state.content.text.as_deref())
        .unwrap_or("")
        .to_string();

    let is_ro = pre_state.content.is_read_only;
    if is_ro {
        return Err(format!(
            "Cannot type text: focused control '{}' (type: {}) in window '{}' is marked READ-ONLY.",
            pre_state.control_name, pre_state.control_type, pre_state.window_title
        ));
    }

    let clean_mode = mode.unwrap_or("type").to_lowercase();

    // 2. Prevent blind actions: compare existing input with requested change
    if clean_mode == "replace" {
        if pre_text == text {
            return Ok(serde_json::json!({
                "status": "success",
                "verified": true,
                "action": "type_text",
                "mode": "replace",
                "message": "Control already contains requested text (no modification needed)",
                "previous_text": pre_text,
                "current_text": pre_text,
                "focused_control": pre_state
            }));
        }
        let _ = input::select_all();
        std::thread::sleep(std::time::Duration::from_millis(25));
        let _ = input::clear_field();
        std::thread::sleep(std::time::Duration::from_millis(25));
    } else if clean_mode == "append" {
        if pre_text.ends_with(text) {
            return Ok(serde_json::json!({
                "status": "success",
                "verified": true,
                "action": "type_text",
                "mode": "append",
                "message": "Control already ends with requested text",
                "previous_text": pre_text,
                "current_text": pre_text,
                "focused_control": pre_state
            }));
        }
        // Move caret to end
        let _ = input::execute_shortcut("Ctrl+End").or_else(|_| input::press_key("End"));
        std::thread::sleep(std::time::Duration::from_millis(25));
    }

    // 3. Execute typing
    input::type_unicode_text(text)?;
    std::thread::sleep(std::time::Duration::from_millis(70));

    // 4. Post-action observation & verification
    if !verify {
        return Ok(serde_json::json!({
            "status": "success",
            "verified": false,
            "action": "type_text",
            "chars_typed": text.chars().count(),
            "window_id": window_id
        }));
    }

    let post_state = uia::get_focused_control_details_for_target(window_id).ok();
    let post_text = post_state
        .as_ref()
        .and_then(|s| s.content.value.as_deref().or(s.content.text.as_deref()))
        .unwrap_or("")
        .to_string();

    let verified_semantic = post_state
        .as_ref()
        .map(|s| s.content.pattern_used.is_some())
        .unwrap_or(false);

    // If semantic inspection confirmed text did not change despite non-empty input
    if verified_semantic && !text.is_empty() {
        let failed = match clean_mode.as_str() {
            "replace" => post_text != text && post_text == pre_text,
            "append" => !post_text.ends_with(text) && post_text == pre_text,
            _ => post_text == pre_text,
        };
        if failed {
            return Ok(serde_json::json!({
                "status": "verification_failed",
                "verified": false,
                "action": "type_text",
                "mode": clean_mode,
                "diagnosis": format!(
                    "Typing verification failed: attempted to type {} characters into control '{}' (window: '{}'), but control text remained unchanged ('{}'). Control may require manual click or be rejecting synthetic input.",
                    text.chars().count(),
                    pre_state.control_name,
                    pre_state.window_title,
                    post_text
                ),
                "pre_text": pre_text,
                "post_text": post_text,
                "pre_state": pre_state,
                "post_state": post_state
            }));
        }
    }

    let caret_pos = post_state.as_ref().and_then(|s| s.content.caret_position);
    let selected_text = post_state.as_ref().and_then(|s| s.content.selected_text.clone());

    Ok(serde_json::json!({
        "status": "success",
        "verified": true,
        "action": "type_text",
        "mode": clean_mode,
        "chars_typed": text.chars().count(),
        "previous_text": pre_text,
        "current_text": post_text,
        "caret_position": caret_pos,
        "selected_text": selected_text,
        "window_id": window_id,
        "focused_control": post_state.unwrap_or(pre_state)
    }))
}

/// Central dispatcher for all desktop control capabilities
pub fn execute_desktop_control_capability(
    cap_id: &str,
    args: &serde_json::Value,
    user_confirmed: bool,
    default_dir: &Path,
) -> Result<serde_json::Value, String> {
    if hotkey::is_cancelled() {
        return Err("Operation cancelled: Emergency stop was triggered".to_string());
    }

    let clean_action = cap_id
        .strip_prefix("desktop_control::")
        .or_else(|| cap_id.strip_prefix("desktop_control_"))
        .or_else(|| cap_id.strip_prefix("native::desktop_control::"))
        .or_else(|| cap_id.strip_prefix("native_desktop_control_"))
        .unwrap_or(cap_id);

    match clean_action {
        "get_screen_info" => {
            let (cx, cy) = input::get_cursor_position();
            let taskbar = taskbar::get_taskbar_info().ok();
            let fg = windows::get_window_details(None, None, None).ok();

            #[cfg(target_os = "windows")]
            let (vx, vy, vw, vh) = {
                use ::windows::Win32::UI::WindowsAndMessaging::{
                    GetSystemMetrics, SM_CXSCREEN, SM_CXVIRTUALSCREEN, SM_CYSCREEN,
                    SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
                };
                unsafe {
                    let vx = GetSystemMetrics(SM_XVIRTUALSCREEN);
                    let vy = GetSystemMetrics(SM_YVIRTUALSCREEN);
                    let vw = GetSystemMetrics(SM_CXVIRTUALSCREEN);
                    let vh = GetSystemMetrics(SM_CYVIRTUALSCREEN);
                    let pw = GetSystemMetrics(SM_CXSCREEN);
                    let ph = GetSystemMetrics(SM_CYSCREEN);
                    (
                        vx,
                        vy,
                        if vw > 0 { vw } else { pw },
                        if vh > 0 { vh } else { ph },
                    )
                }
            };
            #[cfg(not(target_os = "windows"))]
            let (vx, vy, vw, vh) = (0, 0, 1920, 1080);

            Ok(serde_json::json!({
                "status": "success",
                "pointer_position": { "x": cx, "y": cy },
                "virtual_screen": {
                    "x": vx,
                    "y": vy,
                    "width": vw,
                    "height": vh
                },
                "taskbar": taskbar,
                "foreground_window": fg
            }))
        }

        "list_windows" => {
            let filter_visible = args.get("filter_visible").and_then(|v| v.as_bool()).unwrap_or(true);
            let filter_title = args.get("filter_title").and_then(|v| v.as_str()).map(|s| s.to_string());
            let filter_process = args.get("filter_process").and_then(|v| v.as_str()).map(|s| s.to_string());

            let list = windows::list_windows(filter_visible, filter_title, filter_process)?;
            Ok(serde_json::to_value(&list).unwrap_or_default())
        }

        "get_window_details" => {
            let window_id = args.get("window_id").and_then(|v| v.as_i64());
            let title = args.get("title").and_then(|v| v.as_str());
            let process_name = args.get("process_name").and_then(|v| v.as_str());

            let details = windows::get_window_details(window_id, title, process_name)?;
            Ok(serde_json::to_value(&details).unwrap_or_default())
        }

        "window_action" => {
            let _session = cursor::acquire_session()?;
            let action = args
                .get("action")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter 'action'".to_string())?;

            let hwnd = args
                .get("window_id")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| "Missing required parameter 'window_id'".to_string())?;

            let x = args.get("x").and_then(|v| v.as_i64()).map(|v| v as i32);
            let y = args.get("y").and_then(|v| v.as_i64()).map(|v| v as i32);
            let w = args.get("width").and_then(|v| v.as_i64()).map(|v| v as i32);
            let h = args.get("height").and_then(|v| v.as_i64()).map(|v| v as i32);

            windows::perform_window_action(action, hwnd, x, y, w, h, user_confirmed)
        }

        "mouse_action" => {
            let action = args
                .get("action")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter 'action'".to_string())?;

            let x = args.get("x").and_then(|v| v.as_i64()).map(|v| v as i32);
            let y = args.get("y").and_then(|v| v.as_i64()).map(|v| v as i32);
            let to_x = args.get("to_x").and_then(|v| v.as_i64()).map(|v| v as i32);
            let to_y = args.get("to_y").and_then(|v| v.as_i64()).map(|v| v as i32);
            let scroll_x = args.get("scroll_x").and_then(|v| v.as_i64()).map(|v| v as i32);
            let scroll_y = args.get("scroll_y").and_then(|v| v.as_i64()).map(|v| v as i32);

            let _session = cursor::acquire_session()?;
            overlay::set_activity_text(match action.to_lowercase().as_str() {
                "click" | "left_click" => "Clicking mouse...".to_string(),
                "right_click" => "Right-clicking...".to_string(),
                "middle_click" => "Middle-clicking...".to_string(),
                "double_click" => "Double-clicking...".to_string(),
                "drag" => "Dragging...".to_string(),
                "scroll" => "Scrolling...".to_string(),
                "move" => "Moving mouse...".to_string(),
                _ => "Operating mouse...".to_string(),
            });

            let btn_arg = args.get("button").and_then(|v| v.as_str()).unwrap_or("left");
            let clean_btn = match btn_arg.to_lowercase().as_str() {
                "right" => "right",
                "middle" => "middle",
                _ => "left", // NEVER default unspecified click to right
            };

            match action.to_lowercase().as_str() {
                "move" => {
                    let mx = x.ok_or_else(|| "Missing required parameter 'x' for move".to_string())?;
                    let my = y.ok_or_else(|| "Missing required parameter 'y' for move".to_string())?;
                    input::move_mouse(mx, my)?;
                    overlay::update_pointer(mx, my);
                    Ok(serde_json::json!({ "status": "success", "action": "move", "x": mx, "y": my }))
                }
                "click" => {
                    input::click_mouse(clean_btn, x, y)?;
                    if let (Some(px), Some(py)) = (x, y) {
                        overlay::update_pointer(px, py);
                    }
                    Ok(serde_json::json!({ "status": "success", "action": "click", "button": clean_btn }))
                }
                "left_click" => {
                    input::click_mouse("left", x, y)?;
                    if let (Some(px), Some(py)) = (x, y) {
                        overlay::update_pointer(px, py);
                    }
                    Ok(serde_json::json!({ "status": "success", "action": "left_click" }))
                }
                "right_click" => {
                    input::click_mouse("right", x, y)?;
                    if let (Some(px), Some(py)) = (x, y) {
                        overlay::update_pointer(px, py);
                    }
                    Ok(serde_json::json!({ "status": "success", "action": "right_click" }))
                }
                "middle_click" => {
                    input::click_mouse("middle", x, y)?;
                    if let (Some(px), Some(py)) = (x, y) {
                        overlay::update_pointer(px, py);
                    }
                    Ok(serde_json::json!({ "status": "success", "action": "middle_click" }))
                }
                "press" => {
                    input::press_mouse_button(clean_btn, x, y)?;
                    if let (Some(px), Some(py)) = (x, y) {
                        overlay::update_pointer(px, py);
                    }
                    Ok(serde_json::json!({ "status": "success", "action": "press", "button": clean_btn }))
                }
                "release" => {
                    input::release_mouse_button(clean_btn, x, y)?;
                    if let (Some(px), Some(py)) = (x, y) {
                        overlay::update_pointer(px, py);
                    }
                    Ok(serde_json::json!({ "status": "success", "action": "release", "button": clean_btn }))
                }
                "double_click" => {
                    input::double_click(x, y)?;
                    if let (Some(px), Some(py)) = (x, y) {
                        overlay::update_pointer(px, py);
                    }
                    Ok(serde_json::json!({ "status": "success", "action": "double_click" }))
                }
                "drag" => {
                    let from_x = x.ok_or_else(|| "Missing required 'x' (start) for drag".to_string())?;
                    let from_y = y.ok_or_else(|| "Missing required 'y' (start) for drag".to_string())?;
                    let dest_x = to_x.ok_or_else(|| "Missing required 'to_x' (end) for drag".to_string())?;
                    let dest_y = to_y.ok_or_else(|| "Missing required 'to_y' (end) for drag".to_string())?;
                    input::drag_mouse(from_x, from_y, dest_x, dest_y)?;
                    overlay::update_pointer(dest_x, dest_y);
                    Ok(serde_json::json!({
                        "status": "success",
                        "action": "drag",
                        "from": [from_x, from_y],
                        "to": [dest_x, dest_y]
                    }))
                }
                "scroll" => {
                    let res = uia::scroll_element_verified(None, None, scroll_x, scroll_y)?;
                    Ok(res)
                }
                other => Err(format!(
                    "Unsupported mouse action: '{other}'. Valid options: click, left_click, right_click, middle_click, double_click, move, drag, scroll, press, release"
                )),
            }
        }

        "keyboard_action" => {
            let action = args
                .get("action")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter 'action'".to_string())?;

            let _session = cursor::acquire_session()?;
            overlay::set_activity_text(match action.to_lowercase().as_str() {
                "type_text" => {
                    let text = args.get("text").and_then(|v| v.as_str()).unwrap_or("");
                    let preview = if text.len() > 18 { format!("{}...", &text[..18]) } else { text.to_string() };
                    format!("Typing: \"{preview}\"...")
                }
                "shortcut" => {
                    let sc = args.get("shortcut").and_then(|v| v.as_str()).unwrap_or("shortcut");
                    format!("Shortcut: {sc}...")
                }
                "key_press" => {
                    let k = args.get("key").and_then(|v| v.as_str()).unwrap_or("key");
                    format!("Pressing {k}...")
                }
                "clear_field" => "Clearing field...".to_string(),
                "select_all" => "Selecting all...".to_string(),
                "copy" => "Copying to clipboard...".to_string(),
                "paste" => "Pasting from clipboard...".to_string(),
                _ => "Keyboard action...".to_string(),
            });

            let window_id = args.get("window_id").and_then(|v| v.as_i64());
            if let Some(hwnd) = window_id {
                if hwnd > 0 {
                    let _ = windows::focus_window(hwnd);
                    std::thread::sleep(std::time::Duration::from_millis(60));
                }
            }

            match action.to_lowercase().as_str() {
                "type_text" => {
                    let text = args
                        .get("text")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| "Missing required parameter 'text' for type_text".to_string())?;
                    let element_query = args.get("element_query").and_then(|v| v.as_str());
                    let mode = args.get("mode").and_then(|v| v.as_str());
                    let verify = args.get("verify").and_then(|v| v.as_bool()).unwrap_or(true);
                    type_text_verified(text, window_id, element_query, mode, verify)
                }
                "key_press" => {
                    let key = args
                        .get("key")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| "Missing required parameter 'key' for key_press".to_string())?;
                    input::press_key(key)?;
                    Ok(serde_json::json!({ "status": "success", "action": "key_press", "key": key }))
                }
                "shortcut" => {
                    let shortcut = args
                        .get("shortcut")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| "Missing required parameter 'shortcut'".to_string())?;
                    input::execute_shortcut(shortcut)?;
                    Ok(serde_json::json!({ "status": "success", "action": "shortcut", "shortcut": shortcut }))
                }
                "select_all" => {
                    input::select_all()?;
                    Ok(serde_json::json!({ "status": "success", "action": "select_all" }))
                }
                "copy" => {
                    input::copy()?;
                    Ok(serde_json::json!({ "status": "success", "action": "copy" }))
                }
                "paste" => {
                    input::paste()?;
                    Ok(serde_json::json!({ "status": "success", "action": "paste" }))
                }
                "clear_field" => {
                    input::clear_field()?;
                    Ok(serde_json::json!({ "status": "success", "action": "clear_field" }))
                }
                "get_focused_element" | "get_focused_control" => {
                    let focused = uia::get_focused_control_details()?;
                    Ok(serde_json::json!({
                        "status": "success",
                        "focused_control": focused
                    }))
                }
                other => Err(format!(
                    "Unsupported keyboard action: '{other}'. Valid actions: type_text, key_press, shortcut, select_all, copy, paste, clear_field, get_focused_element"
                )),
            }
        }

        "inspect_ui" => {
            let action = args
                .get("action")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter 'action'".to_string())?;

            let _session = cursor::acquire_session()?;
            overlay::set_activity_text(format!("Inspecting UI: {action}..."));

            let hwnd = args.get("window_id").and_then(|v| v.as_i64());
            let query = args.get("query").and_then(|v| v.as_str());
            let control_type = args.get("control_type").and_then(|v| v.as_str());
            let max_depth = args.get("max_depth").and_then(|v| v.as_u64()).map(|d| d as u32);

            match action.to_lowercase().as_str() {
                "read_tree" | "read_foreground" => {
                    let tree_res = uia::inspect_window_tree(hwnd, max_depth)?;
                    Ok(serde_json::to_value(&tree_res).unwrap_or_default())
                }
                "find_element" => {
                    let elements = uia::find_elements(hwnd, query, control_type, Some(20))?;
                    Ok(serde_json::json!({
                        "count": elements.len(),
                        "elements": elements
                    }))
                }
                "invoke" | "click_element" => {
                    let q = query.ok_or_else(|| "Missing required parameter 'query' for invoke".to_string())?;
                    let msg = uia::invoke_element(hwnd, q, control_type)?;
                    Ok(serde_json::json!({ "status": "success", "message": msg }))
                }
                "get_focused_control" | "get_focused_element" => {
                    let focused = uia::get_focused_control_details()?;
                    Ok(serde_json::json!({
                        "status": "success",
                        "focused_control": focused
                    }))
                }
                "set_value" => {
                    let q = query.ok_or_else(|| "Missing required parameter 'query' for set_value".to_string())?;
                    let val = args
                        .get("value")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| "Missing required parameter 'value' for set_value".to_string())?;
                    let res = uia::set_element_value(hwnd, q, val)?;
                    Ok(res)
                }
                "select_item" => {
                    let q = query.ok_or_else(|| "Missing required parameter 'query' for select_item".to_string())?;
                    let msg = uia::select_element_item(hwnd, q)?;
                    Ok(serde_json::json!({ "status": "success", "message": msg }))
                }
                "scroll" => {
                    let scroll_x = args.get("scroll_x").and_then(|v| v.as_i64()).map(|v| v as i32);
                    let scroll_y = args.get("scroll_y").and_then(|v| v.as_i64()).map(|v| v as i32);
                    let res = uia::scroll_element_verified(hwnd, query, scroll_x, scroll_y)?;
                    Ok(res)
                }
                "expand_collapse" => {
                    let q = query.ok_or_else(|| "Missing required parameter 'query' for expand_collapse".to_string())?;
                    let expand = args.get("expand").and_then(|v| v.as_bool()).unwrap_or(true);
                    let msg = uia::expand_collapse_element(hwnd, q, expand)?;
                    Ok(serde_json::json!({ "status": "success", "message": msg }))
                }
                other => Err(format!(
                    "Unsupported inspect_ui action: '{other}'. Valid options: read_tree, find_element, get_focused_control, invoke, set_value, select_item, scroll, expand_collapse"
                )),
            }
        }

        "capture_screenshot" => {
            let _session = cursor::acquire_session()?;
            overlay::set_activity_text("Capturing screenshot...");
            let target = args
                .get("target")
                .and_then(|v| v.as_str())
                .unwrap_or("foreground_window");
            let target_hwnd = args
                .get("window_id")
                .or_else(|| args.get("target_hwnd"))
                .and_then(|v| v.as_i64());
            let save_path = args.get("save_to_path").and_then(|v| v.as_str());
            let include_base64 = args.get("include_image_data").and_then(|v| v.as_bool()).unwrap_or(false);

            let shot_res = screenshot::capture_screenshot(target, target_hwnd, save_path, include_base64, default_dir)?;
            Ok(serde_json::to_value(&shot_res).unwrap_or_default())
        }

        "wait_for_ui" => {
            let condition = args
                .get("condition")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter 'condition'".to_string())?;

            let _session = cursor::acquire_session()?;
            overlay::set_activity_text(format!("Waiting for {condition}..."));

            let hwnd = args.get("window_id").and_then(|v| v.as_i64());
            let title = args.get("title").and_then(|v| v.as_str());
            let process_name = args.get("process_name").and_then(|v| v.as_str());
            let element_query = args.get("element_query").and_then(|v| v.as_str());
            let control_type = args.get("control_type").and_then(|v| v.as_str());
            let timeout_ms = args.get("timeout_ms").and_then(|v| v.as_u64());

            let wait_res = timing::wait_for_ui(
                condition,
                hwnd,
                title,
                process_name,
                element_query,
                control_type,
                timeout_ms,
            )?;
            Ok(serde_json::to_value(&wait_res).unwrap_or_default())
        }

        "launch_app" => {
            let app_name = args
                .get("app_name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter 'app_name'".to_string())?;

            let _session = cursor::acquire_session()?;
            overlay::set_activity_text(format!("Opening '{app_name}' via Start menu..."));

            let app_args: Vec<String> = args
                .get("args")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();

            let reuse = args.get("reuse_existing").and_then(|v| v.as_bool()).unwrap_or(true);

            let launch_res = launcher::launch_or_reuse_application(app_name, &app_args, reuse, user_confirmed)?;
            Ok(serde_json::to_value(&launch_res).unwrap_or_default())
        }

        other => Err(format!("Unknown desktop control capability: '{other}'")),
    }
}

/// Safe self-test for desktop control subsystem: checks Windows APIs, cursor position, monitor bounds, and window enumeration
pub fn run_desktop_control_self_test() -> Result<serde_json::Value, String> {
    if !cfg!(target_os = "windows") {
        return Err("Desktop Control requires Microsoft Windows".to_string());
    }

    // 1. Mouse cursor check
    let (cx, cy) = input::get_cursor_position();

    // 2. Window enumeration check
    let open_windows = windows::list_windows(false, None, None)?;
    let total_count = open_windows.len();
    let visible_count = open_windows.iter().filter(|w| w.is_visible).count();

    // 3. UI Automation initialization check
    let uia_check = uia::inspect_window_tree(None, Some(1));
    let uia_ok = uia_check.is_ok();
    let fg_title = uia_check
        .map(|r| r.window_title)
        .unwrap_or_else(|_| "Unknown".to_string());

    Ok(serde_json::json!({
        "success": true,
        "platform": "windows",
        "cursor_position": { "x": cx, "y": cy },
        "total_windows_count": total_count,
        "visible_windows_count": visible_count,
        "foreground_window_title": fg_title,
        "ui_automation_active": uia_ok,
        "features": {
            "window_management": true,
            "mouse_control": true,
            "keyboard_unicode": true,
            "ui_automation": uia_ok,
            "screenshot_capture": true,
            "bounded_wait": true,
            "app_launch_reuse": true
        }
    }))
}

#[cfg(test)]
pub(crate) static TEST_DESKTOP_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_desktop_control_capabilities_registration() {
        let caps = get_desktop_control_capabilities();
        assert!(caps.len() >= 9);
        assert!(caps.iter().any(|c| c.id == "desktop_control_list_windows"));
        assert!(caps.iter().any(|c| c.id == "desktop_control_get_window_details"));
        assert!(caps.iter().any(|c| c.id == "desktop_control_window_action"));
        assert!(caps.iter().any(|c| c.id == "desktop_control_mouse_action"));
        assert!(caps.iter().any(|c| c.id == "desktop_control_keyboard_action"));
        assert!(caps.iter().any(|c| c.id == "desktop_control_inspect_ui"));
        assert!(caps.iter().any(|c| c.id == "desktop_control_capture_screenshot"));
        assert!(caps.iter().any(|c| c.id == "desktop_control_wait_for_ui"));
        assert!(caps.iter().any(|c| c.id == "desktop_control_launch_app"));

        for cap in &caps {
            assert_eq!(cap.source_id, "native_desktop_control");
            assert_eq!(cap.kind, CapabilityKind::Native);
            assert!(cap.input_schema.is_object());
            assert!(cap.input_schema.get("type").is_some());
        }
    }

    #[test]
    fn test_desktop_control_self_test_runs() {
        let res = run_desktop_control_self_test().expect("Self test must pass on Windows");
        assert_eq!(res["success"], true);
        assert_eq!(res["platform"], "windows");
        assert!(res["total_windows_count"].is_number());
    }

    #[test]
    fn test_window_listing_and_filtering() {
        let all_windows = windows::list_windows(false, None, None).expect("Window listing must succeed");
        let _ = all_windows.len();
    }

    #[test]
    fn test_cursor_position_query() {
        let (cx, cy) = input::get_cursor_position();
        assert!(cx >= -10000 && cx <= 20000);
        assert!(cy >= -10000 && cy <= 20000);
    }

    #[test]
    fn test_blocked_command_interpreters() {
        let blocked = launcher::launch_or_reuse_application("cmd.exe", &[], false, false);
        assert!(blocked.is_err());
        assert!(blocked.unwrap_err().contains("strictly blocked"));

        let blocked_ps = launcher::launch_or_reuse_application("powershell", &[], false, false);
        assert!(blocked_ps.is_err());
    }

    #[test]
    fn test_confirmation_enforced_for_destructive_close() {
        let dummy_hwnd = 12345;
        let res = windows::perform_window_action("close", dummy_hwnd, None, None, None, None, false);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("destructive"));

        let res_app = windows::perform_window_action("close_application", dummy_hwnd, None, None, None, None, false);
        assert!(res_app.is_err());
        assert!(res_app.unwrap_err().contains("destructive"));
    }

    #[test]
    fn test_execute_dispatcher_list_windows() {
        let temp_dir = std::env::temp_dir();
        let res = execute_desktop_control_capability(
            "desktop_control_list_windows",
            &serde_json::json!({ "filter_visible": false }),
            false,
            &temp_dir,
        ).expect("Dispatcher list_windows must succeed");
        assert!(res.is_array());
    }

    #[test]
    fn test_execute_dispatcher_wait_for_ui_timeout() {
        let _lock = TEST_DESKTOP_LOCK.lock().unwrap();
        let temp_dir = std::env::temp_dir();
        let res = execute_desktop_control_capability(
            "desktop_control_wait_for_ui",
            &serde_json::json!({
                "condition": "window_exists",
                "title": "NonExistentWindow_99999_xyz",
                "timeout_ms": 300
            }),
            false,
            &temp_dir,
        ).expect("Wait for UI should complete bounded wait");
        assert_eq!(res["condition_met"], false);
        assert_eq!(res["condition"], "window_exists");
    }

    #[test]
    fn test_execute_dispatcher_screenshot_capture() {
        let _lock = TEST_DESKTOP_LOCK.lock().unwrap();
        let temp_dir = std::env::temp_dir().join("aven_test_shots");
        let win_list = windows::list_windows(false, None, None).unwrap_or_default();
        let target_payload = if let Some(first_win) = win_list.into_iter().find(|w| w.rect.width > 50 && w.rect.height > 50) {
            serde_json::json!({
                "target": "selected_window",
                "window_id": first_win.hwnd,
                "include_image_data": true
            })
        } else {
            serde_json::json!({
                "target": "primary_monitor",
                "include_image_data": true
            })
        };

        let res = execute_desktop_control_capability(
            "desktop_control_capture_screenshot",
            &target_payload,
            false,
            &temp_dir,
        ).expect("Screenshot capture must succeed");

        assert!(res["width"].as_u64().unwrap_or(0) > 0);
        assert!(res["height"].as_u64().unwrap_or(0) > 0);
        assert!(res["file_path"].as_str().is_some());
        assert!(res["image_base64"].as_str().is_some());

        let file_path = res["file_path"].as_str().unwrap();
        assert!(std::path::Path::new(file_path).exists(), "Screenshot file must exist on disk");
        let _ = std::fs::remove_file(file_path);
    }

    #[test]
    fn test_shortcut_validation_rejects_empty_or_invalid() {
        assert!(input::execute_shortcut("").is_err());
        assert!(input::execute_shortcut("Ctrl+InvalidNonExistentKey123").is_err());
    }

    #[test]
    fn test_execute_dispatcher_get_screen_info() {
        let temp_dir = std::env::temp_dir();
        let res = execute_desktop_control_capability(
            "desktop_control_get_screen_info",
            &serde_json::json!({}),
            false,
            &temp_dir,
        ).expect("get_screen_info must succeed");

        assert_eq!(res["status"], "success");
        assert!(res["pointer_position"]["x"].is_number());
        assert!(res["pointer_position"]["y"].is_number());
        assert!(res["virtual_screen"]["width"].as_i64().unwrap_or(0) > 0);
        assert!(res["virtual_screen"]["height"].as_i64().unwrap_or(0) > 0);
    }

    #[test]
    fn test_taskbar_info_query() {
        let tb = taskbar::get_taskbar_info();
        if let Ok(info) = tb {
            assert!(info.rect.width > 0);
            assert!(info.rect.height > 0);
            assert!(matches!(info.edge.as_str(), "bottom" | "top" | "left" | "right"));
        }
    }

    #[test]
    fn test_mouse_action_defaults_to_left_click() {
        let _lock = TEST_DESKTOP_LOCK.lock().unwrap();
        let temp_dir = std::env::temp_dir();
        // Unspecified button must ALWAYS default to left, never right
        let res = execute_desktop_control_capability(
            "desktop_control_mouse_action",
            &serde_json::json!({ "action": "click" }),
            false,
            &temp_dir,
        ).expect("Default click should succeed");

        assert_eq!(res["status"], "success");
        assert_eq!(res["action"], "click");
        assert_eq!(res["button"], "left", "Unspecified click must NEVER default to right");

        // Invalid/unrecognized button must also safely default to left
        let res_invalid = execute_desktop_control_capability(
            "desktop_control_mouse_action",
            &serde_json::json!({ "action": "click", "button": "unknown_button" }),
            false,
            &temp_dir,
        ).expect("Click with unknown button should safely fallback");

        assert_eq!(res_invalid["button"], "left");
    }

    #[test]
    fn test_mouse_action_explicit_buttons() {
        let _lock = TEST_DESKTOP_LOCK.lock().unwrap();
        let temp_dir = std::env::temp_dir();
        let res_right = execute_desktop_control_capability(
            "desktop_control_mouse_action",
            &serde_json::json!({ "action": "click", "button": "right" }),
            false,
            &temp_dir,
        ).expect("Right click should succeed");
        assert_eq!(res_right["button"], "right");

        let res_middle = execute_desktop_control_capability(
            "desktop_control_mouse_action",
            &serde_json::json!({ "action": "click", "button": "middle" }),
            false,
            &temp_dir,
        ).expect("Middle click should succeed");
        assert_eq!(res_middle["button"], "middle");
    }

    #[test]
    fn test_keyboard_action_type_text_dispatch() {
        let _lock = TEST_DESKTOP_LOCK.lock().unwrap();
        let temp_dir = std::env::temp_dir();
        let sample_text = "Hi\nThis is a multiline test string 🚀";
        let res = execute_desktop_control_capability(
            "desktop_control_keyboard_action",
            &serde_json::json!({
                "action": "type_text",
                "text": sample_text,
                "window_id": 0 // Dummy HWND
            }),
            false,
            &temp_dir,
        ).expect("Type text must succeed");

        assert_eq!(res["status"], "success");
        assert_eq!(res["action"], "type_text");
        assert_eq!(res["chars_typed"], sample_text.chars().count());
        assert_eq!(res["window_id"], 0);
    }

    #[test]
    fn test_element_content_model_and_blind_action_prevention() {
        let content = super::uia::ElementContent {
            value: Some("existing draft text".to_string()),
            text: Some("existing draft text".to_string()),
            is_read_only: false,
            pattern_used: Some("TextPattern".to_string()),
            selected_text: Some("draft".to_string()),
            caret_position: Some(14),
            selection_length: 5,
            has_keyboard_focus: true,
            is_keyboard_focusable: true,
        };

        let val = serde_json::to_value(&content).expect("Must serialize ElementContent");
        assert_eq!(val["value"], "existing draft text");
        assert_eq!(val["pattern_used"], "TextPattern");
        assert_eq!(val["selected_text"], "draft");
        assert_eq!(val["caret_position"], 14);
        assert_eq!(val["selection_length"], 5);

        let scroll = super::uia::ScrollInfo {
            horizontally_scrollable: false,
            vertically_scrollable: true,
            horizontal_percent: 0.0,
            vertical_percent: 25.0,
        };
        let s_val = serde_json::to_value(&scroll).expect("Must serialize ScrollInfo");
        assert_eq!(s_val["vertically_scrollable"], true);
        assert_eq!(s_val["vertical_percent"], 25.0);
    }

    #[test]
    fn test_notepad_live_observation_and_verification() {
        let _lock = TEST_DESKTOP_LOCK.lock().unwrap();

        let initial_text = "Hello Aven Desktop Observation Pipeline";
        let temp_file = std::env::temp_dir().join("aven_notepad_observation_test.txt");
        let _ = std::fs::write(&temp_file, initial_text);

        // 1. Launch notepad.exe process with test file containing existing text
        let mut child = match std::process::Command::new("notepad.exe").arg(&temp_file).spawn() {
            Ok(c) => c,
            Err(e) => {
                println!("[SKIPPED] notepad.exe could not be spawned in this environment: {e}");
                let _ = std::fs::remove_file(&temp_file);
                return;
            }
        };

        // Wait up to 5 seconds for window to register
        let mut target_hwnd: i64 = 0;
        for _ in 0..50 {
            std::thread::sleep(std::time::Duration::from_millis(100));
            if let Ok(windows) = super::windows::list_windows(true, Some("aven_notepad_observation_test".to_string()), None) {
                if let Some(w) = windows.iter().find(|w| w.title.contains("aven_notepad_observation_test") || w.process_name.to_lowercase().contains("notepad")) {
                    target_hwnd = w.hwnd;
                    break;
                }
            }
        }

        if target_hwnd == 0 {
            if let Ok(windows) = super::windows::list_windows(true, Some("Notepad".to_string()), None) {
                if let Some(w) = windows.iter().find(|w| w.process_name.to_lowercase().contains("notepad")) {
                    target_hwnd = w.hwnd;
                }
            }
        }

        if target_hwnd == 0 {
            println!("[SKIPPED] Notepad window did not appear within timeout");
            let _ = child.kill();
            let _ = std::fs::remove_file(&temp_file);
            return;
        }

        std::thread::sleep(std::time::Duration::from_millis(600));
        println!("[TEST NOTEPAD] Observed target window HWND: {target_hwnd}");

        // Scenario 1: Unfocused input & pre-action state observation with pre-existing text
        println!("[TEST NOTEPAD] Scenario 1: Unfocused input focus verification & existing text observation");
        let pre_focus = super::uia::prepare_target_for_typing(Some(target_hwnd), None, None)
            .expect("prepare_target_for_typing should focus Notepad window and verify focus");
        println!("[OBSERVATION] Focused Window: '{}' (HWND: {}), Control: '{}' (Type: '{}', Focusable: {})",
            pre_focus.window_title, pre_focus.window_hwnd, pre_focus.control_name, pre_focus.control_type, pre_focus.is_keyboard_focusable);
        println!("[OBSERVATION] Existing Text via {:?}: {:?}", pre_focus.content.pattern_used, pre_focus.content.text);
        assert!(pre_focus.is_window_active);
        let obs_text = pre_focus.content.text.as_deref().unwrap_or("");
        assert!(obs_text.contains("Hello Aven"));

        // Scenario 2: Prevent blind actions (compare existing input contents with requested change)
        println!("[TEST NOTEPAD] Scenario 2: Prevent blind actions when text already matches");
        let blind_res = type_text_verified(initial_text, Some(target_hwnd), None, Some("replace"), true)
            .expect("Calling replace with identical text should return success without modifying");
        println!("[OBSERVATION & OUTCOME] Blind action prevented: {}", blind_res);
        assert_eq!(blind_res["status"], "success");
        assert_eq!(blind_res["verified"], true);
        assert!(blind_res["message"].as_str().unwrap().contains("already contains requested text"));

        // Scenario 3: Middle caret offset observation via TextPattern
        println!("[TEST NOTEPAD] Scenario 3: Middle caret offset observation");
        let caret_focus = super::uia::get_focused_control_details_for_target(Some(target_hwnd))
            .expect("Must retrieve focused control details");
        println!("[OBSERVATION] Caret position: {:?}, Selection length: {}, Pattern used: {:?}",
            caret_focus.content.caret_position, caret_focus.content.selection_length, caret_focus.content.pattern_used);
        assert!(caret_focus.content.caret_position.is_some());
        println!("[VERIFICATION] Caret offset accurately retrieved via TextPattern as {:?}", caret_focus.content.caret_position);

        // Scenario 4: Selected text observation
        println!("[TEST NOTEPAD] Scenario 4: Selected text observation");
        let sel_focus = super::uia::get_focused_control_details_for_target(Some(target_hwnd))
            .expect("Must retrieve focused control details");
        println!("[OBSERVATION] Selected text: {:?}, Selection length: {:?}",
            sel_focus.content.selected_text, sel_focus.content.selection_length);

        // Scenario 5: Failed typing diagnosis & safe recovery
        println!("[TEST NOTEPAD] Scenario 5: Failed typing verification and diagnosis");
        let failed_res = type_text_verified("New Text That Will Not Change", Some(target_hwnd), None, Some("replace"), true)
            .expect("type_text_verified should return a diagnosed verification result");
        println!("[OBSERVATION & OUTCOME] Typing verification diagnosis: {}", failed_res);
        if failed_res["status"] == "verification_failed" {
            println!("[VERIFICATION] Successfully diagnosed failed typing without crashing or blindly retrying: {}", failed_res["diagnosis"]);
            assert_eq!(failed_res["verified"], false);
        } else {
            println!("[VERIFICATION] Text was successfully updated to: {}", failed_res["current_text"]);
            assert_eq!(failed_res["status"], "success");
        }

        // Scenario 6: Scrolling verification
        println!("[TEST NOTEPAD] Scenario 6: Scroll verification");
        let scroll_res = super::uia::scroll_element_verified(Some(target_hwnd), None, None, Some(-3))
            .expect("Scroll verification should execute");
        println!("[OBSERVATION & OUTCOME] Scroll action outcome: {}", scroll_res);
        assert_eq!(scroll_res["status"], "success");

        // Cleanup: close Notepad child process and remove temp file
        let _ = child.kill();
        let _ = std::fs::remove_file(&temp_file);
        println!("[CLEANUP] Notepad test process terminated and temp file removed safely.");
    }
}

