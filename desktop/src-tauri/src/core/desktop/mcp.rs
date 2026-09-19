use super::mcp_adapter::{resolve_target, McpNativeFormat};
use super::model::{
    McpPromotionResult, NativeMcpPromotionRequest, ProjectMcpPromotionRequest,
    PushMcpToAgentRequest, PushMcpToAgentResult, SaveMcpRequest, SaveMcpResult,
    VerifyMcpInAgentRequest, VerifyMcpInAgentResult,
};
use super::rules::validate_project_root;
use super::snapshot::parse_native_mcp_body;
use super::Core;
use crate::core::{iox, mcp, paths, source};
use serde_json::{json, Map as JsonMap, Value};
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

impl Core {
    pub(crate) fn promote_project_mcp(
        &self,
        request: ProjectMcpPromotionRequest,
    ) -> Result<McpPromotionResult, String> {
        let project = validate_project_root(&request.project_path)?;
        if request.mcp_id.trim().is_empty() {
            return Err("MCP id is required".to_string());
        }
        self.with_lock(|| {
            let project_home = paths::project_home(&project);
            let project_canonical = source::load(&project_home)
                .map_err(|error| format!("load project MCP source: {error}"))?;
            let server = project_canonical
                .mcp_servers
                .iter()
                .find(|server| server.id == request.mcp_id)
                .ok_or_else(|| format!("project MCP {} was not found", request.mcp_id))?;
            self.promote_server(server)
        })
    }

    pub(crate) fn promote_native_mcp(
        &self,
        request: NativeMcpPromotionRequest,
    ) -> Result<McpPromotionResult, String> {
        if request.agent.trim().is_empty() || request.mcp_id.trim().is_empty() {
            return Err("native MCP agent and id are required".to_string());
        }
        self.with_lock(|| {
            let (spec, target) = resolve_target(&self.opts.native_root, &request.agent)?;
            let parser_agent = if spec.format == McpNativeFormat::Toml {
                "codex"
            } else if matches!(
                request.agent.as_str(),
                "antigravity" | "antigravity-ide" | "antigravity-cli"
            ) {
                "antigravity"
            } else {
                request.agent.as_str()
            };
            let body = fs::read_to_string(&target.physical_path)
                .map_err(|error| format!("read native MCP source: {error}"))?;
            let mut server = parse_native_mcp_body(parser_agent, &body)?
                .into_iter()
                .find(|server| server.id == request.mcp_id)
                .ok_or_else(|| format!("native MCP {} was not found", request.mcp_id))?;
            mcp::sanitize_imported_credentials(&mut server);
            self.promote_server(&server)
        })
    }

    /// 强制从原生配置文件导入 MCP 到母版。
    /// 与 `promote_native_mcp` 的区别：
    /// - 当母版中已存在同名 MCP 但内容不同时，**强制覆盖**（而非返回错误）。
    /// - 保留母版中已有的 description/recipe 等语义字段，仅更新运行时字段（command/url/args/env）。
    pub(crate) fn promote_native_mcp_force(
        &self,
        request: NativeMcpPromotionRequest,
    ) -> Result<McpPromotionResult, String> {
        if request.agent.trim().is_empty() || request.mcp_id.trim().is_empty() {
            return Err("native MCP agent and id are required".to_string());
        }
        self.with_lock(|| {
            let (spec, target) = resolve_target(&self.opts.native_root, &request.agent)?;
            let parser_agent = if spec.format == McpNativeFormat::Toml {
                "codex"
            } else if matches!(
                request.agent.as_str(),
                "antigravity" | "antigravity-ide" | "antigravity-cli"
            ) {
                "antigravity"
            } else {
                request.agent.as_str()
            };
            let body = fs::read_to_string(&target.physical_path)
                .map_err(|error| format!("read native MCP source: {error}"))?;
            let mut server = parse_native_mcp_body(parser_agent, &body)?
                .into_iter()
                .find(|server| server.id == request.mcp_id)
                .ok_or_else(|| format!("native MCP {} was not found", request.mcp_id))?;
            mcp::sanitize_imported_credentials(&mut server);
            self.promote_server_force(&server)
        })
    }

    fn promote_server(&self, server: &source::McpServer) -> Result<McpPromotionResult, String> {
        mcp::validate_global_promotion(server)?;
        let global = source::load(&self.opts.home)
            .map_err(|error| format!("load global MCP source: {error}"))?;
        if let Some(existing) = global
            .mcp_servers
            .iter()
            .find(|existing| existing.id == server.id)
        {
            if existing == server {
                return Ok(McpPromotionResult {
                    id: server.id.clone(),
                    path: self.mcp_path(&server.id),
                    status: "already-global".to_string(),
                });
            }
            return Err(format!(
                "global MCP {} already exists with different configuration",
                server.id
            ));
        }
        source::write_mcp_server(&self.opts.home, server)?;
        Ok(McpPromotionResult {
            id: server.id.clone(),
            path: self.mcp_path(&server.id),
            status: "promoted".to_string(),
        })
    }

    /// 强制写入母版，母版已存在时合并运行时字段。
    fn promote_server_force(
        &self,
        server: &source::McpServer,
    ) -> Result<McpPromotionResult, String> {
        mcp::validate_global_promotion(server)?;
        let global = source::load(&self.opts.home)
            .map_err(|error| format!("load global MCP source: {error}"))?;
        if let Some(existing) = global
            .mcp_servers
            .iter()
            .find(|existing| existing.id == server.id)
        {
            if existing == server {
                return Ok(McpPromotionResult {
                    id: server.id.clone(),
                    path: self.mcp_path(&server.id),
                    status: "already-global".to_string(),
                });
            }
            // 强制覆盖：保留语义字段，更新运行时字段
            let mut merged = existing.clone();
            if !server.command.is_empty() {
                merged.command = server.command.clone();
            }
            if !server.url.is_empty() {
                merged.url = server.url.clone();
            }
            if !server.args.is_empty() {
                merged.args = server.args.clone();
            }
            for (key, value) in &server.env {
                merged.env.insert(key.clone(), value.clone());
            }
            source::write_mcp_server(&self.opts.home, &merged)
                .map_err(|error| format!("force-write global MCP {}: {error}", server.id))?;
            return Ok(McpPromotionResult {
                id: server.id.clone(),
                path: self.mcp_path(&server.id),
                status: "force-imported".to_string(),
            });
        }
        // 母版中不存在，直接新建
        source::write_mcp_server(&self.opts.home, server)?;
        Ok(McpPromotionResult {
            id: server.id.clone(),
            path: self.mcp_path(&server.id),
            status: "promoted".to_string(),
        })
    }

