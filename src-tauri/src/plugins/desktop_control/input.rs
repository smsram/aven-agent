use std::mem::size_of;
use std::thread::sleep;
use std::time::Duration;
use windows::Win32::Foundation::POINT;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYBD_EVENT_FLAGS,
    KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, MOUSEEVENTF_HWHEEL, MOUSEEVENTF_LEFTDOWN,
    MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_RIGHTDOWN,
    MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_WHEEL, MOUSEINPUT, MOUSE_EVENT_FLAGS, VIRTUAL_KEY, VK_BACK,
    VK_CONTROL, VK_DELETE, VK_DOWN, VK_END, VK_ESCAPE, VK_F1, VK_F10, VK_F11, VK_F12, VK_F2,
    VK_F3, VK_F4, VK_F5, VK_F6, VK_F7, VK_F8, VK_F9, VK_HOME, VK_LEFT, VK_LWIN, VK_MENU,
    VK_NEXT, VK_PRIOR, VK_RETURN, VK_RIGHT, VK_SHIFT, VK_SPACE, VK_TAB, VK_UP,
};
use windows::Win32::UI::WindowsAndMessaging::{GetCursorPos, SetCursorPos};

const WHEEL_DELTA: i32 = 120;

/// Get current cursor coordinates
pub fn get_cursor_position() -> (i32, i32) {
    let mut pt = POINT::default();
    unsafe {
        let _ = GetCursorPos(&mut pt);
    }
    (pt.x, pt.y)
}

/// Move cursor to coordinates with natural smooth interpolation
pub fn move_mouse(target_x: i32, target_y: i32) -> Result<(), String> {
    if super::hotkey::is_cancelled() {
        super::cursor::force_restore_and_cleanup();
        return Err("Operation cancelled by user (emergency stop)".to_string());
    }

    let _session = super::cursor::acquire_session()?;

    let (start_x, start_y) = get_cursor_position();
    let dx = (target_x - start_x).abs();
    let dy = (target_y - start_y).abs();
    let dist = ((dx * dx + dy * dy) as f64).sqrt();

    if dist < 6.0 {
        unsafe {
            let _ = SetCursorPos(target_x, target_y);
        }
        super::overlay::notify_cursor_moved(target_x, target_y);
        super::overlay::notify_cursor_idle(target_x, target_y);
        return Ok(());
    }

    // Number of steps based on distance: 8 to 18 steps (100ms - 180ms total)
    let steps = (dist / 35.0).clamp(8.0, 18.0) as i32;
    for i in 1..=steps {
        if super::hotkey::is_cancelled() {
            super::cursor::force_restore_and_cleanup();
            return Err("Operation cancelled by user (emergency stop)".to_string());
        }

        // Cubic ease-out
        let t = i as f64 / steps as f64;
        let ease = 1.0 - (1.0 - t).powi(3);

        let cx = (start_x as f64 + (target_x - start_x) as f64 * ease).round() as i32;
        let cy = (start_y as f64 + (target_y - start_y) as f64 * ease).round() as i32;

        unsafe {
            let _ = SetCursorPos(cx, cy);
        }
        super::overlay::notify_cursor_moved(cx, cy);
        sleep(Duration::from_millis(8));
    }

    unsafe {
        let _ = SetCursorPos(target_x, target_y);
    }
    super::overlay::notify_cursor_idle(target_x, target_y);
    Ok(())
}

/// Send mouse click at optional coordinates
pub fn click_mouse(button: &str, x: Option<i32>, y: Option<i32>) -> Result<(), String> {
    if let (Some(px), Some(py)) = (x, y) {
        move_mouse(px, py)?;
        sleep(Duration::from_millis(15));
    }

    let (down_flag, up_flag) = match button.to_lowercase().as_str() {
        "left" => (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP),
        "right" => (MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP),
        "middle" => (MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP),
        other => return Err(format!("Unsupported mouse button: '{other}'. Use left, right, or middle")),
    };

    send_mouse_event(down_flag, 0)?;
    sleep(Duration::from_millis(25));
    send_mouse_event(up_flag, 0)?;

    Ok(())
}

