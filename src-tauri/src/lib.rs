use keyring::Entry;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::Manager;

pub mod cloudflare;
pub mod context_manager;
pub mod credentials;
pub mod plugins;
pub mod storage;

const SERVICE_NAME: &str = "com.aven.agent";

#[derive(Debug, Serialize, Deserialize)]
pub struct KeyStatus {
    pub provider: String,
    pub has_key: bool,
}

#[tauri::command]
fn set_api_key(provider: String, key: String) -> Result<(), String> {
    let validated = credentials::parse_and_validate_keys(&key)?;
    let joined = validated.join(", ");
    let entry = Entry::new(SERVICE_NAME, &format!("api_key_{}", provider))
        .map_err(|e| format!("Failed to create keyring entry: {e}"))?;
    entry.set_password(&joined)
        .map_err(|e| format!("Failed to store key in OS credential manager: {e}"))?;
    Ok(())
}

#[tauri::command]
fn get_api_key(provider: String) -> Result<Option<String>, String> {
    let entry = Entry::new(SERVICE_NAME, &format!("api_key_{}", provider))
        .map_err(|e| format!("Failed to access keyring: {e}"))?;
    match entry.get_password() {
        Ok(secret) => Ok(Some(secret)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("Keyring error: {e}")),
    }
}

#[tauri::command]
fn has_api_key(provider: String) -> Result<bool, String> {
    let entry = Entry::new(SERVICE_NAME, &format!("api_key_{}", provider))
        .map_err(|e| format!("Failed to access keyring: {e}"))?;
    match entry.get_password() {
        Ok(secret) => Ok(!secret.is_empty()),
        Err(keyring::Error::NoEntry) => Ok(false),
        Err(_) => Ok(false),
    }
}

#[tauri::command]
fn delete_api_key(provider: String) -> Result<(), String> {
    let entry = Entry::new(SERVICE_NAME, &format!("api_key_{}", provider))
        .map_err(|e| format!("Failed to access keyring: {e}"))?;
    match entry.delete_credential() {
        Ok(_) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("Failed to delete credential: {e}")),
    }
}

fn get_settings_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let config_dir = app.path().app_config_dir()
        .map_err(|e| format!("Failed to resolve app config dir: {e}"))?;
    if !config_dir.exists() {
        let _ = fs::create_dir_all(&config_dir);
    }
    Ok(config_dir.join("aven_settings.json"))
}

#[tauri::command]
fn save_settings(app: tauri::AppHandle, settings_json: String) -> Result<(), String> {
    let file_path = get_settings_path(&app)?;
    fs::write(file_path, settings_json)
        .map_err(|e| format!("Failed to write settings: {e}"))?;
    Ok(())
}

#[tauri::command]
fn load_settings(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let file_path = get_settings_path(&app)?;
    if !file_path.exists() {
        return Ok(None);
    }
    match fs::read_to_string(file_path) {
        Ok(contents) => Ok(Some(contents)),
        Err(e) => Err(format!("Failed to read settings: {e}")),
    }
}

#[tauri::command]
fn get_system_status() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "status": "ready",
        "version": "0.1.0",
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "security_storage": "Windows Credential Manager",
    }))
}

// Window control commands for custom title bar
#[tauri::command]
fn minimize_window(window: tauri::Window) -> Result<(), String> {
    window.minimize().map_err(|e| e.to_string())
}

#[tauri::command]
fn toggle_maximize_window(window: tauri::Window) -> Result<bool, String> {
    let is_max = window.is_maximized().map_err(|e| e.to_string())?;
    if is_max {
        window.unmaximize().map_err(|e| e.to_string())?;
        Ok(false)
    } else {
        window.maximize().map_err(|e| e.to_string())?;
        Ok(true)
    }
}

#[tauri::command]
fn is_window_maximized(window: tauri::Window) -> Result<bool, String> {
    window.is_maximized().map_err(|e| e.to_string())
}

#[tauri::command]
fn close_window(window: tauri::Window) -> Result<(), String> {
    window.close().map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let initial_pairs = cloudflare::load_credentials_from_keyring().unwrap_or_default();
    let cloudflare_state = Arc::new(cloudflare::CloudflareState::new(initial_pairs));
    let plugins_manager = Arc::new(plugins::PluginsManager::new("d:\\Projects\\Desktop\\Aven Agent".to_string()));
    let storage_state = Arc::new(storage::ChatStorageState::new());

    let pm_clone = Arc::clone(&plugins_manager);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(cloudflare_state)
        .manage(plugins_manager)
        .manage(storage_state)
        .setup(move |app| {
            // Initialize cursor subsystem lifecycle, inactivity watchdog, and ensure default cursor restoration
            plugins::desktop_control::cursor::init_lifecycle_and_restore();

            // Guarantee cursor and overlay restoration even in case of unexpected panic
            let default_panic = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |info| {
                plugins::desktop_control::cursor::force_restore_and_cleanup();
                default_panic(info);
            }));

            let icon_bytes = include_bytes!("../icons/icon.png");
            if let Ok(icon) = tauri::image::Image::from_bytes(icon_bytes) {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.set_icon(icon);
                    window.on_window_event(move |event| {
                        match event {
                            tauri::WindowEvent::Destroyed | tauri::WindowEvent::CloseRequested { .. } => {
                                plugins::desktop_control::cursor::force_restore_and_cleanup();
                            }
                            _ => {}
                        }
                    });
                }
            }

            let handle = app.handle().clone();
            plugins::desktop_control::hotkey::init_global_emergency_hotkey(handle.clone());
            tauri::async_runtime::spawn(async move {
                pm_clone.load_configs_from_disk(&handle).await;
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            set_api_key,
            get_api_key,
            has_api_key,
            delete_api_key,
            save_settings,
            load_settings,
            get_system_status,
            minimize_window,
            toggle_maximize_window,
            is_window_maximized,
            close_window,
            cloudflare::get_cloudflare_status,
            cloudflare::save_cloudflare_credentials,
            cloudflare::test_cloudflare_credentials,
            cloudflare::clear_cloudflare_credentials,
            cloudflare::start_chat_stream,
            cloudflare::stop_chat_stream,
            plugins::get_plugins_state,
            plugins::save_plugin_config,
            plugins::delete_plugin_config,
            plugins::test_plugin_connection,
            plugins::connect_plugin,
            plugins::disconnect_plugin,
            plugins::execute_capability,
            storage::load_conversations,
            storage::save_conversation,
            storage::delete_conversation,
            storage::rename_conversation,
            storage::save_active_conversation_id,
            storage::clear_all_conversations
        ])
        .build(tauri::generate_context!())
        .expect("error while building Aven desktop application")
        .run(|_app_handle, event| {
            if matches!(event, tauri::RunEvent::Exit | tauri::RunEvent::ExitRequested { .. }) {
                plugins::desktop_control::cursor::force_restore_and_cleanup();
            }
        });
}
