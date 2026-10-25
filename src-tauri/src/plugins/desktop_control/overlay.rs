use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicIsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Duration;
use tauri::{Emitter, Manager};
use windows::core::w;
use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreatePen, CreateSolidBrush, DeleteObject, EndPaint, FillRect, GetStockObject,
    GetTextExtentPoint32W, InvalidateRect, RoundRect, SelectObject, SetBkMode, SetTextColor,
    TextOutW, HDC, PAINTSTRUCT, PS_SOLID, TRANSPARENT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, GetSystemMetrics,
    PostMessageW, RegisterClassW, SetLayeredWindowAttributes, SetWindowPos, ShowWindow,
    CS_HREDRAW, CS_VREDRAW, HWND_TOPMOST, LWA_COLORKEY, SM_CXSCREEN, SM_CXVIRTUALSCREEN,
    SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SWP_NOACTIVATE, SWP_SHOWWINDOW,
    SW_HIDE, SW_SHOWNOACTIVATE, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP, WM_DESTROY, WM_PAINT, WM_USER,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutomationSessionState {
    Idle,
    Running,
    WaitingForModel,
    ExecutingTool,
    Verifying,
    Recovering,
    Completed,
    Failed,
    Cancelled,
}

impl AutomationSessionState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Running => "running",
            Self::WaitingForModel => "waiting_for_model",
            Self::ExecutingTool => "executing_tool",
            Self::Verifying => "verifying",
            Self::Recovering => "recovering",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationSessionPayload {
    pub session_id: String,
    pub state: String,
    pub label: String,
    pub detail: Option<String>,
    pub target: Option<String>,
    pub pointer_x: Option<i32>,
    pub pointer_y: Option<i32>,
}

static AUTOMATION_SESSION_ACTIVE: AtomicBool = AtomicBool::new(false);
static OVERLAY_ACTIVE: AtomicBool = AtomicBool::new(false);
static LAST_TARGET_X: AtomicI32 = AtomicI32::new(-9999);
static LAST_TARGET_Y: AtomicI32 = AtomicI32::new(-9999);
static OVERLAY_HWND_RAW: AtomicIsize = AtomicIsize::new(0);
static OVERLAY_THREAD_INIT: OnceLock<()> = OnceLock::new();
static ACTIVITY_LABEL: OnceLock<Mutex<String>> = OnceLock::new();
static CURRENT_SESSION_ID: OnceLock<Mutex<String>> = OnceLock::new();
static CURRENT_SESSION_STATE: OnceLock<Mutex<AutomationSessionState>> = OnceLock::new();
static WEBVIEW_OVERLAY_WINDOW: OnceLock<Mutex<Option<tauri::WebviewWindow>>> = OnceLock::new();

const WM_UPDATE_OVERLAY: u32 = WM_USER + 101;
const WM_HIDE_OVERLAY: u32 = WM_USER + 102;
const WM_SHOW_OVERLAY: u32 = WM_USER + 103;

fn get_current_session_id() -> &'static Mutex<String> {
    CURRENT_SESSION_ID.get_or_init(|| Mutex::new(String::new()))
}

fn get_current_session_state() -> &'static Mutex<AutomationSessionState> {
    CURRENT_SESSION_STATE.get_or_init(|| Mutex::new(AutomationSessionState::Idle))
}

fn get_webview_overlay_window() -> &'static Mutex<Option<tauri::WebviewWindow>> {
    WEBVIEW_OVERLAY_WINDOW.get_or_init(|| Mutex::new(None))
}

pub fn register_webview_overlay(window: tauri::WebviewWindow) {
    let mut guard = get_webview_overlay_window().lock().unwrap();
    *guard = Some(window);
}

pub fn has_webview_overlay() -> bool {
    get_webview_overlay_window().lock().unwrap().is_some()
}

fn get_activity_label() -> &'static Mutex<String> {
    ACTIVITY_LABEL.get_or_init(|| Mutex::new(String::new()))
}

fn get_overlay_hwnd() -> Option<HWND> {
    let raw = OVERLAY_HWND_RAW.load(Ordering::SeqCst);
    if raw != 0 {
        Some(HWND(raw as *mut _))
    } else {
        None
    }
}

fn set_overlay_hwnd(h: HWND) {
    OVERLAY_HWND_RAW.store(h.0 as isize, Ordering::SeqCst);
}

