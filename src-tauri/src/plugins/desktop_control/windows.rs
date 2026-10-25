use serde::{Deserialize, Serialize};
use std::path::Path;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, RECT, TRUE};
use windows::Win32::System::ProcessStatus::K32GetModuleFileNameExW;
use windows::Win32::System::Threading::{
    AttachThreadInput, GetCurrentThreadId, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Input::KeyboardAndMouse::SetActiveWindow;
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, EnumWindows, GetClassNameW, GetForegroundWindow, GetWindowLongW,
    GetWindowRect, GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindow, IsWindowVisible,
    IsZoomed, MoveWindow, PostMessageW, SetForegroundWindow, ShowWindow, GWL_EXSTYLE, SW_MAXIMIZE,
    SW_MINIMIZE, SW_RESTORE, SW_SHOW, WM_CLOSE, WS_EX_TOOLWINDOW,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowInfo {
    pub hwnd: i64,
    pub title: String,
    pub class_name: String,
    pub process_id: u32,
    pub process_name: String,
    pub is_visible: bool,
    pub is_minimized: bool,
    pub is_maximized: bool,
    pub is_active: bool,
    pub rect: WindowRect,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowDetails {
    pub target: WindowInfo,
    pub application_windows: Vec<WindowInfo>,
}

struct EnumContext {
    filter_visible: bool,
    filter_title: Option<String>,
    filter_process: Option<String>,
    windows: Vec<WindowInfo>,
    foreground_hwnd: HWND,
}

unsafe extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    unsafe {
        let ctx = &mut *(lparam.0 as *mut EnumContext);

        let is_visible = IsWindowVisible(hwnd).as_bool();
        if ctx.filter_visible && !is_visible {
            return TRUE;
        }

        // Ignore tool windows unless explicitly searching without visible filter
        if ctx.filter_visible {
            let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
            if (ex_style & (WS_EX_TOOLWINDOW.0 as i32)) != 0 {
                return TRUE;
            }
        }

        // Window title
        let mut title_buf = [0u16; 512];
        let len = GetWindowTextW(hwnd, &mut title_buf);
        let title = if len > 0 {
            String::from_utf16_lossy(&title_buf[..len as usize])
        } else {
            String::new()
        };

        // If filtering by visible, skip windows with empty titles unless specifically requested
        if ctx.filter_visible && title.trim().is_empty() {
            return TRUE;
        }

        if let Some(ref req_title) = ctx.filter_title {
            if !title.to_lowercase().contains(&req_title.to_lowercase()) {
                return TRUE;
            }
        }

        // Class name
        let mut class_buf = [0u16; 256];
        let class_len = GetClassNameW(hwnd, &mut class_buf);
        let class_name = if class_len > 0 {
            String::from_utf16_lossy(&class_buf[..class_len as usize])
        } else {
            String::new()
        };

        // Process ID and Process Name
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));

        let process_name = get_process_name_by_pid(pid);

        if let Some(ref req_proc) = ctx.filter_process {
            if !process_name.to_lowercase().contains(&req_proc.to_lowercase()) {
                return TRUE;
            }
        }

        // Window bounds
        let mut rect = RECT::default();
        let _ = GetWindowRect(hwnd, &mut rect);
        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;

        // Skip windows with zero size when filtering visible
        if ctx.filter_visible && (width <= 0 || height <= 0) {
            return TRUE;
        }

        let is_minimized = IsIconic(hwnd).as_bool();
        let is_maximized = IsZoomed(hwnd).as_bool();
        let is_active = hwnd == ctx.foreground_hwnd;

        ctx.windows.push(WindowInfo {
            hwnd: hwnd.0 as i64,
            title,
            class_name,
            process_id: pid,
            process_name,
            is_visible,
            is_minimized,
            is_maximized,
            is_active,
            rect: WindowRect {
                x: rect.left,
                y: rect.top,
                width,
                height,
            },
        });

        TRUE
    }
}

