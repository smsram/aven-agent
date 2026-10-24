use serde::{Deserialize, Serialize};
use std::thread::sleep;
use std::time::Duration;
use windows::core::w;
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::UI::Shell::{
    SHAppBarMessage, ABE_LEFT, ABE_RIGHT, ABE_TOP, ABM_GETSTATE, ABM_GETTASKBARPOS,
    ABS_AUTOHIDE, APPBARDATA,
};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN,
};

use super::windows::WindowRect;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskbarInfo {
    pub edge: String,
    pub rect: WindowRect,
    pub is_autohide: bool,
    pub monitor_width: i32,
    pub monitor_height: i32,
}

/// Retrieve the actual position, screen edge, bounds, and auto-hide status of the Windows Taskbar
pub fn get_taskbar_info() -> Result<TaskbarInfo, String> {
    let mon_w = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let mon_h = unsafe { GetSystemMetrics(SM_CYSCREEN) };

    let mut data = APPBARDATA {
        cbSize: std::mem::size_of::<APPBARDATA>() as u32,
        hWnd: HWND::default(),
        uCallbackMessage: 0,
        uEdge: 0,
        rc: RECT::default(),
        lParam: windows::Win32::Foundation::LPARAM(0),
    };

    let pos_res = unsafe { SHAppBarMessage(ABM_GETTASKBARPOS, &mut data) };
    let state_res = unsafe { SHAppBarMessage(ABM_GETSTATE, &mut data) };
    let is_autohide = (state_res as u32 & ABS_AUTOHIDE) != 0;

    let (edge, rect) = if pos_res != 0 {
        let edge_str = match data.uEdge {
            ABE_LEFT => "left",
            ABE_TOP => "top",
            ABE_RIGHT => "right",
            _ => "bottom",
        };
        let r = WindowRect {
            x: data.rc.left,
            y: data.rc.top,
            width: data.rc.right - data.rc.left,
            height: data.rc.bottom - data.rc.top,
        };
        (edge_str.to_string(), r)
    } else {
        // Fallback: search Shell_TrayWnd window directly
        let hwnd_tray = unsafe { FindWindowW(w!("Shell_TrayWnd"), None) };
        if let Ok(tray) = hwnd_tray {
            if !tray.0.is_null() {
                let mut rc = RECT::default();
                let _ = unsafe { windows::Win32::UI::WindowsAndMessaging::GetWindowRect(tray, &mut rc) };
                let width = rc.right - rc.left;
                let height = rc.bottom - rc.top;
                let edge = if rc.top == 0 && width >= mon_w {
                    "top"
                } else if rc.left == 0 && height >= mon_h {
                    "left"
                } else if rc.right >= mon_w && height >= mon_h {
                    "right"
                } else {
                    "bottom"
                };
                (
                    edge.to_string(),
                    WindowRect {
                        x: rc.left,
                        y: rc.top,
                        width,
                        height,
                    },
                )
            } else {
                (
                    "bottom".to_string(),
                    WindowRect {
                        x: 0,
                        y: mon_h - 48,
                        width: mon_w,
                        height: 48,
                    },
                )
            }
        } else {
            (
                "bottom".to_string(),
                WindowRect {
                    x: 0,
                    y: mon_h - 48,
                    width: mon_w,
                    height: 48,
                },
            )
        }
    };

    Ok(TaskbarInfo {
        edge,
        rect,
        is_autohide,
        monitor_width: mon_w,
        monitor_height: mon_h,
    })
}

/// If the taskbar is configured to auto-hide, move the cursor to its detected screen edge to reveal it
pub fn ensure_taskbar_visible() -> Result<TaskbarInfo, String> {
    let info = get_taskbar_info()?;
    if info.is_autohide {
        let (hover_x, hover_y) = match info.edge.as_str() {
            "top" => (info.monitor_width / 2, 0),
            "left" => (0, info.monitor_height / 2),
            "right" => (info.monitor_width - 1, info.monitor_height / 2),
            _ => (info.monitor_width / 2, info.monitor_height - 1),
        };

        // Smoothly step or set cursor to trigger taskbar slide out
        unsafe {
            let _ = windows::Win32::UI::WindowsAndMessaging::SetCursorPos(hover_x, hover_y);
        }
        sleep(Duration::from_millis(300));
    }
    Ok(info)
}

/// Locate the Start button coordinates dynamically via UI Automation or taskbar geometry
pub fn find_start_button_coordinates() -> Result<(i32, i32), String> {
    let taskbar = ensure_taskbar_visible()?;

    // 1. Try semantic UI Automation query on taskbar
    if let Ok(results) = super::uia::find_elements(
        None,
        Some("Start"),
        None,
        Some(5),
    ) {
        if let Some(btn) = results.first() {
            if let Some(ref r) = btn.bounding_rect {
                if r.width > 0 && r.height > 0 {
                    let cx = r.x + r.width / 2;
                    let cy = r.y + r.height / 2;
                    return Ok((cx, cy));
                }
            }
        }
    }

    // 2. Secondary semantic query searching by name "Start"
    if let Ok(results) = super::uia::find_elements(
        None,
        Some("Start"),
        Some("Button"),
        None,
    ) {
        if let Some(btn) = results.first() {
            if let Some(ref r) = btn.bounding_rect {
                if r.width > 0 && r.height > 0 {
                    let cx = r.x + r.width / 2;
                    let cy = r.y + r.height / 2;
                    return Ok((cx, cy));
                }
            }
        }
    }

    // 3. Fallback to taskbar geometry
    let (cx, cy) = match taskbar.edge.as_str() {
        "top" => (taskbar.rect.x + 24, taskbar.rect.y + taskbar.rect.height / 2),
        "left" => (taskbar.rect.x + taskbar.rect.width / 2, taskbar.rect.y + 24),
        "right" => (taskbar.rect.x + taskbar.rect.width / 2, taskbar.rect.y + 24),
        _ => {
            // If Windows 11 centered taskbar or standard bottom
            // Check if Windows 11 center icons or traditional bottom-left
            if taskbar.rect.width > 300 {
                (taskbar.rect.x + 24, taskbar.rect.y + taskbar.rect.height / 2)
            } else {
                (taskbar.rect.x + taskbar.rect.width / 2, taskbar.rect.y + taskbar.rect.height / 2)
            }
        }
    };

    Ok((cx, cy))
}