unsafe extern "system" fn overlay_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_PAINT => {
            unsafe {
                let mut ps = PAINTSTRUCT::default();
                let hdc = BeginPaint(hwnd, &mut ps);
                if !hdc.0.is_null() {
                    paint_overlay(hwnd, hdc);
                }
                let _ = EndPaint(hwnd, &ps);
            }
            LRESULT(0)
        }
        WM_UPDATE_OVERLAY => {
            unsafe {
                let _ = InvalidateRect(hwnd, None, false);
            }
            LRESULT(0)
        }
        WM_SHOW_OVERLAY => {
            unsafe {
                let vx = GetSystemMetrics(SM_XVIRTUALSCREEN);
                let vy = GetSystemMetrics(SM_YVIRTUALSCREEN);
                let vw = GetSystemMetrics(SM_CXVIRTUALSCREEN);
                let vh = GetSystemMetrics(SM_CYVIRTUALSCREEN);

                let _ = SetWindowPos(
                    hwnd,
                    HWND_TOPMOST,
                    vx,
                    vy,
                    vw,
                    vh,
                    SWP_NOACTIVATE | SWP_SHOWWINDOW,
                );
                let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                let _ = InvalidateRect(hwnd, None, false);
            }
            LRESULT(0)
        }
        WM_HIDE_OVERLAY => {
            unsafe {
                let _ = ShowWindow(hwnd, SW_HIDE);
            }
            LRESULT(0)
        }
        WM_DESTROY => LRESULT(0),
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

unsafe fn paint_overlay(hwnd: HWND, hdc: HDC) {
    unsafe {
        let mut rect = RECT::default();
        let _ = windows::Win32::UI::WindowsAndMessaging::GetClientRect(hwnd, &mut rect);
        let w = rect.right - rect.left;
        let h = rect.bottom - rect.top;

        // 1. Fill entire window with pure black (the colorkey) -> 100% transparent.
        // NO blue tint, NO backdrop, NO dimming, NO blur, NO glow, NO screen tint.
        let transparent_brush = CreateSolidBrush(COLORREF(0x00000000));
        let _ = FillRect(hdc, &rect, transparent_brush);
        let _ = DeleteObject(transparent_brush);

        // If automation is not active, keep everything completely transparent
        if !OVERLAY_ACTIVE.load(Ordering::Relaxed) {
            return;
        }

        // 2. Thin clean 2px border around the entire virtual desktop (#686DF6 -> BGR: 0x00F66D68)
        let aven_color = COLORREF(0x00F66D68);
        let border_pen = CreatePen(PS_SOLID, 2, aven_color);
        let old_pen = SelectObject(hdc, border_pen);
        let null_brush = GetStockObject(windows::Win32::Graphics::Gdi::NULL_BRUSH);
        let old_brush = SelectObject(hdc, null_brush);

        let _ = windows::Win32::Graphics::Gdi::Rectangle(hdc, 0, 0, w, h);

        let _ = SelectObject(hdc, old_brush);
        let _ = SelectObject(hdc, old_pen);
        let _ = DeleteObject(border_pen);

        // If Webview overlay is active, the HTML/CSS container handles the activity indicator
        // with crisp DirectWrite typography and SVG animations. Skip GDI text drawing.
        if has_webview_overlay() {
            return;
        }

        // NOTE: Custom cursor drawing is completely removed per user instruction.
        // Real Windows mouse pointer is never replaced, hidden, or modified.

        // 3. Render unobtrusive live activity container near top-center of primary screen
        let label_text = {
            let guard = get_activity_label().lock().unwrap();
            if guard.is_empty() {
                "Analyzing screen".to_string()
            } else {
                guard.clone()
            }
        };

        let wide_text: Vec<u16> = label_text.encode_utf16().collect();
        let mut text_size = SIZE::default();
        let _ = GetTextExtentPoint32W(hdc, &wide_text, &mut text_size);

        let container_width = (text_size.cx + 38).max(140);
        let container_height = 28i32;

        // Position on primary monitor (origin is (0,0) relative to Windows virtual screen)
        let vx = GetSystemMetrics(SM_XVIRTUALSCREEN);
        let vy = GetSystemMetrics(SM_YVIRTUALSCREEN);
        let pw = GetSystemMetrics(SM_CXSCREEN);

        let primary_origin_x = -vx;
        let primary_origin_y = -vy;

        // Positioned at top-center of primary screen with 16px top margin
        let container_x = (primary_origin_x + (pw - container_width) / 2).clamp(vx.abs() + 16, w - container_width - 16);
        let container_y = (primary_origin_y + 16).clamp(16, h - container_height - 16);

        // Transparent container background (NULL_BRUSH) with 1px #686DF6 border
        let null_brush2 = GetStockObject(windows::Win32::Graphics::Gdi::NULL_BRUSH);
        let border_pen2 = CreatePen(PS_SOLID, 1, COLORREF(0x00F66D68));
        let old_brush2 = SelectObject(hdc, null_brush2);
        let old_pen2 = SelectObject(hdc, border_pen2);

        let _ = RoundRect(
            hdc,
            container_x,
            container_y,
            container_x + container_width,
            container_y + container_height,
            8,
            8,
        );

        // Subtle glowing accent status dot
        let dot_brush = CreateSolidBrush(COLORREF(0x00F66D68));
        let old_dot_brush = SelectObject(hdc, dot_brush);
        let dot_radius = 4;
        let dot_cx = container_x + 14;
        let dot_cy = container_y + container_height / 2;
        let _ = windows::Win32::Graphics::Gdi::Ellipse(
            hdc,
            dot_cx - dot_radius,
            dot_cy - dot_radius,
            dot_cx + dot_radius,
            dot_cy + dot_radius,
        );
        let _ = SelectObject(hdc, old_dot_brush);
        let _ = DeleteObject(dot_brush);

        // Crisp off-white text (#F0F0FF)
        let _ = SetBkMode(hdc, TRANSPARENT);
        let _ = SetTextColor(hdc, COLORREF(0x00F0F0FF));
        let text_x = container_x + 24;
        let text_y = container_y + (container_height - text_size.cy) / 2;
        let _ = TextOutW(hdc, text_x, text_y, &wide_text);

        let _ = SelectObject(hdc, old_brush2);
        let _ = SelectObject(hdc, old_pen2);
        let _ = DeleteObject(border_pen2);
    }
}

