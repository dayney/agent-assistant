use serde::Serialize;
use std::io::Write;
use std::process::{Command, Stdio};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoreHealth {
    pub status: &'static str,
    pub mode: &'static str,
}

#[tauri::command]
fn core_health() -> CoreHealth {
    if core_binary().is_some() {
        CoreHealth {
            status: "ready",
            mode: "sidecar",
        }
    } else {
        CoreHealth {
            status: "unavailable",
            mode: "sidecar",
        }
    }
}

#[tauri::command]
fn get_workspace_snapshot() -> Result<serde_json::Value, String> {
    invoke_core("snapshot")
}

#[tauri::command]
fn preview_apply() -> Result<serde_json::Value, String> {
    invoke_core("preview")
}

fn core_binary() -> Option<std::path::PathBuf> {
    if let Ok(path) = std::env::var("AGENT_ASSISTANT_CORE_BIN") {
        let path = std::path::PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }
    if let Ok(executable) = std::env::current_exe() {
        if let Some(parent) = executable.parent() {
            for name in [
                "agent-assistant-core",
                "agent-assistant-core-aarch64-apple-darwin",
                "agent-assistant-core-x86_64-apple-darwin",
            ] {
                let sibling = parent.join(name);
                if sibling.is_file() {
                    return Some(sibling);
                }
            }
            if let Ok(entries) = std::fs::read_dir(parent) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file()
                        && path
                            .file_name()
                            .and_then(|name| name.to_str())
                            .is_some_and(|name| name.starts_with("agent-assistant-core-"))
                    {
                        return Some(path);
                    }
                }
            }
            if let Some(manifest_dir) = parent.parent().and_then(std::path::Path::parent) {
                let development_sidecar = manifest_dir.join("agent-assistant-core");
                if development_sidecar.is_file() {
                    return Some(development_sidecar);
                }
            }
        }
    }
    which_core_binary()
}

fn which_core_binary() -> Option<std::path::PathBuf> {
    let path = std::env::var_os("PATH")?;
    for directory in std::env::split_paths(&path) {
        let candidate = directory.join("agent-assistant-core");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn invoke_core(method: &str) -> Result<serde_json::Value, String> {
    let binary = core_binary().ok_or_else(|| {
        "core_unavailable: agent-assistant-core was not found; set AGENT_ASSISTANT_CORE_BIN or place the sidecar beside the app".to_string()
    })?;
    let mut child = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("core_spawn_failed: {error}"))?;
    let request = serde_json::json!({"method": method});
    child
        .stdin
        .as_mut()
        .ok_or_else(|| "core_spawn_failed: sidecar stdin unavailable".to_string())?
        .write_all(format!("{request}\n").as_bytes())
        .map_err(|error| format!("core_write_failed: {error}"))?;
    drop(child.stdin.take());
    let output = child
        .wait_with_output()
        .map_err(|error| format!("core_wait_failed: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "core_exit_failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let value: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("core_protocol_failed: {error}"))?;
    if let Some(error) = value.get("error").and_then(serde_json::Value::as_str) {
        return Err(format!("core_read_failed: {error}"));
    }
    Ok(value)
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            core_health,
            get_workspace_snapshot,
            preview_apply
        ])
        .run(tauri::generate_context!())
        .expect("error while running agent-assistant");
}