pub fn get_process_name_by_pid(pid: u32) -> String {
    if pid == 0 {
        return "System".to_string();
    }
    unsafe {
        if let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
            let mut buf = [0u16; 1024];
            let len = K32GetModuleFileNameExW(handle, None, &mut buf);
            let _ = windows::Win32::Foundation::CloseHandle(handle);
            if len > 0 {
                let full_path = String::from_utf16_lossy(&buf[..len as usize]);
                if let Some(file_name) = Path::new(&full_path).file_name() {
                    return file_name.to_string_lossy().to_string();
                }
                return full_path;
            }
        }
    }
    format!("pid_{pid}")
}

static ENUM_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Enumerate open desktop windows
pub fn list_windows(
    filter_visible: bool,
    filter_title: Option<String>,
    filter_process: Option<String>,
) -> Result<Vec<WindowInfo>, String> {
    let _guard = ENUM_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

    // Ensure thread is attached to the desktop window station
    let mut pt = windows::Win32::Foundation::POINT::default();
    let _ = unsafe { windows::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut pt) };

    let foreground = unsafe { GetForegroundWindow() };

    let mut context = EnumContext {
        filter_visible,
        filter_title: filter_title.clone(),
        filter_process: filter_process.clone(),
        windows: Vec::new(),
        foreground_hwnd: foreground,
    };

    // Retry EnumWindows up to 4 times with brief backoff if access is temporarily throttled
    for attempt in 0..4 {
        unsafe {
            windows::Win32::Foundation::SetLastError(windows::Win32::Foundation::WIN32_ERROR(0));
            let lparam = LPARAM(&mut context as *mut _ as isize);
            let _ = EnumWindows(Some(enum_windows_proc), lparam);
        }
        if !context.windows.is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25 * (attempt + 1)));
    }

    if context.windows.is_empty() {
        if let Ok(uia_windows) = get_windows_via_uia(filter_visible, &filter_title, &filter_process, foreground) {
            context.windows = uia_windows;
        }
    }

    Ok(context.windows)
}

fn get_windows_via_uia(
    filter_visible: bool,
    filter_title: &Option<String>,
    filter_process: &Option<String>,
    foreground: HWND,
) -> Result<Vec<WindowInfo>, String> {
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED};
    use windows::Win32::UI::Accessibility::{CUIAutomation, IUIAutomation, TreeScope_Children};

    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let uia: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)
            .map_err(|e| format!("{e}"))?;
        let root = uia.GetRootElement().map_err(|e| format!("{e}"))?;
        let cond = uia.CreateTrueCondition().map_err(|e| format!("{e}"))?;
        let arr = root.FindAll(TreeScope_Children, &cond).map_err(|e| format!("{e}"))?;
        let len = arr.Length().unwrap_or(0);
        let mut list = Vec::new();

        for i in 0..len {
            if let Ok(elem) = arr.GetElement(i) {
                let hwnd_val = elem.CurrentNativeWindowHandle().map(|h| h.0 as i64).unwrap_or(0);
                if hwnd_val == 0 {
                    continue;
                }
                let hwnd = HWND(hwnd_val as *mut _);
                let is_visible = IsWindowVisible(hwnd).as_bool();
                if filter_visible && !is_visible {
                    continue;
                }
                let title = elem.CurrentName().map(|b| b.to_string()).unwrap_or_default();
                if filter_visible && title.trim().is_empty() {
                    continue;
                }
                if let Some(req_t) = filter_title {
                    if !title.to_lowercase().contains(&req_t.to_lowercase()) {
                        continue;
                    }
                }
                let class_name = elem.CurrentClassName().map(|b| b.to_string()).unwrap_or_default();
                let pid = elem.CurrentProcessId().map(|p| p as u32).unwrap_or(0);
                let process_name = get_process_name_by_pid(pid);
                if let Some(req_p) = filter_process {
                    if !process_name.to_lowercase().contains(&req_p.to_lowercase()) {
                        continue;
                    }
                }

                let mut rect = RECT::default();
                let _ = GetWindowRect(hwnd, &mut rect);
                let width = rect.right - rect.left;
                let height = rect.bottom - rect.top;
                if filter_visible && (width <= 0 || height <= 0) {
                    continue;
                }

                let is_minimized = IsIconic(hwnd).as_bool();
                let is_maximized = IsZoomed(hwnd).as_bool();
                let is_active = hwnd == foreground;

                list.push(WindowInfo {
                    hwnd: hwnd_val,
                    title,
                    class_name,
                    process_id: pid,
                    process_name,
                    is_visible,
                    is_minimized,
                    is_maximized,
                    is_active,
                    rect: WindowRect {
                        x: rect.left,
                        y: rect.top,
                        width,
                        height,
                    },
                });
            }
        }
        Ok(list)
    }
}