/// Initialize the native Win32 transparent click-through overlay on a background thread
pub fn ensure_overlay_initialized() {
    OVERLAY_THREAD_INIT.get_or_init(|| {
        thread::spawn(|| {
            unsafe {
                let class_name = w!("AvenAgentAiOverlay");
                let wndclass = WNDCLASSW {
                    style: CS_HREDRAW | CS_VREDRAW,
                    lpfnWndProc: Some(overlay_wnd_proc),
                    cbClsExtra: 0,
                    cbWndExtra: 0,
                    hInstance: HINSTANCE::default(),
                    hIcon: windows::Win32::UI::WindowsAndMessaging::HICON::default(),
                    hCursor: windows::Win32::UI::WindowsAndMessaging::HCURSOR::default(),
                    hbrBackground: windows::Win32::Graphics::Gdi::HBRUSH::default(),
                    lpszMenuName: windows::core::PCWSTR::null(),
                    lpszClassName: class_name,
                };

                let _ = RegisterClassW(&wndclass);

                let vx = GetSystemMetrics(SM_XVIRTUALSCREEN);
                let vy = GetSystemMetrics(SM_YVIRTUALSCREEN);
                let vw = GetSystemMetrics(SM_CXVIRTUALSCREEN);
                let vh = GetSystemMetrics(SM_CYVIRTUALSCREEN);

                let ex_style = WS_EX_LAYERED
                    | WS_EX_TRANSPARENT
                    | WS_EX_TOPMOST
                    | WS_EX_TOOLWINDOW
                    | WS_EX_NOACTIVATE;

                let hwnd = CreateWindowExW(
                    ex_style,
                    class_name,
                    w!("Aven AI Desktop Control Overlay"),
                    WS_POPUP,
                    vx,
                    vy,
                    vw,
                    vh,
                    HWND::default(),
                    windows::Win32::UI::WindowsAndMessaging::HMENU::default(),
                    HINSTANCE::default(),
                    None,
                );

                if let Ok(h) = hwnd {
                    if !h.0.is_null() {
                        // Pure black colorkey (0x00000000) makes the entire interior 100% transparent.
                        // Only the #686DF6 border is visible.
                        let _ = SetLayeredWindowAttributes(h, COLORREF(0), 0, LWA_COLORKEY);
                        let _ = windows::Win32::UI::WindowsAndMessaging::SetWindowDisplayAffinity(
                            h,
                            windows::Win32::UI::WindowsAndMessaging::WDA_EXCLUDEFROMCAPTURE,
                        );
                        set_overlay_hwnd(h);
                    }
                }

                // Win32 message loop for the overlay window
                let mut msg = windows::Win32::UI::WindowsAndMessaging::MSG::default();
                while GetMessageW(&mut msg, HWND::default(), 0, 0).as_bool() {
                    let _ = DispatchMessageW(&msg);
                }
            }
        });
    });
}

