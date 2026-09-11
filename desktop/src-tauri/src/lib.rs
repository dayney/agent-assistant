use serde::Serialize;
use std::ffi::OsString;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use tauri_plugin_shell::{process::CommandEvent, ShellExt};

const CORE_SIDECAR: &str = "agent-assistant-core";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoreHealth {
    pub status: &'static str,
    pub mode: &'static str,
}

#[tauri::command]
async fn core_health(app: tauri::AppHandle) -> CoreHealth {
    if invoke_core(&app, "health", serde_json::Value::Null)
        .await
        .is_ok()
    {
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
async fn get_workspace_snapshot(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    invoke_core(&app, "snapshot", serde_json::Value::Null).await
}

#[tauri::command]
async fn preview_apply(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    invoke_core(&app, "preview", serde_json::Value::Null).await
}

#[tauri::command]
async fn rules_get(
    app: tauri::AppHandle,
    request: serde_json::Value,
) -> Result<serde_json::Value, String> {
    invoke_core(&app, "rules_get", request).await
}

#[tauri::command]
async fn rules_save(
    app: tauri::AppHandle,
    request: serde_json::Value,
) -> Result<serde_json::Value, String> {
    invoke_core(&app, "rules_save", request).await
}

#[tauri::command]
async fn rules_sync(
    app: tauri::AppHandle,
    request: serde_json::Value,
) -> Result<serde_json::Value, String> {
    invoke_core(&app, "rules_sync", request).await
}

#[tauri::command]
async fn rules_import_native(
    app: tauri::AppHandle,
    request: serde_json::Value,
) -> Result<serde_json::Value, String> {
    invoke_core(&app, "rules_import_native", request).await
}

#[tauri::command]
async fn project_import(
    app: tauri::AppHandle,
    request: serde_json::Value,
) -> Result<serde_json::Value, String> {
    invoke_core(&app, "project_import", request).await
}

#[tauri::command]
async fn project_analyze_rules(
    app: tauri::AppHandle,
    request: serde_json::Value,
) -> Result<serde_json::Value, String> {
    invoke_core(&app, "project_analyze_rules", request).await
}

fn development_core_binary() -> Option<PathBuf> {
    if !cfg!(debug_assertions) {
        return None;
    }
    development_core_binary_for(true, std::env::var_os("AGENT_ASSISTANT_CORE_BIN"))
}

fn development_core_binary_for(enabled: bool, value: Option<OsString>) -> Option<PathBuf> {
    if !enabled {
        return None;
    }
    value.map(PathBuf::from).filter(|path| path.is_file())
}

async fn invoke_core(
    app: &tauri::AppHandle,
    method: &str,
    payload: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let request = encode_core_request(method, payload)?;
    if let Some(binary) = development_core_binary() {
        return invoke_development_core(binary, &request);
    }
    invoke_bundled_core(app, &request).await
}

fn invoke_development_core(binary: PathBuf, request: &[u8]) -> Result<serde_json::Value, String> {
    let mut child = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("core_spawn_failed: {error}"))?;
    child
        .stdin
        .as_mut()
        .ok_or_else(|| "core_spawn_failed: sidecar stdin unavailable".to_string())?
        .write_all(request)
        .map_err(|error| format!("core_write_failed: {error}"))?;
    drop(child.stdin.take());
    let output = child
        .wait_with_output()
        .map_err(|error| format!("core_wait_failed: {error}"))?;
    decode_core_response(output.status.success(), &output.stdout, &output.stderr)
}

async fn invoke_bundled_core(
    app: &tauri::AppHandle,
    request: &[u8],
) -> Result<serde_json::Value, String> {
    let command = app
        .shell()
        .sidecar(CORE_SIDECAR)
        .map_err(|error| format!("core_unavailable: {error}"))?;
    let (mut events, mut child) = command
        .spawn()
        .map_err(|error| format!("core_spawn_failed: {error}"))?;
    child
        .write(request)
        .map_err(|error| format!("core_write_failed: {error}"))?;
    drop(child);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let mut successful = false;
    let mut event_failed = false;
    while let Some(event) = events.recv().await {
        match event {
            CommandEvent::Stdout(line) => {
                stdout.extend(line);
                stdout.push(b'\n');
            }
            CommandEvent::Stderr(line) => {
                stderr.extend(line);
                stderr.push(b'\n');
            }
            CommandEvent::Error(error) => {
                event_failed = true;
                stderr.extend(error.as_bytes());
            }
            CommandEvent::Terminated(payload) => successful = payload.code == Some(0),
            _ => {}
        }
    }
    decode_core_response(successful && !event_failed, &stdout, &stderr)
}

fn encode_core_request(method: &str, payload: serde_json::Value) -> Result<Vec<u8>, String> {
    let request = if payload.is_null() {
        serde_json::json!({"method": method})
    } else {
        serde_json::json!({"method": method, "payload": payload})
    };
    let mut encoded =
        serde_json::to_vec(&request).map_err(|error| format!("core_protocol_failed: {error}"))?;
    encoded.push(b'\n');
    Ok(encoded)
}

fn decode_core_response(
    successful: bool,
    stdout: &[u8],
    stderr: &[u8],
) -> Result<serde_json::Value, String> {
    if !successful {
        let detail = String::from_utf8_lossy(stderr);
        let detail = detail.trim();
        let detail = if detail.is_empty() {
            "sidecar exited without an error message"
        } else {
            detail
        };
        return Err(format!("core_exit_failed: {detail}"));
    }
    let value: serde_json::Value =
        serde_json::from_slice(stdout).map_err(|error| format!("core_protocol_failed: {error}"))?;
    if let Some(error) = value.get("error").and_then(serde_json::Value::as_str) {
        return Err(format!("core_read_failed: {error}"));
    }
    Ok(value)
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            core_health,
            get_workspace_snapshot,
            preview_apply,
            rules_get,
            rules_save,
            rules_sync,
            rules_import_native,
            project_import,
            project_analyze_rules
        ])
        .run(tauri::generate_context!())
        .expect("error while running agent-assistant");
}