/// Get detailed information on a specific window and identify all windows belonging to the same application
pub fn get_window_details(
    target_hwnd: Option<i64>,
    title_query: Option<&str>,
    process_query: Option<&str>,
) -> Result<WindowDetails, String> {
    let all = list_windows(false, None, None)?;

    let target = all
        .iter()
        .find(|w| {
            if let Some(h) = target_hwnd {
                if w.hwnd == h {
                    return true;
                }
            }
            if let Some(t) = title_query {
                if !t.is_empty() && w.title.to_lowercase().contains(&t.to_lowercase()) {
                    return true;
                }
            }
            if let Some(p) = process_query {
                if !p.is_empty() && w.process_name.to_lowercase().contains(&p.to_lowercase()) {
                    return true;
                }
            }
            false
        })
        .cloned()
        .ok_or_else(|| {
            format!(
                "Window not found matching criteria (hwnd: {:?}, title: {:?}, process: {:?})",
                target_hwnd, title_query, process_query
            )
        })?;

    // Find all visible windows belonging to this same application (matching PID or process name)
    let application_windows: Vec<WindowInfo> = all
        .into_iter()
        .filter(|w| {
            w.hwnd != target.hwnd
                && (w.process_id == target.process_id
                    || (!target.process_name.is_empty() && w.process_name == target.process_name))
                && w.is_visible
                && !w.title.trim().is_empty()
        })
        .collect();

    Ok(WindowDetails {
        target,
        application_windows,
    })
}

/// Focus and activate a window
pub fn focus_window(hwnd: i64) -> Result<String, String> {
    let h = HWND(hwnd as *mut _);
    unsafe {
        if !IsWindow(h).as_bool() {
            return Err(format!("Window handle {hwnd} is invalid or has been closed"));
        }
        if IsIconic(h).as_bool() {
            let _ = ShowWindow(h, SW_RESTORE);
        } else {
            let _ = ShowWindow(h, SW_SHOW);
        }

        let cur_fg = GetForegroundWindow();
        let cur_thread = GetCurrentThreadId();
        let target_thread = GetWindowThreadProcessId(h, None);
        let fg_thread = if !cur_fg.0.is_null() {
            GetWindowThreadProcessId(cur_fg, None)
        } else {
            0
        };

        if fg_thread != 0 && fg_thread != cur_thread {
            let _ = AttachThreadInput(cur_thread, fg_thread, true);
        }
        if cur_thread != target_thread && target_thread != 0 {
            let _ = AttachThreadInput(cur_thread, target_thread, true);
        }

        let _ = BringWindowToTop(h);
        let _ = SetForegroundWindow(h);
        let _ = SetActiveWindow(h);

        if cur_thread != target_thread && target_thread != 0 {
            let _ = AttachThreadInput(cur_thread, target_thread, false);
        }
        if fg_thread != 0 && fg_thread != cur_thread {
            let _ = AttachThreadInput(cur_thread, fg_thread, false);
        }
    }
    std::thread::sleep(std::time::Duration::from_millis(60));
    Ok(format!("Window (hwnd: {hwnd}) focused"))
}