fn emit_session_event(payload: &AutomationSessionPayload) {
    if let Ok(guard) = get_webview_overlay_window().lock() {
        if let Some(ref win) = *guard {
            let _ = win.emit("automation-session-state", payload);
            let _ = win.app_handle().emit("automation-session-state", payload);
        }
    }
}

pub fn get_current_session_id_str() -> String {
    get_current_session_id().lock().unwrap().clone()
}

pub fn get_current_session_state_enum() -> AutomationSessionState {
    *get_current_session_state().lock().unwrap()
}

/// Set real-time activity label text to display in the live activity container
pub fn set_activity_text(text: impl Into<String>) {
    let text_str = text.into();
    if super::hotkey::is_cancelled() {
        return;
    }
    {
        let mut guard = get_activity_label().lock().unwrap();
        *guard = text_str;
    }
    if OVERLAY_ACTIVE.load(Ordering::Relaxed) {
        if let Some(hwnd) = get_overlay_hwnd() {
            unsafe {
                let _ = PostMessageW(hwnd, WM_UPDATE_OVERLAY, WPARAM(0), LPARAM(0));
            }
        }
    }
}

/// Clear real-time activity label text
pub fn clear_activity_text() {
    {
        let mut guard = get_activity_label().lock().unwrap();
        guard.clear();
    }
    if OVERLAY_ACTIVE.load(Ordering::Relaxed) {
        if let Some(hwnd) = get_overlay_hwnd() {
            unsafe {
                let _ = PostMessageW(hwnd, WM_UPDATE_OVERLAY, WPARAM(0), LPARAM(0));
            }
        }
    }
}

/// Authoritative start of a desktop automation session.
pub fn start_desktop_automation_session(session_id: &str, initial_label: &str) {
    if super::hotkey::is_cancelled() {
        return;
    }
    ensure_overlay_initialized();
    super::cursor::touch_task_heartbeat(session_id, AutomationSessionState::Running);

    {
        let mut id_guard = get_current_session_id().lock().unwrap();
        *id_guard = session_id.to_string();
    }
    {
        let mut st_guard = get_current_session_state().lock().unwrap();
        *st_guard = AutomationSessionState::Running;
    }

    set_activity_text(initial_label);
    AUTOMATION_SESSION_ACTIVE.store(true, Ordering::SeqCst);
    OVERLAY_ACTIVE.store(true, Ordering::SeqCst);

    if let Some(hwnd) = get_overlay_hwnd() {
        unsafe {
            let _ = PostMessageW(hwnd, WM_SHOW_OVERLAY, WPARAM(0), LPARAM(0));
        }
    }

    if let Ok(guard) = get_webview_overlay_window().lock() {
        if let Some(ref win) = *guard {
            let _ = win.show();
        }
    }

    let payload = AutomationSessionPayload {
        session_id: session_id.to_string(),
        state: AutomationSessionState::Running.as_str().to_string(),
        label: initial_label.to_string(),
        detail: None,
        target: None,
        pointer_x: None,
        pointer_y: None,
    };
    emit_session_event(&payload);
}

/// Authoritative update of the desktop automation session lifecycle and status
pub fn update_automation_session(
    session_id: &str,
    state: AutomationSessionState,
    label: &str,
    detail: Option<String>,
    target: Option<String>,
) {
    if super::hotkey::is_cancelled() {
        return;
    }
    {
        let curr_id = get_current_session_id().lock().unwrap();
        if !curr_id.is_empty() && curr_id.as_str() != session_id {
            // Guard against stale asynchronous events reactivating an already terminated task
            return;
        }
    }

    super::cursor::touch_task_heartbeat(session_id, state);

    {
        let mut st_guard = get_current_session_state().lock().unwrap();
        *st_guard = state;
    }

    set_activity_text(label);

    let payload = AutomationSessionPayload {
        session_id: session_id.to_string(),
        state: state.as_str().to_string(),
        label: label.to_string(),
        detail,
        target,
        pointer_x: {
            let x = LAST_TARGET_X.load(Ordering::Relaxed);
            if x != -9999 { Some(x) } else { None }
        },
        pointer_y: {
            let y = LAST_TARGET_Y.load(Ordering::Relaxed);
            if y != -9999 { Some(y) } else { None }
        },
    };
    emit_session_event(&payload);
}

