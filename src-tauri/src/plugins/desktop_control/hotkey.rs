use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::thread;
use tauri::Emitter;
use windows::Win32::Foundation::{HWND, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT,
    VK_BACK,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, MSG, WM_HOTKEY,
};

pub static GLOBAL_CANCELLED: AtomicBool = AtomicBool::new(false);
static HOTKEY_INITIALIZED: OnceLock<()> = OnceLock::new();
const EMERGENCY_HOTKEY_ID: i32 = 0xAA01;

/// Initialize the global emergency stop hotkey (Ctrl+Shift+Backspace) on a dedicated message thread
pub fn init_global_emergency_hotkey(app: tauri::AppHandle) {
    HOTKEY_INITIALIZED.get_or_init(|| {
        let app_clone = app.clone();
        thread::spawn(move || {
            unsafe {
                let modifiers = HOT_KEY_MODIFIERS(MOD_CONTROL.0 | MOD_SHIFT.0 | MOD_NOREPEAT.0);
                let vk = VK_BACK.0 as u32;

                let reg_res = RegisterHotKey(HWND::default(), EMERGENCY_HOTKEY_ID, modifiers, vk);
                if let Err(e) = reg_res {
                    eprintln!("Warning: Failed to register emergency hotkey Ctrl+Shift+Backspace: {e:?}");
                    return;
                }

                let mut msg = MSG::default();
                while GetMessageW(&mut msg, HWND::default(), 0, 0).as_bool() {
                    if msg.message == WM_HOTKEY && msg.wParam == WPARAM(EMERGENCY_HOTKEY_ID as usize) {
                        eprintln!("Emergency global hotkey triggered: Ctrl+Shift+Backspace!");
                        trigger_emergency_stop(&app_clone);
                    }
                    let _ = DispatchMessageW(&msg);
                }

                let _ = UnregisterHotKey(HWND::default(), EMERGENCY_HOTKEY_ID);
            }
        });
    });
}

/// Trigger emergency stop across all desktop automation, overlays, input locks, and active AI streams
pub fn trigger_emergency_stop(app: &tauri::AppHandle) {
    GLOBAL_CANCELLED.store(true, Ordering::SeqCst);

    // 1. Immediately restore cursor, reset session counts, hide overlay, and release locks
    super::cursor::force_restore_and_cleanup();

    // 2. Emit emergency stop event to frontend chat UI
    let _ = app.emit("agent-emergency-stopped", serde_json::json!({
        "shortcut": "Ctrl+Shift+Backspace",
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    }));
}

/// Check if global cancellation was triggered
pub fn is_cancelled() -> bool {
    GLOBAL_CANCELLED.load(Ordering::SeqCst)
}

/// Reset cancellation flag when a new agent task starts
pub fn reset_cancel_flag() {
    GLOBAL_CANCELLED.store(false, Ordering::SeqCst);
}