/// Execute window state actions (minimize, maximize, restore, move, resize, close, close_application)
pub fn perform_window_action(
    action: &str,
    target_hwnd: i64,
    x: Option<i32>,
    y: Option<i32>,
    width: Option<i32>,
    height: Option<i32>,
    user_confirmed: bool,
) -> Result<serde_json::Value, String> {
    let h = HWND(target_hwnd as *mut _);

    match action.to_lowercase().as_str() {
        "focus" | "activate" => {
            focus_window(target_hwnd)?;
            Ok(serde_json::json!({
                "status": "success",
                "action": "focus",
                "hwnd": target_hwnd
            }))
        }
        "minimize" => {
            unsafe {
                let _ = ShowWindow(h, SW_MINIMIZE);
            }
            Ok(serde_json::json!({
                "status": "success",
                "action": "minimize",
                "hwnd": target_hwnd
            }))
        }
        "maximize" => {
            unsafe {
                let _ = ShowWindow(h, SW_MAXIMIZE);
            }
            Ok(serde_json::json!({
                "status": "success",
                "action": "maximize",
                "hwnd": target_hwnd
            }))
        }
        "restore" => {
            unsafe {
                let _ = ShowWindow(h, SW_RESTORE);
            }
            Ok(serde_json::json!({
                "status": "success",
                "action": "restore",
                "hwnd": target_hwnd
            }))
        }
        "move" => {
            let new_x = x.ok_or_else(|| "Missing required parameter 'x' for move".to_string())?;
            let new_y = y.ok_or_else(|| "Missing required parameter 'y' for move".to_string())?;

            let mut rect = RECT::default();
            unsafe {
                let _ = GetWindowRect(h, &mut rect);
                let current_w = rect.right - rect.left;
                let current_h = rect.bottom - rect.top;
                MoveWindow(h, new_x, new_y, current_w, current_h, true)
                    .map_err(|e| format!("MoveWindow failed: {e}"))?;
            }

            Ok(serde_json::json!({
                "status": "success",
                "action": "move",
                "hwnd": target_hwnd,
                "x": new_x,
                "y": new_y
            }))
        }
        "resize" => {
            let new_w = width.ok_or_else(|| "Missing required parameter 'width' for resize".to_string())?;
            let new_h = height.ok_or_else(|| "Missing required parameter 'height' for resize".to_string())?;

            let mut rect = RECT::default();
            unsafe {
                let _ = GetWindowRect(h, &mut rect);
                MoveWindow(h, rect.left, rect.top, new_w, new_h, true)
                    .map_err(|e| format!("Resize MoveWindow failed: {e}"))?;
            }

            Ok(serde_json::json!({
                "status": "success",
                "action": "resize",
                "hwnd": target_hwnd,
                "width": new_w,
                "height": new_h
            }))
        }
        "close" => {
            if !user_confirmed {
                return Err("Closing a window is potentially destructive. Confirmation is required.".to_string());
            }

            unsafe {
                let _ = PostMessageW(h, WM_CLOSE, windows::Win32::Foundation::WPARAM(0), windows::Win32::Foundation::LPARAM(0));
            }

            Ok(serde_json::json!({
                "status": "success",
                "action": "close",
                "hwnd": target_hwnd,
                "message": "Sent WM_CLOSE message to window"
            }))
        }
        "close_application" => {
            if !user_confirmed {
                return Err("Closing an entire application is destructive. Confirmation is required.".to_string());
            }

            // Find all windows of this application and post WM_CLOSE
            let details = get_window_details(Some(target_hwnd), None, None)?;
            let mut closed_count = 1;

            unsafe {
                let _ = PostMessageW(h, WM_CLOSE, windows::Win32::Foundation::WPARAM(0), windows::Win32::Foundation::LPARAM(0));
                for sibling in details.application_windows {
                    let sib_h = HWND(sibling.hwnd as *mut _);
                    let _ = PostMessageW(sib_h, WM_CLOSE, windows::Win32::Foundation::WPARAM(0), windows::Win32::Foundation::LPARAM(0));
                    closed_count += 1;
                }
            }

            Ok(serde_json::json!({
                "status": "success",
                "action": "close_application",
                "process_name": details.target.process_name,
                "windows_closed": closed_count
            }))
        }
        other => Err(format!(
            "Unsupported window action: '{other}'. Valid actions: focus, minimize, maximize, restore, move, resize, close, close_application"
        )),
    }
}