/// Authoritative end of desktop automation session.
pub fn end_desktop_automation_session(
    session_id: &str,
    final_state: AutomationSessionState,
    final_label: &str,
) {
    {
        let curr_id = get_current_session_id().lock().unwrap();
        if !curr_id.is_empty() && curr_id.as_str() != session_id {
            return;
        }
    }

    super::cursor::touch_task_heartbeat(session_id, final_state);

    {
        let mut st_guard = get_current_session_state().lock().unwrap();
        *st_guard = final_state;
    }

    set_activity_text(final_label);

    let payload = AutomationSessionPayload {
        session_id: session_id.to_string(),
        state: final_state.as_str().to_string(),
        label: final_label.to_string(),
        detail: None,
        target: None,
        pointer_x: None,
        pointer_y: None,
    };
    emit_session_event(&payload);

    let sid = session_id.to_string();
    if final_state == AutomationSessionState::Cancelled {
        // Immediate termination
        {
            let mut curr = get_current_session_id().lock().unwrap();
            if *curr == sid || curr.is_empty() {
                curr.clear();
            }
        }
        *get_current_session_state().lock().unwrap() = AutomationSessionState::Idle;
        AUTOMATION_SESSION_ACTIVE.store(false, Ordering::SeqCst);
        OVERLAY_ACTIVE.store(false, Ordering::SeqCst);
        clear_activity_text();

        if let Some(hwnd) = get_overlay_hwnd() {
            unsafe {
                let _ = PostMessageW(hwnd, WM_HIDE_OVERLAY, WPARAM(0), LPARAM(0));
            }
        }
        if let Ok(guard) = get_webview_overlay_window().lock() {
            if let Some(ref win) = *guard {
                let _ = win.hide();
            }
        }
    } else {
        // Completed or Failed: keep terminal state visible briefly so user sees the result, then hide
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(800));
            let mut curr = get_current_session_id().lock().unwrap();
            if *curr == sid {
                curr.clear();
                *get_current_session_state().lock().unwrap() = AutomationSessionState::Idle;
                AUTOMATION_SESSION_ACTIVE.store(false, Ordering::SeqCst);
                OVERLAY_ACTIVE.store(false, Ordering::SeqCst);
                clear_activity_text();

                if let Some(hwnd) = get_overlay_hwnd() {
                    unsafe {
                        let _ = PostMessageW(hwnd, WM_HIDE_OVERLAY, WPARAM(0), LPARAM(0));
                    }
                }
                if let Ok(guard) = get_webview_overlay_window().lock() {
                    if let Some(ref win) = *guard {
                        let _ = win.hide();
                    }
                }
            }
        });
    }
}

