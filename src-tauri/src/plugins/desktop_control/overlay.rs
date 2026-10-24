use std::sync::atomic::{AtomicBool, AtomicI32, AtomicIsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread;
use windows::core::w;
use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreatePen, CreateSolidBrush, DeleteObject, EndPaint, FillRect, GetStockObject,
    GetTextExtentPoint32W, InvalidateRect, LineTo, MoveToEx, RoundRect, SelectObject, SetBkMode,
    SetTextColor, TextOutW, BLACK_BRUSH, HBRUSH, HDC, PAINTSTRUCT, PS_SOLID,
    TRANSPARENT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, GetSystemMetrics,
    PostMessageW, RegisterClassW, SetLayeredWindowAttributes, SetWindowPos, ShowWindow,
    CS_HREDRAW, CS_VREDRAW, HWND_TOPMOST, LWA_COLORKEY, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN,
    SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW,
    SW_HIDE, SW_SHOWNOACTIVATE, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP, WM_DESTROY, WM_PAINT, WM_USER,
};

static OVERLAY_ACTIVE: AtomicBool = AtomicBool::new(false);
static CURSOR_X: AtomicI32 = AtomicI32::new(0);
static CURSOR_Y: AtomicI32 = AtomicI32::new(0);
static OVERLAY_HWND_RAW: AtomicIsize = AtomicIsize::new(0);
static OVERLAY_THREAD_INIT: OnceLock<()> = OnceLock::new();
static ACTIVITY_LABEL: OnceLock<Mutex<String>> = OnceLock::new();