/// Press and hold mouse button down without immediate release
pub fn press_mouse_button(button: &str, x: Option<i32>, y: Option<i32>) -> Result<(), String> {
    if let (Some(px), Some(py)) = (x, y) {
        move_mouse(px, py)?;
        sleep(Duration::from_millis(15));
    }

    let down_flag = match button.to_lowercase().as_str() {
        "left" => MOUSEEVENTF_LEFTDOWN,
        "right" => MOUSEEVENTF_RIGHTDOWN,
        "middle" => MOUSEEVENTF_MIDDLEDOWN,
        other => return Err(format!("Unsupported mouse button: '{other}'. Use left, right, or middle")),
    };

    send_mouse_event(down_flag, 0)
}

/// Release previously held mouse button
pub fn release_mouse_button(button: &str, x: Option<i32>, y: Option<i32>) -> Result<(), String> {
    if let (Some(px), Some(py)) = (x, y) {
        move_mouse(px, py)?;
        sleep(Duration::from_millis(15));
    }

    let up_flag = match button.to_lowercase().as_str() {
        "left" => MOUSEEVENTF_LEFTUP,
        "right" => MOUSEEVENTF_RIGHTUP,
        "middle" => MOUSEEVENTF_MIDDLEUP,
        other => return Err(format!("Unsupported mouse button: '{other}'. Use left, right, or middle")),
    };

    send_mouse_event(up_flag, 0)
}

/// Double-click with left mouse button
pub fn double_click(x: Option<i32>, y: Option<i32>) -> Result<(), String> {
    click_mouse("left", x, y)?;
    sleep(Duration::from_millis(80));
    click_mouse("left", None, None)?;
    Ok(())
}

/// Smoothly drag mouse from (from_x, from_y) to (to_x, to_y)
pub fn drag_mouse(from_x: i32, from_y: i32, to_x: i32, to_y: i32) -> Result<(), String> {
    move_mouse(from_x, from_y)?;
    sleep(Duration::from_millis(20));

    // Mouse down
    send_mouse_event(MOUSEEVENTF_LEFTDOWN, 0)?;

    // Smooth movement over steps with cubic easing
    let steps = 18;
    for i in 1..=steps {
        if super::hotkey::is_cancelled() {
            super::cursor::force_restore_and_cleanup();
            return Err("Drag cancelled by user (emergency stop)".to_string());
        }

        let t = i as f64 / steps as f64;
        let ease = 1.0 - (1.0 - t).powi(2); // smooth drag curve

        let cur_x = (from_x as f64 + (to_x - from_x) as f64 * ease).round() as i32;
        let cur_y = (from_y as f64 + (to_y - from_y) as f64 * ease).round() as i32;
        unsafe {
            let _ = SetCursorPos(cur_x, cur_y);
        }
        super::overlay::notify_cursor_moved(cur_x, cur_y);
        sleep(Duration::from_millis(10));
    }

    // Ensure mouse up is always called
    let res = send_mouse_event(MOUSEEVENTF_LEFTUP, 0);
    super::overlay::notify_cursor_idle(to_x, to_y);
    sleep(Duration::from_millis(20));
    res
}

/// Safety cleanup: release all potentially stuck mouse buttons and modifier keys
pub fn release_all_input_locks() {
    // Release mouse buttons
    let _ = send_mouse_event(MOUSEEVENTF_LEFTUP, 0);
    let _ = send_mouse_event(MOUSEEVENTF_RIGHTUP, 0);
    let _ = send_mouse_event(MOUSEEVENTF_MIDDLEUP, 0);

    // Release keyboard modifiers
    let modifiers = [VK_SHIFT, VK_CONTROL, VK_MENU, VK_LWIN];
    for vk in modifiers {
        let input = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        unsafe {
            let _ = SendInput(&[input], size_of::<INPUT>() as i32);
        }
    }
}