/// Start desktop automation session with auto-generated session id
pub fn start_desktop_automation() {
    let sid = {
        let guard = get_current_session_id().lock().unwrap();
        if guard.is_empty() {
            format!("session-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis())
        } else {
            guard.clone()
        }
    };
    start_desktop_automation_session(&sid, "Analyzing screen");
}

/// Stop desktop automation session
pub fn stop_desktop_automation() {
    let sid = {
        let guard = get_current_session_id().lock().unwrap();
        guard.clone()
    };
    end_desktop_automation_session(&sid, AutomationSessionState::Cancelled, "Cancelled");
}

/// Check if a desktop automation session is currently running
pub fn is_automation_active() -> bool {
    AUTOMATION_SESSION_ACTIVE.load(Ordering::SeqCst) || OVERLAY_ACTIVE.load(Ordering::SeqCst)
}

/// Show overlay UI (delegates to start_desktop_automation)
pub fn show_overlay_ui() {
    start_desktop_automation();
}

/// Hide overlay UI (only hides if the overarching automation session has ended)
pub fn hide_overlay_ui() {
    if !AUTOMATION_SESSION_ACTIVE.load(Ordering::SeqCst) {
        OVERLAY_ACTIVE.store(false, Ordering::SeqCst);
        if let Some(hwnd) = get_overlay_hwnd() {
            unsafe {
                let _ = PostMessageW(hwnd, WM_HIDE_OVERLAY, WPARAM(0), LPARAM(0));
            }
        }
    }
}

/// Explicit activation
pub fn show_overlay() {
    start_desktop_automation();
}

/// Explicit deactivation
pub fn hide_overlay() {
    stop_desktop_automation();
}

/// Check if the AI control overlay is currently visible
pub fn is_overlay_active() -> bool {
    OVERLAY_ACTIVE.load(Ordering::Relaxed)
}

/// Update intended target coordinates (tracked for diagnostics, real cursor is untouched)
pub fn update_target_position(x: i32, y: i32) {
    LAST_TARGET_X.store(x, Ordering::Relaxed);
    LAST_TARGET_Y.store(y, Ordering::Relaxed);
}

/// Update pointer coordinates (tracked for diagnostics, real cursor is untouched)
pub fn update_pointer(x: i32, y: i32) {
    LAST_TARGET_X.store(x, Ordering::Relaxed);
    LAST_TARGET_Y.store(y, Ordering::Relaxed);
}


/// Notify cursor moved (updates target position for diagnostics; custom cursor rendering is removed)
pub fn notify_cursor_moved(x: i32, y: i32) {
    update_target_position(x, y);
}

/// Notify cursor idle (updates target position for diagnostics; custom cursor rendering is removed)
pub fn notify_cursor_idle(x: i32, y: i32) {
    update_target_position(x, y);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_idle_visibility_initially_hidden() {
        assert_eq!(AUTOMATION_SESSION_ACTIVE.load(Ordering::SeqCst), false);
    }

    #[test]
    fn test_session_start_and_stop_lifecycle() {
        start_desktop_automation();
        assert!(is_automation_active());

        set_activity_text("Analyzing screen");
        {
            let guard = get_activity_label().lock().unwrap();
            assert_eq!(*guard, "Analyzing screen");
        }

        stop_desktop_automation();
        assert!(!is_automation_active());
        {
            let guard = get_activity_label().lock().unwrap();
            assert!(guard.is_empty());
        }
    }

    #[test]
    fn test_multiple_consecutive_tool_calls_preserve_active_session() {
        start_desktop_automation();
        assert!(is_automation_active());

        // An individual tool attempt to hide should be a no-op while session is active
        hide_overlay_ui();
        assert!(is_automation_active());

        set_activity_text("Clicking Save");
        assert!(is_automation_active());

        // Only explicit stop terminates the session
        stop_desktop_automation();
        assert!(!is_automation_active());
    }

    #[test]
    fn test_target_position_updating() {
        update_target_position(500, 300);
        assert_eq!(LAST_TARGET_X.load(Ordering::Relaxed), 500);
        assert_eq!(LAST_TARGET_Y.load(Ordering::Relaxed), 300);
    }

    #[test]
    fn test_session_id_lifecycle_and_stale_event_guard() {
        let sid = "session-test-42";
        start_desktop_automation_session(sid, "Starting task");
        assert_eq!(get_current_session_id_str(), sid);
        assert_eq!(get_current_session_state_enum(), AutomationSessionState::Running);

        // Valid update
        update_automation_session(sid, AutomationSessionState::ExecutingTool, "Typing", None, None);
        assert_eq!(get_current_session_state_enum(), AutomationSessionState::ExecutingTool);
        {
            let guard = get_activity_label().lock().unwrap();
            assert_eq!(*guard, "Typing");
        }

        // Stale update from an old session must be safely ignored
        update_automation_session("old-stale-session", AutomationSessionState::Running, "Old action", None, None);
        assert_eq!(get_current_session_state_enum(), AutomationSessionState::ExecutingTool);
        {
            let guard = get_activity_label().lock().unwrap();
            assert_eq!(*guard, "Typing");
        }

        // Clean termination
        end_desktop_automation_session(sid, AutomationSessionState::Cancelled, "Cancelled");
        assert_eq!(get_current_session_state_enum(), AutomationSessionState::Idle);
        assert!(get_current_session_id_str().is_empty());
    }
}
