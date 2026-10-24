use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;
use std::process::Command;
use std::thread::sleep;
use std::time::{Duration, Instant};

use super::input;
use super::taskbar;
use super::windows;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchAppResult {
    pub success: bool,
    pub reused: bool,
    pub app_name: String,
    pub executable: String,
    pub window_id: Option<i64>,
    pub window_title: Option<String>,
    pub process_id: Option<u32>,
    pub message: String,
}

/// Known standard applications mapped to their system executables and aliases
fn resolve_executable_alias(name: &str) -> Result<String, String> {
    let lower = name.trim().to_lowercase();

    // Security check: strictly block dangerous script and shell interpreters
    let blocked = [
        "cmd", "cmd.exe", "powershell", "powershell.exe", "pwsh", "pwsh.exe", "bash", "bash.exe",
        "wscript", "wscript.exe", "cscript", "cscript.exe", "mshta", "mshta.exe", "regedit",
        "regedit.exe",
    ];
    if blocked.iter().any(|&b| {
        lower == b
            || lower.ends_with(&format!("\\{b}"))
            || lower.ends_with(&format!("/{b}"))
    }) {
        return Err(format!(
            "Command/shell interpreters ('{name}') are strictly blocked from execution"
        ));
    }

    match lower.as_str() {
        "notepad" | "notepad.exe" => Ok("notepad.exe".to_string()),
        "calc" | "calculator" | "calc.exe" => Ok("calc.exe".to_string()),
        "edge" | "msedge" | "msedge.exe" => Ok("msedge.exe".to_string()),
        "chrome" | "chrome.exe" => Ok("chrome.exe".to_string()),
        "firefox" | "firefox.exe" => Ok("firefox.exe".to_string()),
        "explorer" | "files" | "file explorer" | "explorer.exe" => Ok("explorer.exe".to_string()),
        "code" | "vscode" | "visual studio code" => Ok("code".to_string()),
        "terminal" | "wt" | "windows terminal" | "wt.exe" => Ok("wt.exe".to_string()),
        "paint" | "mspaint" | "mspaint.exe" => Ok("mspaint.exe".to_string()),
        "taskmgr" | "task manager" | "taskmgr.exe" => Ok("taskmgr.exe".to_string()),
        other => {
            let p = Path::new(other);
            if p.extension()
                .and_then(|e| e.to_str())
                .map(|s| s.eq_ignore_ascii_case("exe"))
                .unwrap_or(false)
            {
                Ok(other.to_string())
            } else {
                Ok(format!("{other}.exe"))
            }
        }
    }
}

/// Checks whether a process is an internal Windows shell host or background helper
/// that should NOT be mistaken for an launched application window.
pub fn is_internal_system_process(proc_lower: &str, title_lower: &str, query_lower: &str) -> bool {
    proc_lower == "searchhost.exe"
        || proc_lower == "shellexperiencehost.exe"
        || proc_lower == "startmenuexperiencehost.exe"
        || proc_lower == "textinputhost.exe"
        || (proc_lower == "applicationframehost.exe" && title_lower.is_empty())
        || proc_lower == "aven.exe"
        || (proc_lower == "explorer.exe" && !query_lower.contains("explorer") && !query_lower.contains("folder"))
}