/// Scroll mouse vertically and/or horizontally
pub fn scroll_mouse(scroll_x: Option<i32>, scroll_y: Option<i32>) -> Result<(), String> {
    if let Some(sy) = scroll_y {
        if sy != 0 {
            send_mouse_event(MOUSEEVENTF_WHEEL, (sy * WHEEL_DELTA) as u32)?;
        }
    }
    if let Some(sx) = scroll_x {
        if sx != 0 {
            send_mouse_event(MOUSEEVENTF_HWHEEL, (sx * WHEEL_DELTA) as u32)?;
        }
    }
    Ok(())
}

fn send_mouse_event(flags: MOUSE_EVENT_FLAGS, data: u32) -> Result<(), String> {
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: data,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };

    let sent = unsafe { SendInput(&[input], size_of::<INPUT>() as i32) };
    if sent != 1 {
        let err = unsafe { windows::Win32::Foundation::GetLastError() };
        // Fallback: If SendInput is blocked by UIPI or in a background runner, try mouse_event
        unsafe {
            windows::Win32::UI::Input::KeyboardAndMouse::mouse_event(flags, 0, 0, data as i32, 0);
        }
        if err.0 == 5 {
            return Ok(());
        }
        return Err(format!("Failed to dispatch mouse input event (sent {sent}/1, error: {err:?})"));
    }
    Ok(())
}

/// Type Unicode text accurately without keyboard layout mismatch
pub fn type_unicode_text(text: &str) -> Result<(), String> {
    for ch in text.chars() {
        if ch == '\n' {
            press_vk(VK_RETURN)?;
            continue;
        }
        if ch == '\r' {
            continue;
        }

        let mut utf16_buf = [0u16; 2];
        let encoded = ch.encode_utf16(&mut utf16_buf);

        for &code_unit in encoded.iter() {
            // Key down
            let input_down = INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VIRTUAL_KEY(0),
                        wScan: code_unit,
                        dwFlags: KEYEVENTF_UNICODE,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            };

            // Key up
            let input_up = INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VIRTUAL_KEY(0),
                        wScan: code_unit,
                        dwFlags: KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            };

            unsafe {
                SendInput(&[input_down, input_up], size_of::<INPUT>() as i32);
            }
        }
        sleep(Duration::from_millis(5));
    }
    Ok(())
}

/// Press a named key (e.g. "Enter", "Tab", "Escape", "Backspace")
pub fn press_key(key_name: &str) -> Result<(), String> {
    let vk = resolve_virtual_key(key_name)
        .ok_or_else(|| format!("Unknown or unsupported key name: '{key_name}'"))?;
    press_vk(vk)
}

fn press_vk(vk: VIRTUAL_KEY) -> Result<(), String> {
    let input_down = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: KEYBD_EVENT_FLAGS(0),
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };

    let input_up = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: KEYEVENTF_KEYUP,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };

    unsafe {
        let sent = SendInput(&[input_down, input_up], size_of::<INPUT>() as i32);
        if sent != 2 {
            let err = windows::Win32::Foundation::GetLastError();
            if err.0 == 5 {
                windows::Win32::UI::Input::KeyboardAndMouse::keybd_event(vk.0 as u8, 0, KEYBD_EVENT_FLAGS(0), 0);
                windows::Win32::UI::Input::KeyboardAndMouse::keybd_event(vk.0 as u8, 0, KEYEVENTF_KEYUP, 0);
                return Ok(());
            }
            return Err(format!("Failed to send key press for VK {vk:?} (sent {sent}/2, error: {err:?})"));
        }
    }
    Ok(())
}