    fn mcp_path(&self, id: &str) -> String {
        self.opts
            .home
            .join("mcp")
            .join(format!("{id}.toml"))
            .to_string_lossy()
            .to_string()
    }

    pub(crate) fn save_global_mcp(&self, request: SaveMcpRequest) -> Result<SaveMcpResult, String> {
        let id = request.mcp_id.trim();
        if id.is_empty() {
            return Err("mcp_id is required".to_string());
        }
        self.with_lock(|| {
            let global = source::load(&self.opts.home)
                .map_err(|error| format!("load global MCP source: {error}"))?;
            // 找到存量历史文件，保留未暴露字段（args/headers/runner/package 等）
            let mut server = global
                .mcp_servers
                .into_iter()
                .find(|s| s.id == id)
                .ok_or_else(|| format!("global MCP {id} not found"))?;
            // 覆写可编辑字段
            server.name = request.name;
            server.description = request.description;
            server.recipe = request.recipe;
            if !request.url.is_empty() {
                server.url = request.url;
            }
            if !request.command.is_empty() {
                server.command = request.command;
            }
            // 更新凭据：仅更改请求中显式传入的 key，不范围内的 key 保持不变
            for (key, value) in &request.env {
                server.env.insert(key.clone(), value.clone());
            }
            // 🔴 后端安全门：凭据字段不允许保存明文值。
            // 必须使用 ${secret:X} 或 ${env:X} 引用形式。前端应同步阻断，
            // 但后端校验是最终防线，确保 canonical 文件永远不含敏感明文。
            if mcp::credential_state(&server) == "plaintext-blocked" {
                return Err(format!(
                    "refusing to save MCP {id}: secret-like credential fields must use \
                     ${{secret:…}} or ${{env:…}} references, not plaintext values"
                ));
            }
            source::write_mcp_server(&self.opts.home, &server)
                .map_err(|error| format!("write global MCP {id}: {error}"))?;
            Ok(SaveMcpResult {
                id: id.to_string(),
                path: self.mcp_path(id),
            })
        })
    }

    /// 将全局 MCP 写入指定 Agent 的原生 JSON 配置文件（mcpServers 格式）。
    /// 将全局 MCP 写入指定 Agent 的原生配置文件。
    /// - JSON 格式 Agent（cursor/antigravity/gemini 等）：写入 mcpServers[id]
    /// - Codex：写入 TOML mcp_servers[id]
    pub(crate) fn push_mcp_to_agent(
        &self,
        request: PushMcpToAgentRequest,
    ) -> Result<PushMcpToAgentResult, String> {
        let id = request.mcp_id.trim();
        let agent = request.agent.trim();
        if id.is_empty() || agent.is_empty() {
            return Err("mcp_id and agent are required".to_string());
        }
        let (spec, target) = resolve_target(&self.opts.native_root, agent)?;
        self.with_lock(|| {
            let canonical =
                source::load(&self.opts.home).map_err(|e| format!("load canonical: {e}"))?;
            let server = canonical
                .mcp_servers
                .iter()
                .find(|s| s.id == id)
                .ok_or_else(|| format!("global MCP {id} not found"))?;

            let (was_present, body) = match spec.format {
                McpNativeFormat::Toml => {
                    let mut root = read_toml_config(&target.physical_path)?;
                    let was_present = merge_toml_mcp_entry(&mut root, id, server)?;
                    let body = toml::to_string_pretty(&root).map_err(|error| {
                        format!("serialize {}: {error}", target.physical_path.display())
                    })?;
                    (was_present, body)
                }
                McpNativeFormat::Json => {
                    let mut root = read_json_config(&target.physical_path)?;
                    let was_present = merge_json_mcp_entry(&mut root, id, server)?;
                    let body = serde_json::to_string_pretty(&root).map_err(|error| {
                        format!("serialize {}: {error}", target.physical_path.display())
                    })?;
                    (was_present, body)
                }
            };

            if let Some(parent) = target.physical_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|error| format!("create dir {}: {error}", parent.display()))?;
            }
            iox::atomic_write(&target.physical_path, body.as_bytes())
                .map_err(|error| format!("write {}: {error}", target.physical_path.display()))?;

            Ok(PushMcpToAgentResult {
                agent: agent.to_string(),
                path: format!("~/{}", target.logical_relative),
                physical_path: format!("~/{}", target.physical_relative),
                shared_target_id: target.shared_target_id,
                status: if was_present { "updated" } else { "written" }.to_string(),
            })
        })
    }

    /// 检查某个 MCP 是否已写入指定 Agent 的原生配置文件。
    pub(crate) fn verify_mcp_in_agent(
        &self,
        request: VerifyMcpInAgentRequest,
    ) -> Result<VerifyMcpInAgentResult, String> {
        let id = request.mcp_id.trim();
        let agent = request.agent.trim();
        if id.is_empty() || agent.is_empty() {
            return Err("mcp_id and agent are required".to_string());
        }
        let (spec, target) = resolve_target(&self.opts.native_root, agent)?;
        let path_str = format!("~/{}", target.logical_relative);
        let physical_path_str = format!("~/{}", target.physical_relative);
        let empty_result = |status: &str, runtime_status: &str| VerifyMcpInAgentResult {
            agent: agent.to_string(),
            path: path_str.clone(),
            physical_path: physical_path_str.clone(),
            shared_target_id: target.shared_target_id.clone(),
            mismatched_fields: Vec::new(),
            runtime_status: runtime_status.to_string(),
            status: status.to_string(),
        };
        if !target.physical_path.exists() {
            return Ok(VerifyMcpInAgentResult {
                ..empty_result("file-missing", "not-run")
            });
        }
        let body = match fs::read_to_string(&target.physical_path) {
            Ok(b) => b,
            Err(error) => return Err(format!("read {}: {error}", target.physical_path.display())),
        };
        let canonical =
            source::load(&self.opts.home).map_err(|error| format!("load canonical: {error}"))?;
        let expected = canonical
            .mcp_servers
            .iter()
            .find(|server| server.id == id)
            .ok_or_else(|| format!("global MCP {id} not found"))?;
        let parser_agent = if spec.format == McpNativeFormat::Toml {
            "codex"
        } else {
            agent
        };
        let actual = match parse_native_mcp_body(parser_agent, &body) {
            Ok(servers) => match servers.into_iter().find(|server| server.id == id) {
                Some(server) => server,
                None => return Ok(empty_result("missing", "not-run")),
            },
            Err(_) => return Ok(empty_result("parse-error", "not-run")),
        };
        let mismatched_fields = mcp_mismatched_fields(expected, &actual);
        let status = if mismatched_fields.is_empty() {
            "found"
        } else {
            "mismatch"
        };
        let runtime_status = if status == "found" && request.runtime_check {
            // Native config is the runtime source of truth. This lets an
            // Agent keep its local credential while canonical stores only a
            // secret reference.
            runtime_check(&actual)
        } else if request.runtime_check {
            "not-run-mismatch".to_string()
        } else {
            "not-requested".to_string()
        };
        Ok(VerifyMcpInAgentResult {
            agent: agent.to_string(),
            path: path_str,
            physical_path: physical_path_str,
            shared_target_id: target.shared_target_id,
            mismatched_fields,
            runtime_status,
            status: status.to_string(),
        })
    }
}

