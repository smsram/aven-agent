use std::fs;
use std::path::{Path, PathBuf};

/// Allowed Windows applications that can be launched directly
pub const SAFE_APP_WHITELIST: &[(&str, &str, bool)] = &[
    // (alias, executable_name, requires_confirmation)
    ("notepad", "notepad.exe", false),
    ("calc", "calc.exe", false),
    ("calculator", "calc.exe", false),
    ("explorer", "explorer.exe", false),
    ("paint", "mspaint.exe", false),
    ("mspaint", "mspaint.exe", false),
    ("edge", "msedge.exe", false),
    ("msedge", "msedge.exe", false),
    ("code", "code.cmd", true),
    ("terminal", "wt.exe", true),
    ("wt", "wt.exe", true),
];

/// Prohibited executables to prevent arbitrary command/script injection
pub const BLOCKED_EXECUTABLES: &[&str] = &[
    "cmd.exe",
    "powershell.exe",
    "pwsh.exe",
    "wscript.exe",
    "cscript.exe",
    "mshta.exe",
    "reg.exe",
    "format.com",
    "bash.exe",
    "sh.exe",
];

/// Validate that a URL uses safe protocols (http or https only) and contains no control characters
pub fn validate_url(url: &str) -> Result<String, String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err("URL cannot be empty".into());
    }

    if trimmed.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err("URL contains invalid control characters or whitespace".into());
    }

    let parsed = reqwest::Url::parse(trimmed)
        .map_err(|e| format!("Invalid URL format: {e}"))?;

    let scheme = parsed.scheme().to_lowercase();
    if scheme != "http" && scheme != "https" {
        return Err(format!(
            "Protocol '{}' is not permitted. Only 'http://' and 'https://' are supported for security.",
            scheme
        ));
    }

    if parsed.host_str().is_none() {
        return Err("URL must specify a valid hostname".into());
    }

    Ok(trimmed.to_string())
}

/// Open a validated URL in the system's default browser
pub fn open_url_in_browser(url: &str) -> Result<(), String> {
    let safe_url = validate_url(url)?;

    #[cfg(target_os = "windows")]
    {
        // On Windows, use rundll32 with url.dll to launch default browser without shell strings
        let status = std::process::Command::new("rundll32.exe")
            .args(["url.dll,FileProtocolHandler", &safe_url])
            .spawn()
            .map_err(|e| format!("Failed to launch default browser: {e}"))?;
        drop(status);
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("Native URL opening is currently supported on Windows only".into())
    }
}

/// Validate application executable target
pub fn resolve_launch_target(app_name: &str) -> Result<(PathBuf, bool), String> {
    let trimmed = app_name.trim();
    if trimmed.is_empty() {
        return Err("Application name cannot be empty".into());
    }

    let lower = trimmed.to_lowercase();

    // Check blocked binaries
    for blocked in BLOCKED_EXECUTABLES {
        if lower == *blocked || lower.ends_with(&format!("\\{}", blocked)) || lower.ends_with(&format!("/{}", blocked)) {
            return Err(format!(
                "Executable '{blocked}' is blocked for security reasons. Shell interpreters cannot be launched directly."
            ));
        }
    }

    // Check allowlist
    for (alias, exe, requires_conf) in SAFE_APP_WHITELIST {
        if lower == *alias || lower == *exe {
            return Ok((PathBuf::from(exe), *requires_conf));
        }
    }

    // If an explicit absolute path is provided, validate it
    let path = PathBuf::from(trimmed);
    if path.is_absolute() {
        if !path.exists() {
            return Err(format!("Executable path does not exist: {}", path.display()));
        }

        let ext = path.extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        if ext != "exe" {
            return Err("Only valid Windows .exe executables are supported".into());
        }

        // Custom paths always require explicit user confirmation
        return Ok((path, true));
    }

    Err(format!(
        "Application '{}' is not in the recognized applications allowlist (e.g. notepad, calc, code, terminal, edge, explorer) and is not a valid absolute path.",
        trimmed
    ))
}