const WM_UPDATE_OVERLAY: u32 = WM_USER + 101;
const WM_HIDE_OVERLAY: u32 = WM_USER + 102;
const WM_SHOW_OVERLAY: u32 = WM_USER + 103;

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
                let _ = InvalidateRect(hwnd, None, true);
            }
            LRESULT(0)
        }
        WM_SHOW_OVERLAY => {
            unsafe {
                let _ = SetWindowPos(
                    hwnd,
                    HWND_TOPMOST,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
                );
                let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                let _ = InvalidateRect(hwnd, None, true);
            }
            LRESULT(0)
        }
        WM_HIDE_OVERLAY => {
            unsafe {
                let _ = ShowWindow(hwnd, SW_HIDE);
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

unsafe fn paint_overlay(hwnd: HWND, hdc: HDC) {
    unsafe {
        let mut rect = RECT::default();
        let _ = windows::Win32::UI::WindowsAndMessaging::GetClientRect(hwnd, &mut rect);

        // 1. Clear background to black (which LWA_COLORKEY makes 100% transparent)
        let black_brush = GetStockObject(BLACK_BRUSH);
        let _ = FillRect(hdc, &rect, HBRUSH(black_brush.0));

        // Aven Primary Accent Color: RGB(124, 124, 248) -> #7C7CF8
        let aven_color = COLORREF(0x00F87C7C); // 0x00BBGGRR
        let pen = CreatePen(PS_SOLID, 4, aven_color);
        let old_pen = SelectObject(hdc, pen);

        let w = rect.right - rect.left;
        let h = rect.bottom - rect.top;
        let bracket_len = 48;
        let inset = 12;

        // 2. Screen border brackets: active-control visual indicator
        // Top-left corner
        let _ = MoveToEx(hdc, inset, inset + bracket_len, None);
        let _ = LineTo(hdc, inset, inset);
        let _ = LineTo(hdc, inset + bracket_len, inset);

        // Top-right corner
        let _ = MoveToEx(hdc, w - inset - bracket_len, inset, None);
        let _ = LineTo(hdc, w - inset, inset);
        let _ = LineTo(hdc, w - inset, inset + bracket_len);

        // Bottom-left corner
        let _ = MoveToEx(hdc, inset, h - inset - bracket_len, None);
        let _ = LineTo(hdc, inset, h - inset);
        let _ = LineTo(hdc, inset + bracket_len, h - inset);

        // Bottom-right corner
        let _ = MoveToEx(hdc, w - inset - bracket_len, h - inset, None);
        let _ = LineTo(hdc, w - inset, h - inset);
        let _ = LineTo(hdc, w - inset, h - inset - bracket_len);

        let _ = SelectObject(hdc, old_pen);
        let _ = DeleteObject(pen);

        // 3. Render real-time Floating Activity Label near pointer (click-through, always on top)
        let cx = CURSOR_X.load(Ordering::Relaxed);
        let cy = CURSOR_Y.load(Ordering::Relaxed);

        let label_text = {
            let guard = get_activity_label().lock().unwrap();
            if guard.is_empty() {
                "Aven Desktop Control".to_string()
            } else {
                guard.clone()
            }
        };

        let wide_text: Vec<u16> = label_text.encode_utf16().collect();
        let mut text_size = SIZE::default();
        let _ = GetTextExtentPoint32W(hdc, &wide_text, &mut text_size);

        let pill_width = text_size.cx + 28;
        let pill_height = 24.max(text_size.cy + 10);

        // Screen collision-aware positioning:
        // By default offset by (+20, +22) so it does NOT cover the cursor hotspot at (cx, cy)
        let vx = GetSystemMetrics(SM_XVIRTUALSCREEN);
        let vy = GetSystemMetrics(SM_YVIRTUALSCREEN);
        let vw = GetSystemMetrics(SM_CXVIRTUALSCREEN);
        let vh = GetSystemMetrics(SM_CYVIRTUALSCREEN);

        let mut lx = cx + 22;
        let mut ly = cy + 22;

        if lx + pill_width > vx + vw - 12 {
            lx = cx - pill_width - 12; // Flip to left of cursor if near right screen edge
        }
        if ly + pill_height > vy + vh - 12 {
            ly = cy - pill_height - 12; // Flip above cursor if near bottom screen edge
        }

        lx = lx.clamp(vx + 6, vx + vw - pill_width - 6);
        ly = ly.clamp(vy + 6, vy + vh - pill_height - 6);

        // Dark rounded pill background
        let pill_bg_brush = CreateSolidBrush(COLORREF(0x00201818)); // Deep dark blue-gray (RGB 24, 24, 32)
        let pill_border_pen = CreatePen(PS_SOLID, 1, COLORREF(0x00F87C7C)); // #7C7CF8 accent border
        let old_brush = SelectObject(hdc, pill_bg_brush);
        let old_pen2 = SelectObject(hdc, pill_border_pen);

        let _ = RoundRect(hdc, lx, ly, lx + pill_width, ly + pill_height, 8, 8);

        // Glowing status dot inside pill
        let dot_brush = CreateSolidBrush(COLORREF(0x00F87C7C));
        let old_brush2 = SelectObject(hdc, dot_brush);
        let dot_radius = 3;
        let dot_cx = lx + 10;
        let dot_cy = ly + pill_height / 2;
        let _ = windows::Win32::Graphics::Gdi::Ellipse(
            hdc,
            dot_cx - dot_radius,
            dot_cy - dot_radius,
            dot_cx + dot_radius,
            dot_cy + dot_radius,
        );
        let _ = SelectObject(hdc, old_brush2);
        let _ = DeleteObject(dot_brush);

        // Crisp white/light-violet text
        let _ = SetBkMode(hdc, TRANSPARENT);
        let _ = SetTextColor(hdc, COLORREF(0x00F0F0FF)); // Crisp off-white
        let text_x = lx + 18;
        let text_y = ly + (pill_height - text_size.cy) / 2;
        let _ = TextOutW(hdc, text_x, text_y, &wide_text);

        let _ = SelectObject(hdc, old_brush);
        let _ = DeleteObject(pill_bg_brush);
        let _ = SelectObject(hdc, old_pen2);
        let _ = DeleteObject(pill_border_pen);
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
                    hbrBackground: HBRUSH(GetStockObject(BLACK_BRUSH).0),
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
                        // Set black as transparent color key
                        let _ = SetLayeredWindowAttributes(h, COLORREF(0x00000000), 255, LWA_COLORKEY);
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

/// Set real-time activity label text to display near cursor.
/// Stale backend events arriving after cancellation or when no session is active are rejected.
pub fn set_activity_text(text: impl Into<String>) {
    let text_str = text.into();
    if !text_str.is_empty() && (!super::cursor::is_in_active_session() || super::hotkey::is_cancelled()) {
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

/// Show the overlay visual window (called when a desktop control session is active)
pub fn show_overlay_ui() {
    if super::hotkey::is_cancelled() {
        return;
    }
    ensure_overlay_initialized();
    OVERLAY_ACTIVE.store(true, Ordering::SeqCst);
    if let Some(hwnd) = get_overlay_hwnd() {
        unsafe {
            let _ = PostMessageW(hwnd, WM_SHOW_OVERLAY, WPARAM(0), LPARAM(0));
        }
    }
}

/// Hide the overlay visual window (called when all desktop control sessions have ended)
pub fn hide_overlay_ui() {
    OVERLAY_ACTIVE.store(false, Ordering::SeqCst);
    if let Some(hwnd) = get_overlay_hwnd() {
        unsafe {
            let _ = PostMessageW(hwnd, WM_HIDE_OVERLAY, WPARAM(0), LPARAM(0));
        }
    }
}

/// Legacy/Explicit activation: delegates to cursor session acquisition
pub fn show_overlay() {
    let _ = super::cursor::replace_system_cursor();
}

/// Legacy/Explicit deactivation: delegates to forced cleanup
pub fn hide_overlay() {
    super::cursor::force_restore_and_cleanup();
}

/// Check if the AI control overlay is currently visible
pub fn is_overlay_active() -> bool {
    OVERLAY_ACTIVE.load(Ordering::Relaxed)
}

/// Update pointer position on overlay
pub fn update_pointer(x: i32, y: i32) {
    notify_cursor_idle(x, y);
}

/// Notify the overlay of cursor movement. Stale events ignored if no active session.
pub fn notify_cursor_moved(x: i32, y: i32) {
    CURSOR_X.store(x, Ordering::Relaxed);
    CURSOR_Y.store(y, Ordering::Relaxed);

    if OVERLAY_ACTIVE.load(Ordering::Relaxed) && super::cursor::is_in_active_session() {
        if let Some(hwnd) = get_overlay_hwnd() {
            unsafe {
                let _ = PostMessageW(hwnd, WM_UPDATE_OVERLAY, WPARAM(0), LPARAM(0));
            }
        }
    }
}

/// Notify the overlay of cursor arriving at target / becoming idle. Stale events ignored if no active session.
pub fn notify_cursor_idle(x: i32, y: i32) {
    CURSOR_X.store(x, Ordering::Relaxed);
    CURSOR_Y.store(y, Ordering::Relaxed);

    if OVERLAY_ACTIVE.load(Ordering::Relaxed) && super::cursor::is_in_active_session() {
        if let Some(hwnd) = get_overlay_hwnd() {
            unsafe {
                let _ = PostMessageW(hwnd, WM_UPDATE_OVERLAY, WPARAM(0), LPARAM(0));
            }
        }
    }
}