fn read_json_config(path: &std::path::Path) -> Result<Value, String> {
    if !path.exists() {
        return Ok(json!({ "mcpServers": {} }));
    }
    let body =
        fs::read_to_string(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    json5::from_str(&body).map_err(|error| format!("parse {}: {error}", path.display()))
}

fn read_toml_config(path: &std::path::Path) -> Result<toml::Value, String> {
    if !path.exists() {
        return Ok(toml::Value::Table(toml::map::Map::new()));
    }
    let body =
        fs::read_to_string(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    body.parse::<toml::Value>()
        .map_err(|error| format!("parse {}: {error}", path.display()))
}

/// canonical 的 `command` / `url` 字段可能是 `safe_endpoint()` 生成的展示占位符。
/// 此函数将其解析为可写入原生配置的真实值：
/// - 若 canonical 值不是展示占位符 → 直接使用 canonical 值（空字符串也允许，用于删除字段）。
/// - 若 canonical 值是展示占位符，且原生文件已有真实值 → 保留原生值，不覆盖。
/// - 若 canonical 值是展示占位符，且原生文件也没有真实值 → 返回错误，拒绝写入。
fn resolve_executable_field(
    field: &str,
    mcp_id: &str,
    canonical_value: &str,
    native_value: &str,
) -> Result<String, String> {
    if !mcp::is_display_placeholder(canonical_value) {
        // canonical 是真实值（或空），直接使用
        return Ok(canonical_value.to_string());
    }
    // canonical 是展示占位符
    if !native_value.is_empty() && !mcp::is_display_placeholder(native_value) {
        // 原生文件已有真实可执行值，保留它
        return Ok(native_value.to_string());
    }
    // 原生也没有真实值，无法安全写入
    Err(format!(
        "cannot push MCP {mcp_id}: canonical `{field}` is a display placeholder \
         (\"{canonical_value}\") and no executable value exists in the native config. \
         Fix the canonical {field} before pushing."
    ))
}

fn merge_json_mcp_entry(
    root: &mut Value,
    id: &str,
    server: &source::McpServer,
) -> Result<bool, String> {
    let root_object = root
        .as_object_mut()
        .ok_or_else(|| "native JSON root must be an object".to_string())?;
    let servers = root_object
        .entry("mcpServers".to_string())
        .or_insert_with(|| Value::Object(JsonMap::new()))
        .as_object_mut()
        .ok_or_else(|| "mcpServers must be an object".to_string())?;
    let was_present = servers.contains_key(id);
    let mut entry = servers
        .get(id)
        .cloned()
        .unwrap_or_else(|| Value::Object(JsonMap::new()))
        .as_object()
        .cloned()
        .ok_or_else(|| format!("mcpServers.{id} must be an object"))?;
    // 🔴 展示占位符安全检查：若 canonical command/url 是 safe_endpoint() 生成的脱敏字符串，
    // 不能将其写入原生运行配置。应保留原生已有的真实可执行值；若原生也没有真实值，则拒绝写入。
    let effective_command = resolve_executable_field(
        "command",
        id,
        &server.command,
        entry.get("command").and_then(Value::as_str).unwrap_or(""),
    )?;
    let effective_url = resolve_executable_field(
        "url",
        id,
        &server.url,
        entry.get("url").and_then(Value::as_str).unwrap_or(""),
    )?;
    set_json_string(&mut entry, "command", &effective_command);
    set_json_string(&mut entry, "url", &effective_url);
    set_json_array(&mut entry, "args", &server.args);
    set_json_map(&mut entry, "env", &server.env);
    set_json_map(&mut entry, "headers", &server.headers);
    servers.insert(id.to_string(), Value::Object(entry));
    Ok(was_present)
}

fn set_json_string(entry: &mut JsonMap<String, Value>, key: &str, value: &str) {
    if value.is_empty() {
        entry.remove(key);
    } else {
        entry.insert(key.to_string(), Value::String(value.to_string()));
    }
}

fn set_json_array(entry: &mut JsonMap<String, Value>, key: &str, values: &[String]) {
    if values.is_empty() {
        entry.remove(key);
    } else {
        entry.insert(
            key.to_string(),
            Value::Array(values.iter().cloned().map(Value::String).collect()),
        );
    }
}

fn set_json_map(
    entry: &mut JsonMap<String, Value>,
    key: &str,
    values: &std::collections::BTreeMap<String, String>,
) {
    if values.is_empty() {
        entry.remove(key);
    } else {
        let existing = entry
            .get(key)
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        entry.insert(
            key.to_string(),
            Value::Object(preserve_json_credentials(values, &existing)),
        );
    }
}

fn preserve_json_credentials(
    values: &std::collections::BTreeMap<String, String>,
    existing: &JsonMap<String, Value>,
) -> JsonMap<String, Value> {
    values
        .iter()
        .map(|(key, value)| {
            let value = if mcp::is_secret_ref(value) {
                existing
                    .get(key)
                    .and_then(Value::as_str)
                    .filter(|existing| !existing.is_empty() && !mcp::is_secret_ref(existing))
                    .unwrap_or(value)
            } else {
                value
            };
            (key.clone(), Value::String(value.to_string()))
        })
        .collect()
}

fn merge_toml_mcp_entry(
    root: &mut toml::Value,
    id: &str,
    server: &source::McpServer,
) -> Result<bool, String> {
    let root_table = root
        .as_table_mut()
        .ok_or_else(|| "codex config.toml root must be a table".to_string())?;
    let servers = root_table
        .entry("mcp_servers".to_string())
        .or_insert_with(|| toml::Value::Table(toml::map::Map::new()))
        .as_table_mut()
        .ok_or_else(|| "mcp_servers must be a table".to_string())?;
    let was_present = servers.contains_key(id);
    let mut entry = servers
        .get(id)
        .cloned()
        .unwrap_or_else(|| toml::Value::Table(toml::map::Map::new()))
        .as_table()
        .cloned()
        .ok_or_else(|| format!("mcp_servers.{id} must be a table"))?;
    // 🔴 展示占位符安全检查（与 JSON 分支逻辑相同）
    let effective_command = resolve_executable_field(
        "command",
        id,
        &server.command,
        entry.get("command").and_then(toml::Value::as_str).unwrap_or(""),
    )?;
    let effective_url = resolve_executable_field(
        "url",
        id,
        &server.url,
        entry.get("url").and_then(toml::Value::as_str).unwrap_or(""),
    )?;
    set_toml_string(&mut entry, "command", &effective_command);
    set_toml_string(&mut entry, "url", &effective_url);
    set_toml_array(&mut entry, "args", &server.args);
    set_toml_map(&mut entry, "env", &server.env);
    set_toml_map(&mut entry, "http_headers", &server.headers);
    servers.insert(id.to_string(), toml::Value::Table(entry));
    Ok(was_present)
}

fn set_toml_string(entry: &mut toml::map::Map<String, toml::Value>, key: &str, value: &str) {
    if value.is_empty() {
        entry.remove(key);
    } else {
        entry.insert(key.to_string(), toml::Value::String(value.to_string()));
    }
}

fn set_toml_array(entry: &mut toml::map::Map<String, toml::Value>, key: &str, values: &[String]) {
    if values.is_empty() {
        entry.remove(key);
    } else {
        entry.insert(
            key.to_string(),
            toml::Value::Array(values.iter().cloned().map(toml::Value::String).collect()),
        );
    }
}

fn set_toml_map(
    entry: &mut toml::map::Map<String, toml::Value>,
    key: &str,
    values: &std::collections::BTreeMap<String, String>,
) {
    if values.is_empty() {
        entry.remove(key);
    } else {
        let existing = entry
            .get(key)
            .and_then(toml::Value::as_table)
            .cloned()
            .unwrap_or_default();
        entry.insert(
            key.to_string(),
            toml::Value::Table(
                values
                    .iter()
                    .map(|(key, value)| {
                        let value = if mcp::is_secret_ref(value) {
                            existing
                                .get(key)
                                .and_then(toml::Value::as_str)
                                .filter(|existing| {
                                    !existing.is_empty() && !mcp::is_secret_ref(existing)
                                })
                                .unwrap_or(value)
                        } else {
                            value
                        };
                        (key.clone(), toml::Value::String(value.to_string()))
                    })
                    .collect(),
            ),
        );
    }
}

fn mcp_mismatched_fields(expected: &source::McpServer, actual: &source::McpServer) -> Vec<String> {
    let mut fields = Vec::new();
    if expected.command != actual.command {
        fields.push("command".to_string());
    }
    if expected.url != actual.url {
        fields.push("url".to_string());
    }
    if expected.args != actual.args {
        fields.push("args".to_string());
    }
    if !mcp_credential_maps_match(&expected.env, &actual.env) {
        fields.push("env".to_string());
    }
    if !mcp_credential_maps_match(&expected.headers, &actual.headers) {
        fields.push("headers".to_string());
    }
    fields
}

fn mcp_credential_maps_match(
    expected: &std::collections::BTreeMap<String, String>,
    actual: &std::collections::BTreeMap<String, String>,
) -> bool {
    expected.len() == actual.len()
        && expected.iter().all(|(key, expected_value)| {
            actual.get(key).is_some_and(|actual_value| {
                expected_value == actual_value
                    || (mcp::is_secret_ref(expected_value)
                        && !mcp::is_secret_ref(actual_value)
                        && !actual_value.is_empty())
            })
        })
}

fn runtime_check(server: &source::McpServer) -> String {
    if server.command.is_empty() {
        return "not-applicable".to_string();
    }
    if server
        .env
        .values()
        .any(|value| value.starts_with("${secret:") || value.starts_with("${env:"))
    {
        return "blocked-secret-reference".to_string();
    }
    let mut child = match Command::new(&server.command)
        .args(&server.args)
        .envs(server.env.iter())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(error) => return format!("spawn-error: {error}"),
    };
    let Some(mut stdin) = child.stdin.take() else {
        let _ = child.kill();
        let _ = child.wait();
        return "stdin-unavailable".to_string();
    };
    let request = br#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"agentsync","version":"0.1.0"}}}
"#;
    if let Err(error) = stdin.write_all(request) {
        let _ = child.kill();
        let _ = child.wait();
        return format!("write-error: {error}");
    }
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let (sender, receiver) = mpsc::channel();
    let (error_sender, error_receiver) = mpsc::channel();
    if let Some(mut stdout) = stdout {
        std::thread::spawn(move || {
            let result = read_initialize_response(&mut BufReader::new(&mut stdout));
            let _ = sender.send(result);
        });
    }
    if let Some(mut stderr) = stderr {
        std::thread::spawn(move || {
            let mut diagnostics = String::new();
            let _ = stderr.read_to_string(&mut diagnostics);
            let _ = error_sender.send(diagnostics);
        });
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    let status = loop {
        if let Ok(Ok(status)) = receiver.try_recv() {
            if status == "initialized" {
                let _ = stdin.write_all(
                    br#"{"jsonrpc":"2.0","method":"notifications/initialized"}
"#,
                );
            }
            break status;
        }
        if let Ok(Some(exit)) = child.try_wait() {
            break format!("process-exited: {}", exit);
        }
        if Instant::now() >= deadline {
            break "timeout".to_string();
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let _ = child.kill();
    let _ = child.wait();
    if let Ok(diagnostics) = error_receiver.recv_timeout(Duration::from_secs(1)) {
        if diagnostics.lines().any(|line| {
            let line = line.to_ascii_lowercase();
            line.contains("unknown argument")
                || line.contains("unknown arguments")
                || line.contains("error")
        }) {
            let first = diagnostics
                .lines()
                .find(|line| {
                    let line = line.to_ascii_lowercase();
                    line.contains("unknown argument")
                        || line.contains("unknown arguments")
                        || line.contains("error")
                })
                .unwrap_or("runtime diagnostics reported an error");
            return format!("diagnostic-error: {first}");
        }
    }
    status
}

fn read_initialize_response(reader: &mut impl BufRead) -> std::io::Result<String> {
    let mut line = String::new();
    for _ in 0..16 {
        line.clear();
        let size = reader.read_line(&mut line)?;
        if size == 0 {
            return Ok("eof-before-initialize".to_string());
        }
        if let Some(status) = initialize_response_status(&line) {
            return Ok(status.to_string());
        }
    }
    Ok("invalid-initialize-response".to_string())
}

fn initialize_response_status(line: &str) -> Option<&'static str> {
    let value: Value = serde_json::from_str(line).ok()?;
    let object = value.as_object()?;
    if object.get("jsonrpc").and_then(Value::as_str) != Some("2.0")
        || object.get("id").and_then(Value::as_i64) != Some(1)
    {
        return None;
    }
    if object.get("result").is_some() {
        Some("initialized")
    } else if object.get("error").is_some() {
        Some("initialize-error")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::Core;
    use crate::core::desktop::model::{Options, ProjectMcpPromotionRequest};
    use crate::core::test_support::TestDir;
    use std::fs;

    fn options(dir: &TestDir) -> Options {
        Options {
            home: dir.path().join("global/.agentsync"),
            projects_root: dir.path().join("projects"),
            native_root: dir.path().join("native"),
        }
    }

    fn project(dir: &TestDir) -> std::path::PathBuf {
        let project = dir.path().join("project");
        fs::create_dir_all(project.join(".agentsync/mcp")).unwrap();
        project
    }

    #[test]
    fn promotes_global_recipe_without_project_scope_leaking_into_snapshot() {
        let dir = TestDir::new("mcp-promotion");
        let project = project(&dir);
        fs::write(
            project.join(".agentsync/mcp/chrome-devtools.toml"),
            "[server]\nname = \"chrome-devtools\"\nrecipe = \"chrome-devtools\"\n",
        )
        .unwrap();
        let core = Core::new(options(&dir));
        let result = core
            .promote_project_mcp(ProjectMcpPromotionRequest {
                project_path: project.to_string_lossy().to_string(),
                mcp_id: "chrome-devtools".to_string(),
            })
            .unwrap();
        assert_eq!(result.status, "promoted");
        assert!(dir
            .path()
            .join("global/.agentsync/mcp/chrome-devtools.toml")
            .is_file());
    }

    #[test]
    fn rejects_conflicting_global_mcp_instead_of_overwriting() {
        let dir = TestDir::new("mcp-promotion-conflict");
        let project = project(&dir);
        fs::create_dir_all(dir.path().join("global/.agentsync/mcp")).unwrap();
        fs::write(
            project.join(".agentsync/mcp/chrome-devtools.toml"),
            "[server]\nname = \"chrome-devtools\"\nrecipe = \"chrome-devtools\"\n",
        )
        .unwrap();
        fs::write(
            dir.path()
                .join("global/.agentsync/mcp/chrome-devtools.toml"),
            "[server]\nname = \"other\"\nrecipe = \"chrome-devtools\"\n",
        )
        .unwrap();
        let core = Core::new(options(&dir));
        let error = core
            .promote_project_mcp(ProjectMcpPromotionRequest {
                project_path: project.to_string_lossy().to_string(),
                mcp_id: "chrome-devtools".to_string(),
            })
            .unwrap_err();
        assert!(error.contains("different configuration"));
    }

    #[test]
    fn push_mcp_to_agent_writes_codex_toml() {
        let dir = TestDir::new("mcp-push-codex");
        let opts = options(&dir);
        // 准备 canonical 全局 MCP
        fs::create_dir_all(opts.home.join("mcp")).unwrap();
        fs::write(
            opts.home.join("mcp/github.toml"),
            "[server]\nname = \"github\"\ncommand = \"npx\"\nargs = [\"-y\", \"@mcp/github\"]\n\n[server.env]\nGITHUB_TOKEN = \"${secret:GITHUB_TOKEN}\"\n",
        )
        .unwrap();
        // 创建 native root（codex 目录）
        fs::create_dir_all(opts.native_root.join(".codex")).unwrap();

        let core = Core::new(opts.clone());
        let result = core
            .push_mcp_to_agent(super::super::model::PushMcpToAgentRequest {
                mcp_id: "github".to_string(),
                agent: "codex".to_string(),
            })
            .unwrap();

        assert_eq!(result.status, "written");
        assert!(result.path.contains(".codex/config.toml"));

        // 检验写入的 TOML 格式
        let written = fs::read_to_string(opts.native_root.join(".codex/config.toml")).unwrap();
        let parsed: toml::Value = written.parse().unwrap();
        let entry = parsed
            .get("mcp_servers")
            .and_then(|s| s.get("github"))
            .expect("mcp_servers.github must exist");
        assert_eq!(entry.get("command").and_then(|v| v.as_str()), Some("npx"));
        let env_token = entry
            .get("env")
            .and_then(|e| e.get("GITHUB_TOKEN"))
            .and_then(|v| v.as_str());
        assert_eq!(env_token, Some("${secret:GITHUB_TOKEN}"));
    }

    #[test]
    fn push_mcp_to_agent_updates_existing_codex_entry() {
        let dir = TestDir::new("mcp-push-codex-update");
        let opts = options(&dir);
        fs::create_dir_all(opts.home.join("mcp")).unwrap();
        fs::write(
            opts.home.join("mcp/github.toml"),
            "[server]\nname = \"github\"\ncommand = \"npx\"\n",
        )
        .unwrap();
        fs::create_dir_all(opts.native_root.join(".codex")).unwrap();
        // 预先写一个旧的 entry
        fs::write(
            opts.native_root.join(".codex/config.toml"),
            "[mcp_servers.github]\ncommand = \"old-command\"\n",
        )
        .unwrap();

        let core = Core::new(opts.clone());
        let result = core
            .push_mcp_to_agent(super::super::model::PushMcpToAgentRequest {
                mcp_id: "github".to_string(),
                agent: "codex".to_string(),
            })
            .unwrap();

        assert_eq!(result.status, "updated");
        let written = fs::read_to_string(opts.native_root.join(".codex/config.toml")).unwrap();
        let parsed: toml::Value = written.parse().unwrap();
        let cmd = parsed
            .get("mcp_servers")
            .and_then(|s| s.get("github"))
            .and_then(|e| e.get("command"))
            .and_then(|v| v.as_str());
        assert_eq!(cmd, Some("npx")); // 已更新为新值
    }

    #[test]
    fn verify_mcp_in_agent_reads_codex_toml() {
        let dir = TestDir::new("mcp-verify-codex");
        let opts = options(&dir);
        fs::create_dir_all(opts.home.join("mcp")).unwrap();
        fs::write(
            opts.home.join("mcp/github.toml"),
            "[server]\nname = \"github\"\ncommand = \"npx\"\n",
        )
        .unwrap();
        fs::write(
            opts.home.join("mcp/missing-mcp.toml"),
            "[server]\nname = \"missing-mcp\"\ncommand = \"npx\"\n",
        )
        .unwrap();
        fs::create_dir_all(opts.native_root.join(".codex")).unwrap();
        fs::write(
            opts.native_root.join(".codex/config.toml"),
            "[mcp_servers.github]\ncommand = \"npx\"\n",
        )
        .unwrap();

        let core = Core::new(opts);
        // github 存在
        let found = core
            .verify_mcp_in_agent(super::super::model::VerifyMcpInAgentRequest {
                mcp_id: "github".to_string(),
                agent: "codex".to_string(),
                runtime_check: false,
            })
            .unwrap();
        assert_eq!(found.status, "found");

        // missing-mcp 不存在
        let missing = core
            .verify_mcp_in_agent(super::super::model::VerifyMcpInAgentRequest {
                mcp_id: "missing-mcp".to_string(),
                agent: "codex".to_string(),
                runtime_check: false,
            })
            .unwrap();
        assert_eq!(missing.status, "missing");
    }

    #[cfg(unix)]
    #[test]
    fn adapts_antigravity_verified_alias_and_verifies_shared_target() {
        use std::os::unix::fs::symlink;

        let dir = TestDir::new("mcp-antigravity-shared-alias");
        let opts = options(&dir);
        fs::create_dir_all(opts.home.join("mcp")).unwrap();
        fs::write(
            opts.home.join("mcp/chrome-devtools.toml"),
            "[server]\nname = \"chrome-devtools\"\ncommand = \"node\"\nargs = [\"--version\"]\n",
        )
        .unwrap();
        fs::create_dir_all(opts.native_root.join(".gemini/config")).unwrap();
        fs::create_dir_all(opts.native_root.join(".gemini/antigravity")).unwrap();
        fs::write(
            opts.native_root.join(".gemini/config/mcp_config.json"),
            r#"{"mcpServers":{"existing":{"command":"keep"}}}"#,
        )
        .unwrap();
        symlink(
            "../config/mcp_config.json",
            opts.native_root.join(".gemini/antigravity/mcp_config.json"),
        )
        .unwrap();

        let core = Core::new(opts.clone());
        let result = core
            .push_mcp_to_agent(super::super::model::PushMcpToAgentRequest {
                mcp_id: "chrome-devtools".to_string(),
                agent: "antigravity".to_string(),
            })
            .unwrap();

        assert_eq!(result.status, "written");
        assert_eq!(result.path, "~/.gemini/antigravity/mcp_config.json");
        assert_eq!(result.physical_path, "~/.gemini/config/mcp_config.json");

        let verify = core
            .verify_mcp_in_agent(super::super::model::VerifyMcpInAgentRequest {
                mcp_id: "chrome-devtools".to_string(),
                agent: "antigravity".to_string(),
                runtime_check: false,
            })
            .unwrap();
        assert_eq!(verify.status, "found");
        assert_eq!(verify.mismatched_fields, Vec::<String>::new());
        assert_eq!(verify.physical_path, "~/.gemini/config/mcp_config.json");
        assert!(
            fs::read_to_string(opts.native_root.join(".gemini/config/mcp_config.json"))
                .unwrap()
                .contains("\"existing\"")
        );

        let imported = core
            .promote_native_mcp_force(super::super::model::NativeMcpPromotionRequest {
                agent: "antigravity".to_string(),
                mcp_id: "chrome-devtools".to_string(),
            })
            .unwrap();
        assert_eq!(imported.status, "already-global");
    }

    #[test]
    fn verification_rejects_existing_entry_with_different_runtime_fields() {
        let dir = TestDir::new("mcp-verify-mismatch");
        let opts = options(&dir);
        fs::create_dir_all(opts.home.join("mcp")).unwrap();
        fs::write(
            opts.home.join("mcp/chrome-devtools.toml"),
            "[server]\nname = \"chrome-devtools\"\ncommand = \"node\"\nargs = [\"--version\"]\n",
        )
        .unwrap();
        fs::create_dir_all(opts.native_root.join(".cursor")).unwrap();
        fs::write(
            opts.native_root.join(".cursor/mcp.json"),
            r#"{"mcpServers":{"chrome-devtools":{"command":"wrong","args":["--help"]}}}"#,
        )
        .unwrap();

        let result = Core::new(opts)
            .verify_mcp_in_agent(super::super::model::VerifyMcpInAgentRequest {
                mcp_id: "chrome-devtools".to_string(),
                agent: "cursor".to_string(),
                runtime_check: false,
            })
            .unwrap();

        assert_eq!(result.status, "mismatch");
        assert_eq!(result.mismatched_fields, vec!["command", "args"]);
    }

    #[test]
    fn native_import_converts_plaintext_secret_to_environment_reference() {
        let dir = TestDir::new("mcp-import-plaintext-secret");
        let opts = options(&dir);
        fs::create_dir_all(opts.native_root.join(".gemini/config")).unwrap();
        fs::write(
            opts.native_root.join(".gemini/config/mcp_config.json"),
            r#"{"mcpServers":{"f2c-mcp":{"command":"node","args":["@f2c/mcp"],"env":{"PATH":"/usr/bin","personalToken":"figd_private"}}}}"#,
        )
        .unwrap();

        let result = Core::new(opts.clone())
            .promote_native_mcp_force(super::super::model::NativeMcpPromotionRequest {
                agent: "antigravity-ide".to_string(),
                mcp_id: "f2c-mcp".to_string(),
            })
            .unwrap();

        assert_eq!(result.status, "promoted");
        let canonical = fs::read_to_string(opts.home.join("mcp/f2c-mcp.toml")).unwrap();
        assert!(canonical.contains("${env:personalToken}"));
        assert!(!canonical.contains("figd_private"));
    }

    #[test]
    fn update_preserves_native_credentials_and_runs_with_them() {
        let dir = TestDir::new("mcp-update-preserves-secret");
        let opts = options(&dir);
        fs::create_dir_all(opts.home.join("mcp")).unwrap();
        fs::write(
            opts.home.join("mcp/runtime-test.toml"),
            "[server]\ncommand = \"sh\"\nargs = [\"-c\", \"printf '{\\\"jsonrpc\\\":\\\"2.0\\\",\\\"id\\\":1,\\\"result\\\":{}}'; sleep 1\"]\n[server.env]\nTOKEN = \"${env:TOKEN}\"\n",
        )
        .unwrap();
        fs::create_dir_all(opts.native_root.join(".cursor")).unwrap();
        fs::write(
            opts.native_root.join(".cursor/mcp.json"),
            r#"{"mcpServers":{"runtime-test":{"command":"sh","args":["-c","printf '{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}'; sleep 1"],"env":{"TOKEN":"native-secret"}}}}"#,
        )
        .unwrap();

        let core = Core::new(opts.clone());
        core.push_mcp_to_agent(super::super::model::PushMcpToAgentRequest {
            mcp_id: "runtime-test".to_string(),
            agent: "cursor".to_string(),
        })
        .unwrap();

        let written = fs::read_to_string(opts.native_root.join(".cursor/mcp.json")).unwrap();
        assert!(written.contains("native-secret"));
        assert!(!written.contains("${env:TOKEN}"));

        let verification = core
            .verify_mcp_in_agent(super::super::model::VerifyMcpInAgentRequest {
                mcp_id: "runtime-test".to_string(),
                agent: "cursor".to_string(),
                runtime_check: true,
            })
            .unwrap();
        assert_eq!(verification.status, "found");
        assert_eq!(verification.runtime_status, "initialized");
    }

    #[test]
    fn runtime_check_requires_initialize_response() {
        let server = crate::core::source::McpServer {
            id: "runtime-test".to_string(),
            name: "runtime-test".to_string(),
            description: String::new(),
            recipe: String::new(),
            command: "sh".to_string(),
            url: String::new(),
            args: vec!["-c".to_string(), "printf ready; sleep 1".to_string()],
            env: std::collections::BTreeMap::new(),
            headers: std::collections::BTreeMap::new(),
            transport: String::new(),
            package: String::new(),
            runner: String::new(),
        };

        assert_eq!(super::runtime_check(&server), "eof-before-initialize");
    }

    // ── 以下三个测试覆盖 HANDOFF.md §3.3 中描述的根因问题 ──

    /// save_global_mcp 收到明文凭据时必须返回错误，且 canonical 文件不能被写入。
    #[test]
    fn save_global_mcp_rejects_plaintext_secret() {
        let dir = TestDir::new("mcp-save-plaintext-secret");
        let opts = options(&dir);
        fs::create_dir_all(opts.home.join("mcp")).unwrap();
        // 先写一个合法的 canonical 文件（带引用形式的凭据）
        fs::write(
            opts.home.join("mcp/f2c-mcp.toml"),
            "[server]\nname = \"f2c-mcp\"\nrecipe = \"f2c-mcp\"\n\n[server.env]\npersonalToken = \"${env:F2C_TOKEN}\"\n",
        )
        .unwrap();

        let core = Core::new(opts.clone());
        let err = core
            .save_global_mcp(super::super::model::SaveMcpRequest {
                mcp_id: "f2c-mcp".to_string(),
                name: "f2c-mcp".to_string(),
                description: String::new(),
                recipe: "f2c-mcp".to_string(),
                url: String::new(),
                command: String::new(),
                // 用户填入了明文 token（脱敏后的字面值模拟真实值）
                env: std::collections::BTreeMap::from([(
                    "personalToken".to_string(),
                    "figd_plaintext_token".to_string(),
                )]),
            })
            .unwrap_err();

        // 必须返回包含 secret-like 或 plaintext 字样的错误
        assert!(
            err.to_lowercase().contains("secret")
                || err.to_lowercase().contains("plaintext")
                || err.to_lowercase().contains("reference"),
            "expected plaintext-blocking error, got: {err}"
        );
        // canonical 文件内容不能被明文值覆盖
        let canonical = fs::read_to_string(opts.home.join("mcp/f2c-mcp.toml")).unwrap();
        assert!(
            !canonical.contains("figd_plaintext_token"),
            "canonical file must not contain plaintext token"
        );
        assert!(
            canonical.contains("${env:F2C_TOKEN}"),
            "canonical file must retain original reference"
        );
    }

    /// canonical command 为展示占位符、原生文件已有真实可执行命令时，push 后必须保留原生命令。
    #[test]
    fn push_mcp_preserves_native_command_when_canonical_is_display_placeholder() {
        let dir = TestDir::new("mcp-push-placeholder-command");
        let opts = options(&dir);
        fs::create_dir_all(opts.home.join("mcp")).unwrap();
        // canonical 中 command 是脱敏占位符
        fs::write(
            opts.home.join("mcp/f2c-mcp.toml"),
            "[server]\nname = \"f2c-mcp\"\nrecipe = \"f2c-mcp\"\ncommand = \"本地命令（已脱敏）\"\nargs = [\"/opt/homebrew/lib/node_modules/npm/bin/npx-cli.js\", \"-y\", \"@f2c/mcp\"]\n\n[server.env]\npersonalToken = \"${env:F2C_TOKEN}\"\n",
        )
        .unwrap();
        // 原生文件中有真实可执行命令
        fs::create_dir_all(opts.native_root.join(".gemini/config")).unwrap();
        fs::write(
            opts.native_root.join(".gemini/config/mcp_config.json"),
            r#"{"mcpServers":{"f2c-mcp":{"command":"/opt/homebrew/bin/node","args":["/opt/homebrew/lib/node_modules/npm/bin/npx-cli.js","-y","@f2c/mcp"],"env":{"personalToken":"figd_real_token"}}}}"#,
        )
        .unwrap();

        let core = Core::new(opts.clone());
        core.push_mcp_to_agent(super::super::model::PushMcpToAgentRequest {
            mcp_id: "f2c-mcp".to_string(),
            agent: "antigravity-ide".to_string(),
        })
        .unwrap();

        let written =
            fs::read_to_string(opts.native_root.join(".gemini/config/mcp_config.json")).unwrap();
        // 必须保留真实 Node 命令，不能写入占位符字符串
        assert!(
            written.contains("/opt/homebrew/bin/node"),
            "native command must be preserved, got: {written}"
        );
        assert!(
            !written.contains("本地命令（已脱敏）"),
            "display placeholder must not appear in native config"
        );
        // 凭据保留逻辑：真实 token 应保持（secret ref 不覆盖真实值）
        assert!(
            written.contains("figd_real_token"),
            "native credential must be preserved"
        );
    }

    /// canonical command 为展示占位符、原生文件也没有真实命令时，push 应返回错误而非写入占位符。
    #[test]
    fn push_mcp_fails_when_placeholder_command_and_no_native_fallback() {
        let dir = TestDir::new("mcp-push-placeholder-no-fallback");
        let opts = options(&dir);
        fs::create_dir_all(opts.home.join("mcp")).unwrap();
        // canonical 中 command 是脱敏占位符
        fs::write(
            opts.home.join("mcp/f2c-mcp.toml"),
            "[server]\nname = \"f2c-mcp\"\nrecipe = \"f2c-mcp\"\ncommand = \"本地命令（已脱敏）\"\nargs = [\"/opt/homebrew/lib/node_modules/npm/bin/npx-cli.js\", \"-y\", \"@f2c/mcp\"]\n\n[server.env]\npersonalToken = \"${env:F2C_TOKEN}\"\n",
        )
        .unwrap();
        // 原生文件不存在（无真实命令）
        fs::create_dir_all(opts.native_root.join(".gemini/config")).unwrap();

        let core = Core::new(opts.clone());
        let err = core
            .push_mcp_to_agent(super::super::model::PushMcpToAgentRequest {
                mcp_id: "f2c-mcp".to_string(),
                agent: "antigravity-ide".to_string(),
            })
            .unwrap_err();

        assert!(
            err.to_lowercase().contains("display")
                || err.to_lowercase().contains("placeholder")
                || err.to_lowercase().contains("脱敏")
                || err.to_lowercase().contains("executable"),
            "expected display-placeholder error, got: {err}"
        );
        // 原生文件不应被创建/写入
        assert!(
            !opts
                .native_root
                .join(".gemini/config/mcp_config.json")
                .exists(),
            "native config must not be written when command is a placeholder"
        );
    }
}