#[cfg(test)]
mod tests {
    use super::{decode_core_response, development_core_binary_for, encode_core_request};
    use std::ffi::OsString;

    #[test]
    fn core_request_is_one_json_line() {
        let request = encode_core_request("snapshot", serde_json::Value::Null).unwrap();
        assert_eq!(request, b"{\"method\":\"snapshot\"}\n");
    }

    #[test]
    fn core_request_includes_non_null_payload() {
        let request =
            encode_core_request("rules_get", serde_json::json!({"scope": "global"})).unwrap();
        assert_eq!(
            request,
            b"{\"method\":\"rules_get\",\"payload\":{\"scope\":\"global\"}}\n"
        );
    }

    #[test]
    fn nonzero_core_exit_uses_stderr() {
        let error = decode_core_response(false, b"", b"permission denied\n").unwrap_err();
        assert_eq!(error, "core_exit_failed: permission denied");
    }

    #[test]
    fn invalid_core_json_is_a_protocol_error() {
        let error = decode_core_response(true, b"not-json", b"").unwrap_err();
        assert!(error.starts_with("core_protocol_failed:"), "{error}");
    }

    #[test]
    fn core_error_field_is_not_returned_as_success() {
        let error =
            decode_core_response(true, br#"{"error":"source unavailable"}"#, b"").unwrap_err();
        assert_eq!(error, "core_read_failed: source unavailable");
    }

    #[test]
    fn valid_core_json_is_returned() {
        let value = decode_core_response(true, br#"{"schemaVersion":1}"#, b"").unwrap();
        assert_eq!(value["schemaVersion"], 1);
    }

    #[test]
    fn release_mode_ignores_the_development_binary_override() {
        let current_executable = std::env::current_exe().unwrap();
        let value = Some(OsString::from(current_executable));
        assert_eq!(development_core_binary_for(false, value), None);
    }
}
