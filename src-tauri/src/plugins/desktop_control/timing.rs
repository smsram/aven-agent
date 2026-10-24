use serde::{Deserialize, Serialize};
use std::thread::sleep;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitResult {
    pub condition_met: bool,
    pub condition: String,
    pub elapsed_ms: u64,
    pub timeout_ms: u64,
    pub details: Option<serde_json::Value>,
}

/// Bounded wait for window existence, active state, closure, or UI Automation element appearance
pub fn wait_for_ui(
    condition: &str,
    target_hwnd: Option<i64>,
    title: Option<&str>,
    process_name: Option<&str>,
    element_query: Option<&str>,
    control_type: Option<&str>,
    timeout_ms: Option<u64>,
) -> Result<WaitResult, String> {
    let timeout = timeout_ms.unwrap_or(5000).clamp(200, 30000);
    let start = Instant::now();
    let cond_lower = condition.to_lowercase();

    while start.elapsed().as_millis() < timeout as u128 {
        match cond_lower.as_str() {
            "window_exists" | "window_open" => {
                let windows = super::windows::list_windows(true, None, None).unwrap_or_default();
                let found = windows.iter().find(|w| {
                    if let Some(h) = target_hwnd {
                        if w.hwnd == h {
                            return true;
                        }
                    }
                    if let Some(t) = title {
                        if !t.is_empty() && w.title.to_lowercase().contains(&t.to_lowercase()) {
                            return true;
                        }
                    }
                    if let Some(p) = process_name {
                        if !p.is_empty() && w.process_name.to_lowercase().contains(&p.to_lowercase()) {
                            return true;
                        }
                    }
                    false
                });

                if let Some(w) = found {
                    return Ok(WaitResult {
                        condition_met: true,
                        condition: condition.to_string(),
                        elapsed_ms: start.elapsed().as_millis() as u64,
                        timeout_ms: timeout,
                        details: Some(serde_json::to_value(w).unwrap_or_default()),
                    });
                }
            }
            "window_active" | "window_foreground" => {
                let windows = super::windows::list_windows(true, None, None).unwrap_or_default();
                let active = windows.iter().find(|w| w.is_active);

                if let Some(w) = active {
                    let mut matched = false;
                    if let Some(h) = target_hwnd {
                        if w.hwnd == h {
                            matched = true;
                        }
                    } else if let Some(t) = title {
                        if !t.is_empty() && w.title.to_lowercase().contains(&t.to_lowercase()) {
                            matched = true;
                        }
                    } else if let Some(p) = process_name {
                        if !p.is_empty() && w.process_name.to_lowercase().contains(&p.to_lowercase()) {
                            matched = true;
                        }
                    }

                    if matched {
                        return Ok(WaitResult {
                            condition_met: true,
                            condition: condition.to_string(),
                            elapsed_ms: start.elapsed().as_millis() as u64,
                            timeout_ms: timeout,
                            details: Some(serde_json::to_value(w).unwrap_or_default()),
                        });
                    }
                }
            }
            "window_closed" => {
                let windows = super::windows::list_windows(false, None, None).unwrap_or_default();
                let exists = windows.iter().any(|w| {
                    if let Some(h) = target_hwnd {
                        if w.hwnd == h {
                            return true;
                        }
                    }
                    if let Some(t) = title {
                        if !t.is_empty() && w.title.to_lowercase().contains(&t.to_lowercase()) {
                            return true;
                        }
                    }
                    if let Some(p) = process_name {
                        if !p.is_empty() && w.process_name.to_lowercase().contains(&p.to_lowercase()) {
                            return true;
                        }
                    }
                    false
                });

                if !exists {
                    return Ok(WaitResult {
                        condition_met: true,
                        condition: condition.to_string(),
                        elapsed_ms: start.elapsed().as_millis() as u64,
                        timeout_ms: timeout,
                        details: Some(serde_json::json!({ "status": "closed" })),
                    });
                }
            }
            "element_exists" | "control_exists" => {
                if let Some(q) = element_query {
                    let matches = super::uia::find_elements(target_hwnd, Some(q), control_type, Some(1))
                        .unwrap_or_default();
                    if let Some(first) = matches.first() {
                        return Ok(WaitResult {
                            condition_met: true,
                            condition: condition.to_string(),
                            elapsed_ms: start.elapsed().as_millis() as u64,
                            timeout_ms: timeout,
                            details: Some(serde_json::to_value(first).unwrap_or_default()),
                        });
                    }
                }
            }
            other => {
                return Err(format!(
                    "Unsupported wait condition: '{other}'. Valid conditions: window_exists, window_active, window_closed, element_exists"
                ));
            }
        }

        sleep(Duration::from_millis(150));
    }

    Ok(WaitResult {
        condition_met: false,
        condition: condition.to_string(),
        elapsed_ms: start.elapsed().as_millis() as u64,
        timeout_ms: timeout,
        details: None,
    })
}