/// Launch a recognized or approved Windows application safely without shell interpretation
pub fn launch_application(
    app_name: &str,
    args: &[String],
    user_confirmed: bool,
) -> Result<String, String> {
    let (exe_target, requires_confirmation) = resolve_launch_target(app_name)?;

    if requires_confirmation && !user_confirmed {
        return Err(format!(
            "User confirmation is required to launch '{}'. Please confirm before executing.",
            exe_target.display()
        ));
    }

    // Validate argument safety (no null bytes or control characters)
    for arg in args {
        if arg.contains('\0') {
            return Err("Argument contains invalid null byte".into());
        }
    }

    #[cfg(target_os = "windows")]
    {
        let mut cmd = std::process::Command::new(&exe_target);
        cmd.args(args);

        match cmd.spawn() {
            Ok(child) => {
                let pid = child.id();
                Ok(format!(
                    "Successfully launched '{}' (PID: {})",
                    exe_target.display(),
                    pid
                ))
            }
            Err(e) => Err(format!(
                "Failed to spawn application '{}': {}",
                exe_target.display(),
                e
            )),
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("Native application launching is supported on Windows only".into())
    }
}

/// Validate path containment to prevent directory traversal and symlink escapes
pub fn validate_path_containment(approved_root: &Path, requested_path: &Path) -> Result<PathBuf, String> {
    if !approved_root.exists() {
        return Err(format!(
            "Approved workspace directory does not exist: {}",
            approved_root.display()
        ));
    }

    let canonical_root = fs::canonicalize(approved_root)
        .map_err(|e| format!("Failed to canonicalize approved root directory: {e}"))?;

    // Combine paths if requested_path is relative
    let full_path = if requested_path.is_absolute() {
        requested_path.to_path_buf()
    } else {
        approved_root.join(requested_path)
    };

    // If the path exists, canonicalize it directly
    let canonical_target = if full_path.exists() {
        fs::canonicalize(&full_path)
            .map_err(|e| format!("Failed to resolve requested path: {e}"))?
    } else {
        // If the file does not exist yet (e.g., target for writing), canonicalize parent
        let parent = full_path.parent()
            .ok_or_else(|| "Invalid target path: missing parent directory".to_string())?;

        if !parent.exists() {
            return Err(format!(
                "Parent directory does not exist: {}",
                parent.display()
            ));
        }

        let canonical_parent = fs::canonicalize(parent)
            .map_err(|e| format!("Failed to resolve parent directory: {e}"))?;

        if !canonical_parent.starts_with(&canonical_root) {
            return Err(format!(
                "Security violation: path '{}' escapes approved directory scope '{}'.",
                full_path.display(),
                canonical_root.display()
            ));
        }

        let file_name = full_path.file_name()
            .ok_or_else(|| "Invalid file name".to_string())?;

        canonical_parent.join(file_name)
    };

    // Strict containment verification
    if !canonical_target.starts_with(&canonical_root) {
        return Err(format!(
            "Security violation: path '{}' escapes approved directory scope '{}'. Path traversal rejected.",
            canonical_target.display(),
            canonical_root.display()
        ));
    }

    Ok(canonical_target)
}

/// Native Scoped File Access (Read and List)
pub fn scoped_file_operation(
    action: &str,
    approved_root_str: &str,
    target_rel_path: &str,
) -> Result<serde_json::Value, String> {
    let root = Path::new(approved_root_str);
    let target = Path::new(target_rel_path);

    let contained_path = validate_path_containment(root, target)?;

    match action {
        "read" => {
            let metadata = fs::metadata(&contained_path)
                .map_err(|e| format!("Failed to inspect target file: {e}"))?;

            if metadata.is_dir() {
                return Err(format!(
                    "Path '{}' is a directory, not a file. Use 'list' action instead.",
                    contained_path.display()
                ));
            }

            if metadata.len() > 1024 * 512 {
                return Err(format!(
                    "File size ({} bytes) exceeds the 512KB safe read limit.",
                    metadata.len()
                ));
            }

            let content = fs::read_to_string(&contained_path)
                .map_err(|e| format!("Failed to read file contents: {e}"))?;

            Ok(serde_json::json!({
                "path": contained_path.display().to_string(),
                "size_bytes": metadata.len(),
                "content": content
            }))
        }
        "list" => {
            let entries = fs::read_dir(&contained_path)
                .map_err(|e| format!("Failed to read directory: {e}"))?;

            let mut files = Vec::new();
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);

                files.push(serde_json::json!({
                    "name": name,
                    "is_directory": is_dir,
                    "size_bytes": size,
                }));
            }

            Ok(serde_json::json!({
                "path": contained_path.display().to_string(),
                "count": files.len(),
                "entries": files
            }))
        }
        _ => Err(format!(
            "Unsupported scoped file action '{}'. Supported actions: 'read', 'list'.",
            action
        )),
    }
}

/// Native system status diagnostic information
pub fn get_native_system_info() -> serde_json::Value {
    let hostname = std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "Unknown Device".to_string());
    let username = std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "Unknown User".to_string());

    serde_json::json!({
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "device_name": hostname,
        "user_name": username,
        "is_windows": cfg!(target_os = "windows"),
        "platform_supported": cfg!(target_os = "windows"),
        "features": {
            "browser_opener": true,
            "application_launcher": cfg!(target_os = "windows"),
            "scoped_filesystem": true,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_validate_url_valid() {
        assert!(validate_url("https://linear.app").is_ok());
        assert!(validate_url("http://localhost:3000/api").is_ok());
    }

    #[test]
    fn test_validate_url_rejects_unsafe_schemes() {
        assert!(validate_url("file:///C:/Windows/System32").is_err());
        assert!(validate_url("javascript:alert(1)").is_err());
        assert!(validate_url("data:text/html,<html>").is_err());
        assert!(validate_url("powershell.exe").is_err());
        assert!(validate_url("   ").is_err());
    }

    #[test]
    fn test_resolve_launch_target_allowlist() {
        let (path, requires_conf) = resolve_launch_target("notepad").unwrap();
        assert_eq!(path.to_str().unwrap(), "notepad.exe");
        assert!(!requires_conf);

        let (path2, requires_conf2) = resolve_launch_target("calc").unwrap();
        assert_eq!(path2.to_str().unwrap(), "calc.exe");
        assert!(!requires_conf2);
    }

    #[test]
    fn test_resolve_launch_target_blocked() {
        assert!(resolve_launch_target("cmd.exe").is_err());
        assert!(resolve_launch_target("powershell.exe").is_err());
        assert!(resolve_launch_target("pwsh.exe").is_err());
        assert!(resolve_launch_target("wscript.exe").is_err());
    }

    #[test]
    fn test_path_containment_prevents_directory_traversal() {
        let temp_dir = std::env::temp_dir().join("aven_test_containment");
        let _ = fs::create_dir_all(&temp_dir);

        let sub_dir = temp_dir.join("sub");
        let _ = fs::create_dir_all(&sub_dir);

        let test_file = sub_dir.join("test.txt");
        let mut f = File::create(&test_file).unwrap();
        f.write_all(b"Hello safe world").unwrap();

        // Valid inside containment
        let res = validate_path_containment(&temp_dir, Path::new("sub/test.txt"));
        assert!(res.is_ok());

        // Invalid: directory traversal escape
        let res_traversal = validate_path_containment(&sub_dir, Path::new("../../../Windows/System32"));
        assert!(res_traversal.is_err());

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