/// Parse and execute validated keyboard shortcuts (e.g. "Ctrl+C", "Ctrl+V", "Alt+Tab", "Win+R")
pub fn execute_shortcut(shortcut: &str) -> Result<(), String> {
    let parts: Vec<&str> = shortcut.split('+').map(|s| s.trim()).collect();
    if parts.is_empty() {
        return Err("Empty shortcut string".to_string());
    }

    let mut modifiers = Vec::new();
    let mut main_key_vk = None;

    for part in parts {
        let lower = part.to_lowercase();
        match lower.as_str() {
            "ctrl" | "control" => modifiers.push(VK_CONTROL),
            "alt" | "menu" => modifiers.push(VK_MENU),
            "shift" => modifiers.push(VK_SHIFT),
            "win" | "windows" | "super" => modifiers.push(VK_LWIN),
            other => {
                if main_key_vk.is_some() {
                    return Err(format!(
                        "Invalid shortcut '{shortcut}': multiple non-modifier keys detected"
                    ));
                }
                main_key_vk = resolve_virtual_key(other);
                if main_key_vk.is_none() {
                    return Err(format!("Unsupported key '{other}' in shortcut '{shortcut}'"));
                }
            }
        }
    }

    let key_vk = main_key_vk.ok_or_else(|| format!("No main key specified in shortcut '{shortcut}'"))?;

    // Press all modifiers down
    for &mod_vk in &modifiers {
        send_vk_state(mod_vk, false)?;
    }
    sleep(Duration::from_millis(15));

    // Press main key down and up
    send_vk_state(key_vk, false)?;
    sleep(Duration::from_millis(20));
    send_vk_state(key_vk, true)?;
    sleep(Duration::from_millis(15));

    // Release all modifiers in reverse order to ensure clean keyboard state
    for &mod_vk in modifiers.iter().rev() {
        let _ = send_vk_state(mod_vk, true);
    }

    Ok(())
}

fn send_vk_state(vk: VIRTUAL_KEY, is_up: bool) -> Result<(), String> {
    let flags = if is_up {
        KEYEVENTF_KEYUP
    } else {
        KEYBD_EVENT_FLAGS(0)
    };

    let input = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };

    unsafe {
        let sent = SendInput(&[input], size_of::<INPUT>() as i32);
        if sent != 1 {
            return Err(format!("Failed to send key state for VK {vk:?}"));
        }
    }
    Ok(())
}

/// Select all content (Ctrl+A)
pub fn select_all() -> Result<(), String> {
    execute_shortcut("Ctrl+A")
}

/// Copy selected content (Ctrl+C)
pub fn copy() -> Result<(), String> {
    execute_shortcut("Ctrl+C")
}

/// Paste clipboard content (Ctrl+V)
pub fn paste() -> Result<(), String> {
    execute_shortcut("Ctrl+V")
}

/// Clear active field (Ctrl+A then Backspace)
pub fn clear_field() -> Result<(), String> {
    select_all()?;
    sleep(Duration::from_millis(20));
    press_vk(VK_BACK)
}

fn resolve_virtual_key(name: &str) -> Option<VIRTUAL_KEY> {
    let lower = name.to_lowercase();
    match lower.as_str() {
        "win" | "windows" | "lwin" => Some(VK_LWIN),
        "enter" | "return" => Some(VK_RETURN),
        "escape" | "esc" => Some(VK_ESCAPE),
        "tab" => Some(VK_TAB),
        "space" => Some(VK_SPACE),
        "backspace" | "back" => Some(VK_BACK),
        "delete" | "del" => Some(VK_DELETE),
        "arrowup" | "up" => Some(VK_UP),
        "arrowdown" | "down" => Some(VK_DOWN),
        "arrowleft" | "left" => Some(VK_LEFT),
        "arrowright" | "right" => Some(VK_RIGHT),
        "home" => Some(VK_HOME),
        "end" => Some(VK_END),
        "pageup" | "pgup" => Some(VK_PRIOR),
        "pagedown" | "pgdn" => Some(VK_NEXT),
        "f1" => Some(VK_F1),
        "f2" => Some(VK_F2),
        "f3" => Some(VK_F3),
        "f4" => Some(VK_F4),
        "f5" => Some(VK_F5),
        "f6" => Some(VK_F6),
        "f7" => Some(VK_F7),
        "f8" => Some(VK_F8),
        "f9" => Some(VK_F9),
        "f10" => Some(VK_F10),
        "f11" => Some(VK_F11),
        "f12" => Some(VK_F12),
        s if s.len() == 1 => {
            let c = s.chars().next()?.to_ascii_uppercase();
            if (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9') {
                Some(VIRTUAL_KEY(c as u16))
            } else {
                None
            }
        }
        _ => None,
    }
}