/// inspects search results, clicks the best match, and verifies that the window actually opened.
pub fn launch_or_reuse_application(
    app_name: &str,
    args: &[String],
    reuse_existing: bool,
    _user_confirmed: bool,
) -> Result<LaunchAppResult, String> {
    let trimmed = app_name.trim();
    if trimmed.is_empty() {
        return Err("Application name cannot be empty".to_string());
    }

    let exe = resolve_executable_alias(trimmed)?;

    // 1. If reuse_existing is requested (default true), search for an existing visible window
    if reuse_existing {
        if let Ok(open_wins) = windows::list_windows(true, None, None) {
            let app_ident = Path::new(&exe)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(trimmed)
                .to_lowercase();
            let query_lower = trimmed.to_lowercase();

            let existing = open_wins.iter().find(|w| {
                let proc_lower = w.process_name.to_lowercase();
                let title_lower = w.title.to_lowercase();
                proc_lower.contains(&app_ident)
                    || proc_lower.contains(&query_lower)
                    || title_lower.contains(&query_lower)
            });

            if let Some(target) = existing {
                let _ = windows::focus_window(target.hwnd);
                return Ok(LaunchAppResult {
                    success: true,
                    reused: true,
                    app_name: trimmed.to_string(),
                    executable: exe,
                    window_id: Some(target.hwnd),
                    window_title: Some(target.title.clone()),
                    process_id: Some(target.process_id),
                    message: format!(
                        "Reused existing window for '{}' (PID {}, HWND {})",
                        target.title, target.process_id, target.hwnd
                    ),
                });
            }
        }
    }

    // 2. Snapshot current open windows to accurately verify new window creation
    let before_windows: HashSet<i64> = windows::list_windows(false, None, None)
        .unwrap_or_default()
        .into_iter()
        .map(|w| w.hwnd)
        .collect();

    // 3. Open Windows Start Menu using Desktop & UI Control interactions
    let _ = taskbar::ensure_taskbar_visible();

    // Press Windows Key to open Start Menu
    let win_press_res = input::press_key("win");
    if win_press_res.is_err() {
        // Fallback: click Start button coordinates
        if let Ok((sx, sy)) = taskbar::find_start_button_coordinates() {
            let _ = input::click_mouse("left", Some(sx), Some(sy));
        }
    }

    // Wait for Start Menu / Search UI to appear
    sleep(Duration::from_millis(350));

    // 4. Type the application name into Start Menu search
    input::type_unicode_text(trimmed)?;

    // Allow Windows Search to populate index results
    sleep(Duration::from_millis(500));

    // 5. Inspect search results via UI Automation or confirm top match
    let mut clicked_result = false;
    if let Ok(search_results) = super::uia::find_elements(
        None,
        Some(trimmed),
        None,
        None,
    ) {
        if let Some(res) = search_results.first() {
            if let Some(ref r) = res.bounding_rect {
                if r.width > 0 && r.height > 0 {
                    let cx = r.x + r.width / 2;
                    let cy = r.y + r.height / 2;
                    let _ = input::click_mouse("left", Some(cx), Some(cy));
                    clicked_result = true;
                }
            }
        }
    }

    if !clicked_result {
        // Send Enter key to launch the top Best Match result
        input::press_key("enter")?;
    }

    // 6. Bounded Verification Wait: Verify that the application actually opened
    // Poll up to 6 seconds for the new window to be created and become visible
    let start_wait = Instant::now();
    let timeout = Duration::from_millis(6000);
    let app_stem = Path::new(&exe)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(trimmed)
        .to_lowercase();
    let query_lower = trimmed.to_lowercase();

    let mut verified_window = None;

    while start_wait.elapsed() < timeout {
        sleep(Duration::from_millis(300));

        if let Ok(current_windows) = windows::list_windows(true, None, None) {
            // First look for a newly appeared window that wasn't open before
            for win in &current_windows {
                let proc_lower = win.process_name.to_lowercase();
                let title_lower = win.title.to_lowercase();

                if is_internal_system_process(&proc_lower, &title_lower, &query_lower) {
                    continue;
                }

                let matches_name = proc_lower.contains(&app_stem)
                    || proc_lower.contains(&query_lower)
                    || title_lower.contains(&app_stem)
                    || title_lower.contains(&query_lower);

                let is_new_app_window = !before_windows.contains(&win.hwnd)
                    && !win.title.trim().is_empty()
                    && title_lower != "start"
                    && title_lower != "search"
                    && title_lower != "task view"
                    && title_lower != "program manager";

                if (matches_name || is_new_app_window) && win.is_visible && win.rect.width > 100 && win.rect.height > 100 {
                    verified_window = Some(win.clone());
                    break;
                }
            }

            if verified_window.is_some() {
                break;
            }
        }
    }

    // 7. Verification outcome enforcement: Never claim success when it did not open!
    if let Some(target) = verified_window {
        // Allow the app window to finish initializing, then bring to foreground
        sleep(Duration::from_millis(250));
        let _ = windows::focus_window(target.hwnd);
        sleep(Duration::from_millis(150));
        Ok(LaunchAppResult {
            success: true,
            reused: false,
            app_name: trimmed.to_string(),
            executable: exe,
            window_id: Some(target.hwnd),
            window_title: Some(target.title.clone()),
            process_id: Some(target.process_id),
            message: format!(
                "Application '{}' successfully opened via Windows Start Menu and verified (window '{}', HWND {})",
                trimmed, target.title, target.hwnd
            ),
        })
    } else {
        // If an explicit absolute .exe path was supplied and Start menu didn't pick it up,
        // attempt direct launch as secondary fallback
        let direct_path = Path::new(trimmed);
        if direct_path.is_absolute() && direct_path.exists() {
            let mut cmd = Command::new(direct_path);
            for arg in args {
                cmd.arg(arg);
            }
            if let Ok(child) = cmd.spawn() {
                let pid = child.id();
                sleep(Duration::from_millis(800));
                if let Ok(current) = windows::list_windows(true, None, None) {
                    if let Some(w) = current.iter().find(|w| w.process_id == pid) {
                        let _ = windows::focus_window(w.hwnd);
                        return Ok(LaunchAppResult {
                            success: true,
                            reused: false,
                            app_name: trimmed.to_string(),
                            executable: trimmed.to_string(),
                            window_id: Some(w.hwnd),
                            window_title: Some(w.title.clone()),
                            process_id: Some(pid),
                            message: format!(
                                "Application '{}' opened and verified (PID {}, HWND {})",
                                trimmed, pid, w.hwnd
                            ),
                        });
                    }
                }
            }
        }

        // Clean up the search overlay with Escape so screen is not left obscured
        let _ = input::press_key("escape");

        Err(format!(
            "Failed to open '{}': verified that no application window appeared after Start Menu search within the verification window. The application may not be installed or requires manual administrator approval.",
            trimmed
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_executable_alias_standard_apps() {
        assert_eq!(resolve_executable_alias("notepad").unwrap(), "notepad.exe");
        assert_eq!(resolve_executable_alias("calc").unwrap(), "calc.exe");
        assert_eq!(resolve_executable_alias("edge").unwrap(), "msedge.exe");
        assert_eq!(resolve_executable_alias("chrome").unwrap(), "chrome.exe");
        assert_eq!(resolve_executable_alias("explorer").unwrap(), "explorer.exe");
    }

    #[test]
    fn test_resolve_executable_alias_blocks_command_interpreters() {
        assert!(resolve_executable_alias("cmd").is_err());
        assert!(resolve_executable_alias("powershell").is_err());
        assert!(resolve_executable_alias("pwsh.exe").is_err());
        assert!(resolve_executable_alias("bash").is_err());
        assert!(resolve_executable_alias("regedit").is_err());
    }

    #[test]
    fn test_is_internal_system_process_filters_shell_hosts() {
        assert!(is_internal_system_process("searchhost.exe", "", "notepad"));
        assert!(is_internal_system_process("shellexperiencehost.exe", "", "notepad"));
        assert!(is_internal_system_process("startmenuexperiencehost.exe", "", "notepad"));
        assert!(is_internal_system_process("textinputhost.exe", "", "notepad"));
        assert!(is_internal_system_process("aven.exe", "Aven", "notepad"));
        // Real app processes should NOT be filtered
        assert!(!is_internal_system_process("notepad.exe", "Untitled - Notepad", "notepad"));
        assert!(!is_internal_system_process("whatsapp.exe", "WhatsApp", "whatsapp"));
        assert!(!is_internal_system_process("code.exe", "Visual Studio Code", "code"));
    }
}

