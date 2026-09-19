use super::model::{
    ActivityItem, AgentSummary, ApplyPreview, CapabilityReport, Global, GlobalMcpItem,
    GlobalRuleItem, McpAdapter, McpCredential, Metrics, NativeHookSource, NativeMcpSource,
    NativeRuleSource, NativeSkillItem, NativeSkillSource, NativeWorkflowSource, ProjectMcpItem,
    ProjectSummary, WorkspaceSnapshot,
};
use super::Core;
use crate::core::mcp;
use crate::core::{adapter, jsonkeys, paths, source, state};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use url::Url;

impl Core {
    pub(crate) fn read_snapshot(&self) -> Result<WorkspaceSnapshot, String> {
        let global_source = source::load(&self.opts.home)
            .map_err(|error| format!("load global agentsync source: {error}"))?;
        let mut project_roots = discover_projects(&self.opts.projects_root)?;
        let registry = state::load_project_registry(&self.project_registry_path())
            .map_err(|error| format!("load imported projects: {error}"))?;
        for item in registry.projects {
            if item.path.is_dir() {
                project_roots.insert(item.path);
            }
        }

        let mut snapshot = WorkspaceSnapshot {
            schema_version: 1,
            mode: "real".to_string(),
            generated_at: now_rfc3339(),
            metrics: Metrics::default(),
            global: global_summary(&global_source, &self.opts.home),
            agents: Vec::new(),
            projects: Vec::new(),
            activity: Vec::new(),
        };
        let native = discover_native_mcp(&self.opts.native_root);
        snapshot.global.native_mcp_items = native.items;
        snapshot.global.native_mcp = native.sources;
        snapshot.global.native_rules = discover_native_rules(&self.opts.native_root);
        snapshot.global.native_skills = discover_native_skills(&self.opts.native_root);
        snapshot.global.native_hooks = discover_native_hooks(&self.opts.native_root);
        snapshot.global.native_subagents = discover_native_subagents(&self.opts.native_root);
        snapshot.global.native_workflows = discover_native_workflows(&self.opts.native_root);
        snapshot.global.rules.extend(
            snapshot
                .global
                .native_rules
                .iter()
                .filter(|rule| rule.kind == "primary" && rule.state == "present")
                .map(|rule| GlobalRuleItem {
                    id: format!("{}-native-rule", rule.agent),
                    name: format!("{} 全局规则", display_agent_name(&rule.agent)),
                    description: "来自本机 Agent 原生全局文件，尚未写入 ~/.agentsync canonical。"
                        .to_string(),
                    enabled: true,
                    source: "agent".to_string(),
                }),
        );

        let mut agent_names = enabled_agent_names(&global_source.config.agents)
            .into_iter()
            .collect::<BTreeSet<_>>();
        for item in &snapshot.global.native_mcp_items {
            for adapter in &item.adapters {
                agent_names.insert(canonical_agent_id(&adapter.agent).to_string());
            }
        }
        for root in project_roots {
            let canonical = source::load(&paths::project_home(&root))
                .map_err(|error| format!("load project source {}: {error}", root.display()))?;
            let agents = enabled_agent_names(&canonical.config.agents);
            agent_names.extend(agents.iter().cloned());
            let name = root
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .to_string();
            let (profile, stack) = project_profile(&name);
            let project_mcp = canonical
                .mcp_servers
                .iter()
                .map(|server| project_mcp_summary(server, &agents))
                .collect();
            let sync_state = if agents.is_empty() {
                "not-configured"
            } else {
                "discovered"
            };
            snapshot.projects.push(ProjectSummary {
                id: name.clone(),
                name: name.clone(),
                path: root.to_string_lossy().to_string(),
                profile,
                stack,
                agents,
                mcp: project_mcp,
                sync_state: sync_state.to_string(),
                updated_at: "本机读取".to_string(),
            });
            snapshot.activity.push(ActivityItem {
                id: format!("project-{name}"),
                title: format!("{name} 已读取"),
                detail: format!("从 {} 读取项目 Profile 和 Agent 配置", root.display()),
                timestamp: "本次启动".to_string(),
                tone: "success".to_string(),
            });
        }
        snapshot
            .projects
            .sort_by(|left, right| left.name.cmp(&right.name));
        snapshot.agents = agent_names
            .into_iter()
            .map(|name| agent_summary(&name))
            .collect();
        snapshot
            .agents
            .sort_by(|left, right| left.name.cmp(&right.name));
        let global_mcp_adapters = global_mcp_adapters(&snapshot.agents);
        for server in &mut snapshot.global.mcp {
            server.adapters = global_mcp_adapters.clone();
        }
        snapshot.metrics.agents = snapshot.agents.len();
        snapshot.metrics.projects = snapshot.projects.len();
        snapshot.metrics.components = snapshot.global.rules.len()
            + snapshot.global.mcp.len()
            + snapshot.global.skills
            + snapshot.global.hooks
            + snapshot.global.subagents;
        snapshot.metrics.attention = snapshot
            .agents
            .iter()
            .filter(|agent| agent.health != "good")
            .count();
        Ok(snapshot)
    }

    pub(crate) fn preview_apply(&self) -> Result<ApplyPreview, String> {
        Ok(preview_apply(&self.read_snapshot()?))
    }

    pub(super) fn project_registry_path(&self) -> PathBuf {
        self.opts.home.join(".state/agent-assistant/projects.json")
    }
}

fn preview_apply(snapshot: &WorkspaceSnapshot) -> ApplyPreview {
    let mut preview = ApplyPreview {
        files: snapshot.metrics.components + snapshot.agents.len() + snapshot.projects.len(),
        warnings: Vec::new(),
        ..ApplyPreview::default()
    };
    for agent in &snapshot.agents {
        for capability in agent.capabilities.components.values() {
            match capability.as_str() {
                "partial" => preview.partial += 1,
                "unsupported" => preview.unsupported += 1,
                _ => {}
            }
        }
        if agent.health != "good" {
            preview
                .warnings
                .push(format!("{} 存在能力差异", agent.name));
        }
    }
    preview
}

fn discover_projects(root: &Path) -> Result<BTreeSet<PathBuf>, String> {
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeSet::new()),
        Err(error) => {
            return Err(format!(
                "discover projects under {}: {error}",
                root.display()
            ))
        }
    };
    let mut projects = BTreeSet::new();
    for entry in entries {
        let entry = entry.map_err(|error| format!("read {}: {error}", root.display()))?;
        if entry
            .file_type()
            .map_err(|error| format!("inspect {}: {error}", entry.path().display()))?
            .is_dir()
            && paths::project_home(&entry.path()).is_dir()
        {
            projects.insert(entry.path());
        }
    }
    Ok(projects)
}

fn global_summary(canonical: &source::Canonical, home: &Path) -> Global {
    let canonical_state = if home.join("agentsync.toml").is_file() {
        "ready"
    } else if home.exists() {
        "empty"
    } else {
        "missing"
    };
    let mut global = Global {
        canonical_state: canonical_state.to_string(),
        canonical_path: home.to_string_lossy().to_string(),
        rules: Vec::new(),
        native_rules: Vec::new(),
        mcp: Vec::new(),
        native_mcp: Vec::new(),
        native_mcp_items: Vec::new(),
        skills: canonical.skills,
        native_skills: Vec::new(),
        hooks: canonical.hooks,
        native_hooks: Vec::new(),
        subagents: canonical.subagents,
        native_subagents: Vec::new(),
        workflows: 0,
        native_workflows: Vec::new(),
    };
    if !canonical.memory.body.trim().is_empty() {
        global.rules.push(GlobalRuleItem {
            id: "memory".to_string(),
            name: "全局记忆".to_string(),
            description: "来自 ~/.agentsync/memory/AGENTS.md 的公共 Agent 规则。".to_string(),
            enabled: true,
            source: "global".to_string(),
        });
    }
    global.mcp.extend(
        canonical
            .mcp_servers
            .iter()
            .map(|server| mcp_summary("", server, "global")),
    );
    global
}

#[derive(Default)]
struct NativeGlobal {
    sources: Vec<NativeMcpSource>,
    items: Vec<GlobalMcpItem>,
}

// Known user-level locations; only the three established parsers yield server summaries.
pub(super) const NATIVE_MCP_TARGETS: &[(&str, &str)] = &[
    ("amazonq", ".aws/amazonq/mcp.json"),
    ("amp", ".config/amp/settings.json"),
    // antigravity-ide / antigravity-cli 由 discover_native_mcp 动态派生，此处只保留主条目
    ("antigravity", ".gemini/config/mcp_config.json"),
    ("claude", ".claude.json"),
    ("cline", ".cline/mcp.json"),
    ("codex", ".codex/config.toml"),
    ("continue", ".continue/mcpServers"),
    ("copilot-cli", ".copilot/mcp-config.json"),
    ("cursor", ".cursor/mcp.json"),
    ("factory", ".factory/mcp.json"),
    ("gemini", ".gemini/settings.json"),
    ("junie", ".junie/mcp/mcp.json"),
    ("kiro", ".kiro/settings/mcp.json"),
    ("opencode", ".config/opencode/opencode.json"),
    ("opencode", ".config/opencode/opencode.jsonc"),
    ("pi", ".pi/agent/mcp.json"),
    ("qwen", ".qwen/settings.json"),
    ("warp", ".warp/.mcp.json"),
    ("windsurf", ".codeium/windsurf/mcp_config.json"),
    ("zed", ".config/zed/settings.json"),
];

const MAX_NATIVE_MCP_BYTES: u64 = 1024 * 1024;

fn discover_native_mcp(home: &Path) -> NativeGlobal {
    let mut summary = NativeGlobal::default();
    for agent in adapter::names() {
        let targets = NATIVE_MCP_TARGETS
            .iter()
            .filter(|(name, _)| *name == agent)
            .map(|(_, relative)| *relative)
            .collect::<Vec<_>>();
        if targets.is_empty() {
            summary.sources.push(NativeMcpSource {
                agent: agent.to_string(),
                path: String::new(),
                state: "unavailable".to_string(),
                reason: "无已核实的用户级 MCP 配置路径".to_string(),
                server_count: 0,
            });
        }
        for relative in targets {
            let path = home.join(relative);
            let mut source = NativeMcpSource {
                agent: agent.to_string(),
                path: format!("~/{relative}"),
                state: "missing".to_string(),
                reason: "未找到目标路径".to_string(),
                server_count: 0,
            };
            if paths::reject_symlinks_below(home, &path).is_err() {
                source.state = "unsafe".to_string();
                source.reason = "符号链接或路径不可安全读取".to_string();
            } else {
                match fs::symlink_metadata(&path) {
                    Ok(metadata) if metadata.file_type().is_dir() && agent == "continue" => {
                        source.state = "unverified".to_string();
                        source.reason = "已发现目录，未核实配置格式".to_string();
                    }
                    Ok(metadata) if !metadata.file_type().is_file() => {
                        source.state = "unsafe".to_string();
                        source.reason = "目标不是普通文件".to_string();
                    }
                    Ok(metadata) if metadata.len() == 0 => {
                        source.state = "empty".to_string();
                        source.reason = "文件为空".to_string();
                    }
                    Ok(metadata) if metadata.len() > MAX_NATIVE_MCP_BYTES => {
                        source.state = "oversized".to_string();
                        source.reason = "文件超过 1 MiB 扫描上限".to_string();
                    }
                    Ok(_) if matches!(agent, "codex" | "cursor" | "gemini" | "antigravity") => {
                        match fs::read_to_string(&path) {
                            Ok(body) if body.len() as u64 > MAX_NATIVE_MCP_BYTES => {
                                source.state = "oversized".to_string();
                                source.reason = "文件超过 1 MiB 扫描上限".to_string();
                            }
                            Ok(body) => match parse_native_mcp_body(agent, &body) {
                                Ok(servers) => {
                                    source.server_count = servers.len();
                                    source.state = "parsed".to_string();
                                    source.reason = if servers.is_empty() {
                                        "未配置 MCP 服务器".to_string()
                                    } else {
                                        "仅解析摘要，未验证导入兼容性".to_string()
                                    };
                                    for server in servers {
                                        summary.items.push(mcp_summary(agent, &server, "agent"));
                                    }
                                }
                                Err(_) => {
                                    source.state = "invalid".to_string();
                                    source.reason = "配置无法解析，已跳过该来源".to_string();
                                }
                            },
                            Err(_) => {
                                source.state = "unreadable".to_string();
                                source.reason = "无法读取目标文件".to_string();
                            }
                        }
                    }
                    Ok(_) => {
                        source.state = "unverified".to_string();
                        source.reason = "已发现配置文件，未核实导入兼容性".to_string();
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(_) => {
                        source.state = "unreadable".to_string();
                        source.reason = "无法检查目标文件".to_string();
                    }
                }
            }
            if agent == "antigravity" {
                source.agent = "antigravity-cli".to_string();
                source.reason = format!("{}；与 Antigravity IDE 共享此配置", source.reason);
                let mut ide = source.clone();
                ide.agent = "antigravity-ide".to_string();
                ide.reason = format!("{}；与 Antigravity CLI 共享此配置", ide.reason);
                summary.sources.push(ide);
                let mut app = source.clone();
                app.agent = "antigravity".to_string();
                app.path = "~/.gemini/antigravity/mcp_config.json".to_string();
                let alias = home.join(".gemini/antigravity/mcp_config.json");
                let canonical = home.join(relative);
                match (fs::canonicalize(&alias), fs::canonicalize(&canonical)) {
                    (Ok(alias_target), Ok(config_target)) if alias_target == config_target => {
                        app.reason =
                            "本机配置链接指向 IDE/CLI 共享文件；摘要只解析一次".to_string();
                    }
                    _ => {
                        app.state = "unavailable".to_string();
                        app.server_count = 0;
                        app.reason = "未验证桌面应用配置链接指向共享文件".to_string();
                    }
                }
                summary.sources.push(app);
            }
            summary.sources.push(source);
        }
    }
    summary.items = consolidate_native_mcp(summary.items, &summary.sources);
    summary
}

pub(super) fn discover_native_rules(home: &Path) -> Vec<NativeRuleSource> {
    let mut rules = adapter::names()
        .into_iter()
        .map(|agent| {
            let target = adapter::memory_target(agent, adapter::Scope::User, home, None);
            let (path, state, reason) = match target {
                Ok(target) => match target.path {
                    Some(path) => {
                        let display_path = path
                            .strip_prefix(home)
                            .map(|relative| format!("~/{}", logical_path(relative)))
                            .unwrap_or_default();
                        let status = if paths::reject_symlinks_below(home, &path).is_err() {
                            ("unsafe", "符号链接或路径不可安全读取")
                        } else {
                            match fs::symlink_metadata(&path) {
                                Ok(metadata)
                                    if metadata.file_type().is_file() && metadata.len() > 0 =>
                                {
                                    ("present", "")
                                }
                                Ok(metadata) if metadata.file_type().is_file() => {
                                    ("empty", "文件为空")
                                }
                                Ok(_) => ("unsafe", "目标不是普通文件"),
                                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                                    ("missing", "未找到目标文件")
                                }
                                Err(_) => ("unreadable", "无法检查目标文件"),
                            }
                        };
                        (display_path, status.0.to_string(), status.1.to_string())
                    }
                    None => (
                        String::new(),
                        "unavailable".to_string(),
                        "无已验证的全局文件目标".to_string(),
                    ),
                },
                Err(_) => (
                    String::new(),
                    "unavailable".to_string(),
                    "无法确定全局文件目标".to_string(),
                ),
            };
            NativeRuleSource {
                agent: agent.to_string(),
                kind: "primary".to_string(),
                path,
                state,
                reason,
            }
        })
        .collect::<Vec<_>>();
    rules.retain(|rule| rule.agent != "antigravity" && rule.agent != "cursor");
    for agent in ["antigravity-ide", "antigravity", "antigravity-cli"] {
        let mut shared = inspect_auxiliary_rule(home, agent, ".gemini/GEMINI.md");
        shared.reason = if shared.state == "present" {
            "与 Gemini CLI 及其他 Antigravity 端共享，尚未导入母版".to_string()
        } else {
            format!("{}；与 Gemini CLI 及其他 Antigravity 端共享", shared.reason)
        };
        rules.push(shared);
    }
    rules.push(NativeRuleSource {
        agent: "cursor-account".to_string(),
        kind: "auxiliary".to_string(),
        path: String::new(),
        state: "unavailable".to_string(),
        reason: "Cursor 账户 User Rules 未提供可核实的本机文件；请在 Cursor 设置中查看".to_string(),
    });
    let cursor_dir = home.join(".cursor/rules");
    let (cursor_state, cursor_reason) = if paths::reject_symlinks_below(home, &cursor_dir).is_err()
    {
        ("unsafe", "规则目录不可安全读取")
    } else {
        match fs::symlink_metadata(&cursor_dir) {
            Ok(metadata) if metadata.file_type().is_dir() => {
                ("directory", "本机规则目录已发现，规则文件在下方逐项列出")
            }
            Ok(_) => ("unsafe", "目标不是目录"),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                ("missing", "未找到本机规则目录；不代表账户规则为空")
            }
            Err(_) => ("unreadable", "无法检查本机规则目录"),
        }
    };
    rules.push(NativeRuleSource {
        agent: "cursor-local".to_string(),
        kind: "auxiliary".to_string(),
        path: "~/.cursor/rules".to_string(),
        state: cursor_state.to_string(),
        reason: cursor_reason.to_string(),
    });
    rules.push(inspect_auxiliary_rule(
        home,
        "gemini",
        ".gemini/config/AGENTS.md",
    ));
    for (agent, relative, extension) in [
        ("codex", ".codex/rules", "rules"),
        ("claude", ".claude/rules", "md"),
        ("cursor-local", ".cursor/rules", "mdc"),
    ] {
        let root = home.join(relative);
        if paths::reject_symlinks_below(home, &root).is_err() {
            rules.push(NativeRuleSource {
                agent: agent.to_string(),
                kind: "auxiliary".to_string(),
                path: format!("~/{relative}"),
                state: "unsafe".to_string(),
                reason: "符号链接或路径不可安全读取".to_string(),
            });
            continue;
        }
        match fs::read_dir(&root) {
            Ok(entries) => {
                for entry in entries {
                    match entry {
                        Ok(entry) => {
                            let path = entry.path();
                            if path
                                .extension()
                                .is_some_and(|candidate| candidate == extension)
                            {
                                if let Ok(relative) = path.strip_prefix(home) {
                                    let relative = logical_path(relative);
                                    rules.push(inspect_auxiliary_rule(home, agent, &relative));
                                }
                            }
                        }
                        Err(_) => rules.push(NativeRuleSource {
                            agent: agent.to_string(),
                            kind: "auxiliary".to_string(),
                            path: format!("~/{relative}"),
                            state: "unreadable".to_string(),
                            reason: "无法检查规则目录条目".to_string(),
                        }),
                    }
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => rules.push(NativeRuleSource {
                agent: agent.to_string(),
                kind: "auxiliary".to_string(),
                path: format!("~/{relative}"),
                state: "unreadable".to_string(),
                reason: "无法列出规则目录".to_string(),
            }),
        }
    }
    rules.sort_by(|left, right| {
        left.agent
            .cmp(&right.agent)
            .then(left.path.cmp(&right.path))
    });
    rules
}

fn inspect_auxiliary_rule(home: &Path, agent: &str, relative: &str) -> NativeRuleSource {
    let path = home.join(relative);
    let (state, reason) = if paths::reject_symlinks_below(home, &path).is_err() {
        ("unsafe", "符号链接或路径不可安全读取")
    } else {
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_file() && metadata.len() > 0 => {
                ("present", "仅发现，未验证导入兼容性")
            }
            Ok(metadata) if metadata.file_type().is_file() => ("empty", "文件为空"),
            Ok(_) => ("unsafe", "目标不是普通文件"),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                ("missing", "未找到目标文件")
            }
            Err(_) => ("unreadable", "无法检查目标文件"),
        }
    };
    NativeRuleSource {
        agent: agent.to_string(),
        kind: "auxiliary".to_string(),
        path: format!("~/{relative}"),
        state: state.to_string(),
        reason: reason.to_string(),
    }
}

const NATIVE_SKILL_TARGETS: &[(&str, &str)] = &[
    ("claude", ".claude/skills"),
    ("codex", ".codex/skills"),
    ("copilot", ".copilot/skills"),
    ("crush", ".config/crush/skills"),
    ("cursor", ".cursor/skills"),
    ("cursor", ".agents/skills"),
    ("cursor", ".claude/skills"),
    ("cursor", ".codex/skills"),
    ("factory", ".factory/skills"),
    ("junie", ".junie/skills"),
    ("kilocode", ".kilo/skills"),
    ("kiro", ".kiro/skills"),
    ("opencode", ".config/opencode/skills"),
    ("qwen", ".qwen/skills"),
    ("shared", ".agents/skills"),
    ("antigravity", ".gemini/config/skills"),
    ("antigravity-ide", ".gemini/antigravity/skills"),
    ("antigravity-ide", ".gemini/antigravity/global_skills"),
    ("antigravity-cli", ".gemini/antigravity-cli/skills"),
];

fn discover_native_skills(home: &Path) -> Vec<NativeSkillSource> {
    let mut sources = Vec::new();
    for agent in
        adapter::names()
            .into_iter()
            .chain(["antigravity-ide", "antigravity-cli", "shared"])
    {
        let targets = NATIVE_SKILL_TARGETS
            .iter()
            .filter(|(name, _)| *name == agent)
            .map(|(_, relative)| *relative)
            .collect::<Vec<_>>();
        if targets.is_empty() {
            sources.push(NativeSkillSource {
                agent: agent.to_string(),
                path: String::new(),
                state: "unavailable".to_string(),
                reason: "无已核实的专属目录；通用技能见 shared".to_string(),
                items: Vec::new(),
            });
        }
        for relative in targets {
            let root = home.join(relative);
            let mut source = NativeSkillSource {
                agent: agent.to_string(),
                path: format!("~/{relative}"),
                state: "missing".to_string(),
                reason: "未找到目录".to_string(),
                items: Vec::new(),
            };
            match fs::symlink_metadata(&root) {
                Ok(_) if !within_native_home(home, &root) => {
                    source.state = "unsafe".to_string();
                    source.reason = "目标指向用户目录外或无法安全解析".to_string();
                }
                Ok(_) if !root.is_dir() => {
                    source.state = "unsafe".to_string();
                    source.reason = "目标不是目录".to_string();
                }
                Ok(_) => {
                    let mut visited = BTreeSet::new();
                    let had_errors =
                        collect_skill_items(home, &root, 0, &mut visited, &mut source.items);
                    source
                        .items
                        .sort_by(|left, right| left.path.cmp(&right.path));
                    source.state = if had_errors {
                        "partial"
                    } else if source.items.is_empty() {
                        "empty"
                    } else {
                        "present"
                    }
                    .to_string();
                    source.reason = "仅发现目录条目，未验证导入兼容性".to_string();
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(_) => {
                    source.state = "unreadable".to_string();
                    source.reason = "无法检查目录".to_string();
                }
            }
            sources.push(source);
        }
    }
    sources
}

fn within_native_home(home: &Path, path: &Path) -> bool {
    match (fs::canonicalize(home), fs::canonicalize(path)) {
        (Ok(home), Ok(path)) => path.starts_with(home),
        _ => false,
    }
}

fn logical_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn skill_item(home: &Path, path: &Path, state: &str) -> NativeSkillItem {
    NativeSkillItem {
        name: path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default(),
        path: path
            .strip_prefix(home)
            .map(|relative| format!("~/{}", logical_path(relative)))
            .unwrap_or_default(),
        state: state.to_string(),
    }
}

fn collect_skill_items(
    home: &Path,
    directory: &Path,
    depth: usize,
    visited: &mut BTreeSet<PathBuf>,
    items: &mut Vec<NativeSkillItem>,
) -> bool {
    if depth >= 8 {
        items.push(skill_item(home, directory, "unverified"));
        return true;
    }
    let Ok(resolved) = fs::canonicalize(directory) else {
        items.push(skill_item(home, directory, "unreadable"));
        return true;
    };
    if !visited.insert(resolved) {
        items.push(skill_item(home, directory, "unverified"));
        return true;
    }
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(_) => {
            items.push(skill_item(home, directory, "unreadable"));
            return true;
        }
    };
    let mut had_errors = false;
    for entry in entries {
        let Ok(entry) = entry else {
            items.push(skill_item(home, directory, "unreadable"));
            had_errors = true;
            continue;
        };
        let path = entry.path();
        if !within_native_home(home, &path) {
            items.push(skill_item(home, &path, "unsafe"));
            had_errors = true;
            continue;
        }
        if path.is_dir() {
            let manifest = path.join("SKILL.md");
            match fs::symlink_metadata(&manifest) {
                Ok(_) if !within_native_home(home, &manifest) => {
                    items.push(skill_item(home, &path, "unsafe"));
                    had_errors = true;
                }
                Ok(_) if manifest.is_file() => {
                    let state = match fs::metadata(&manifest) {
                        Ok(metadata) if metadata.len() > 0 => "present",
                        Ok(_) => "empty",
                        Err(_) => "unreadable",
                    };
                    items.push(skill_item(home, &path, state));
                    had_errors |= state == "unreadable";
                }
                Ok(_) => {
                    items.push(skill_item(home, &path, "unverified"));
                    had_errors = true;
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    had_errors |= collect_skill_items(home, &path, depth + 1, visited, items);
                }
                Err(_) => {
                    items.push(skill_item(home, &path, "unreadable"));
                    had_errors = true;
                }
            }
        } else if depth == 0 && path.extension().is_some_and(|extension| extension == "md") {
            items.push(skill_item(home, &path, "unverified"));
        }
    }
    had_errors
}

fn discover_native_workflows(home: &Path) -> Vec<NativeWorkflowSource> {
    let relative = ".gemini/antigravity/global_workflows";
    let root = home.join(relative);
    let mut ide = NativeWorkflowSource {
        agent: "antigravity-ide".to_string(),
        path: format!("~/{relative}"),
        state: "missing".to_string(),
        reason: "未找到全局 Workflow 目录".to_string(),
        items: Vec::new(),
    };
    match fs::symlink_metadata(&root) {
        Ok(_) if !within_native_home(home, &root) || !root.is_dir() => {
            ide.state = "unsafe".to_string();
            ide.reason = "目录无法安全读取".to_string();
        }
        Ok(_) => match fs::read_dir(&root) {
            Ok(entries) => {
                let mut partial = false;
                for entry in entries {
                    let Ok(entry) = entry else {
                        partial = true;
                        continue;
                    };
                    let path = entry.path();
                    if path.extension().is_none_or(|extension| extension != "md") {
                        continue;
                    }
                    let state = if !within_native_home(home, &path) {
                        "unsafe"
                    } else {
                        match fs::metadata(&path) {
                            Ok(metadata) if metadata.is_file() && metadata.len() > 0 => "present",
                            Ok(metadata) if metadata.is_file() => "empty",
                            _ => "unreadable",
                        }
                    };
                    partial |= state == "unsafe" || state == "unreadable";
                    ide.items.push(skill_item(home, &path, state));
                }
                ide.items.sort_by(|left, right| left.path.cmp(&right.path));
                ide.state = if partial {
                    "partial"
                } else if ide.items.is_empty() {
                    "empty"
                } else {
                    "present"
                }
                .to_string();
                ide.reason = "仅发现 Markdown 文件，尚未导入".to_string();
            }
            Err(_) => {
                ide.state = "unreadable".to_string();
                ide.reason = "无法列出 Workflow 目录".to_string();
            }
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => {
            ide.state = "unreadable".to_string();
            ide.reason = "无法检查 Workflow 目录".to_string();
        }
    }
    let mut sources = vec![ide];
    for agent in ["antigravity", "antigravity-cli"] {
        sources.push(NativeWorkflowSource {
            agent: agent.to_string(),
            path: String::new(),
            state: "unavailable".to_string(),
            reason: "无已核实的独立 Workflow 目录；请查看 Skills".to_string(),
            items: Vec::new(),
        });
    }
    sources
}

const NATIVE_HOOK_TARGETS: &[(&str, &str, bool)] = &[
    ("antigravity", ".gemini/config/hooks.json", false),
    ("antigravity-ide", ".gemini/config/hooks.json", false),
    (
        "antigravity-cli",
        ".gemini/antigravity-cli/settings.json",
        true,
    ),
    ("claude", ".claude/settings.json", true),
    ("cursor", ".cursor/hooks.json", false),
    ("gemini", ".gemini/settings.json", true),
];

fn discover_native_hooks(home: &Path) -> Vec<NativeHookSource> {
    let mut sources = Vec::new();
    for agent in adapter::names() {
        let targets = NATIVE_HOOK_TARGETS
            .iter()
            .filter(|(name, _, _)| *name == agent)
            .collect::<Vec<_>>();
        if targets.is_empty() {
            sources.push(NativeHookSource {
                agent: agent.to_string(),
                path: String::new(),
                state: "unavailable".to_string(),
                reason: "无已核实的用户级 Hook 配置路径".to_string(),
            });
        }
        for (_, relative, settings) in targets {
            sources.push(inspect_hook_file(home, agent, relative, *settings));
        }
    }
    sources.push(inspect_hook_file(
        home,
        "antigravity-ide",
        ".gemini/config/hooks.json",
        false,
    ));
    sources.push(inspect_hook_file(
        home,
        "antigravity-cli",
        ".gemini/antigravity-cli/settings.json",
        true,
    ));
    let plugins_relative = ".gemini/antigravity-cli/plugins";
    let plugins_root = home.join(plugins_relative);
    let mut plugins = NativeHookSource {
        agent: "antigravity-cli".to_string(),
        path: format!("~/{plugins_relative}"),
        state: "missing".to_string(),
        reason: "未找到 CLI 插件目录".to_string(),
    };
    if paths::reject_symlinks_below(home, &plugins_root).is_err() {
        plugins.state = "unsafe".to_string();
        plugins.reason = "目录含符号链接，未扫描插件".to_string();
    } else {
        match fs::read_dir(&plugins_root) {
            Ok(entries) => {
                plugins.state = "unverified".to_string();
                plugins.reason = "已发现插件目录，逐个检查 hooks.json".to_string();
                for entry in entries {
                    let Ok(entry) = entry else {
                        plugins.state = "unreadable".to_string();
                        plugins.reason = "无法读取部分插件目录条目".to_string();
                        continue;
                    };
                    let Ok(file_type) = entry.file_type() else {
                        plugins.state = "unreadable".to_string();
                        continue;
                    };
                    if file_type.is_dir() {
                        let relative = format!(
                            "{plugins_relative}/{}/hooks.json",
                            entry.file_name().to_string_lossy()
                        );
                        sources.push(inspect_hook_file(home, "antigravity-cli", &relative, false));
                    } else if file_type.is_symlink() {
                        sources.push(NativeHookSource {
                            agent: "antigravity-cli".to_string(),
                            path: format!(
                                "~/{plugins_relative}/{}",
                                entry.file_name().to_string_lossy()
                            ),
                            state: "unsafe".to_string(),
                            reason: "插件目录是符号链接，未扫描".to_string(),
                        });
                    }
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => {
                plugins.state = "unreadable".to_string();
                plugins.reason = "无法列出 CLI 插件目录".to_string();
            }
        }
    }
    sources.push(plugins);
    sources.sort_by(|left, right| {
        left.agent
            .cmp(&right.agent)
            .then(left.path.cmp(&right.path))
    });
    sources
}

fn inspect_hook_file(home: &Path, agent: &str, relative: &str, settings: bool) -> NativeHookSource {
    let path = home.join(relative);
    let (state, reason) = if paths::reject_symlinks_below(home, &path).is_err() {
        ("unsafe", "文件含符号链接，未读取")
    } else {
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_file() && metadata.len() > 0 => {
                if settings && agent != "antigravity-cli" {
                    if metadata.len() > MAX_NATIVE_MCP_BYTES {
                        ("oversized", "配置文件超过 1 MiB 扫描上限")
                    } else {
                        match fs::read_to_string(&path) {
                            Ok(body) if body.len() as u64 > MAX_NATIVE_MCP_BYTES => {
                                ("oversized", "配置文件超过 1 MiB 扫描上限")
                            }
                            Ok(body) => match json5::from_str::<Value>(&body) {
                                Ok(Value::Object(config)) => match config.get("hooks") {
                                    Some(Value::Object(hooks)) if !hooks.is_empty() => {
                                        ("present", "发现 Hooks 字段，定义与命令尚未核对")
                                    }
                                    None | Some(Value::Null) | Some(Value::Object(_)) => {
                                        ("no-hooks", "配置文件没有 Hook 定义")
                                    }
                                    Some(_) => ("invalid", "Hooks 字段格式无效"),
                                },
                                _ => ("invalid", "配置文件格式无效"),
                            },
                            Err(_) => ("unreadable", "无法读取配置文件"),
                        }
                    }
                } else if settings {
                    ("unverified", "配置文件存在，Hooks 字段尚未核对")
                } else {
                    ("present", "仅发现 Hook 文件，定义与命令尚未核对")
                }
            }
            Ok(metadata) if metadata.file_type().is_file() => ("empty", "文件为空"),
            Ok(_) => ("unsafe", "目标不是普通文件"),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => ("missing", "未找到文件"),
            Err(_) => ("unreadable", "无法检查文件"),
        }
    };
    NativeHookSource {
        agent: agent.to_string(),
        path: format!("~/{relative}"),
        state: state.to_string(),
        reason: reason.to_string(),
    }
}

const NATIVE_SUBAGENT_TARGETS: &[(&str, &str)] = &[
    ("antigravity", ".gemini/config/agents"),
    ("antigravity-cli", ".gemini/config/agents"),
    ("claude", ".claude/agents"),
    ("cursor", ".cursor/agents"),
    ("cursor", ".claude/agents"),
    ("cursor", ".codex/agents"),
];

fn discover_native_subagents(home: &Path) -> Vec<NativeSkillSource> {
    let mut sources = Vec::new();
    for agent in adapter::names()
        .into_iter()
        .chain(["antigravity-cli", "antigravity-ide"])
    {
        let targets = NATIVE_SUBAGENT_TARGETS
            .iter()
            .filter(|(name, _)| *name == agent)
            .collect::<Vec<_>>();
        if targets.is_empty() {
            sources.push(NativeSkillSource {
                agent: agent.to_string(),
                path: String::new(),
                state: "unavailable".to_string(),
                reason: if agent == "antigravity-ide" {
                    "未核实 IDE 独立的全局 Subagent 目录"
                } else {
                    "无已核实的用户级 Subagent 目录"
                }
                .to_string(),
                items: Vec::new(),
            });
        }
        for (_, relative) in targets {
            sources.push(inspect_subagent_dir(home, agent, relative));
        }
    }
    for (agent, relative) in [
        ("antigravity", ".gemini/config/plugins"),
        ("antigravity-cli", ".gemini/antigravity-cli/plugins"),
    ] {
        let root = home.join(relative);
        let mut plugin_source = NativeSkillSource {
            agent: agent.to_string(),
            path: format!("~/{relative}"),
            state: "missing".to_string(),
            reason: "未找到全局插件目录".to_string(),
            items: Vec::new(),
        };
        if fs::symlink_metadata(&root).is_ok() && !within_native_home(home, &root) {
            plugin_source.state = "unsafe".to_string();
            plugin_source.reason = "插件目录不可安全读取".to_string();
        } else {
            match fs::read_dir(&root) {
                Ok(entries) => {
                    plugin_source.state = "empty".to_string();
                    plugin_source.reason = "未找到含 agents/ 的插件".to_string();
                    for entry in entries {
                        let Ok(entry) = entry else {
                            plugin_source.state = "partial".to_string();
                            plugin_source.reason = "部分插件目录不可读取".to_string();
                            continue;
                        };
                        let path = entry.path();
                        if !within_native_home(home, &path) {
                            sources.push(NativeSkillSource {
                                agent: agent.to_string(),
                                path: format!(
                                    "~/{relative}/{}",
                                    entry.file_name().to_string_lossy()
                                ),
                                state: "unsafe".to_string(),
                                reason: "插件目录不可安全读取".to_string(),
                                items: Vec::new(),
                            });
                            plugin_source.state = "partial".to_string();
                            plugin_source.reason = "部分插件目录不可安全读取".to_string();
                            continue;
                        }
                        if !path.is_dir() {
                            continue;
                        }
                        let agents_path = path.join("agents");
                        if fs::symlink_metadata(&agents_path).is_ok() {
                            let relative_path = format!(
                                "{relative}/{}/agents",
                                entry.file_name().to_string_lossy()
                            );
                            sources.push(inspect_subagent_dir(home, agent, &relative_path));
                            if plugin_source.state != "partial" {
                                plugin_source.state = "present".to_string();
                                plugin_source.reason =
                                    "插件 Agent 来源见下方各目录；是否启用尚未核实".to_string();
                            }
                        }
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(_) => {
                    plugin_source.state = "unreadable".to_string();
                    plugin_source.reason = "无法列出插件目录".to_string();
                }
            }
        }
        sources.push(plugin_source);
    }
    sources.sort_by(|left, right| {
        left.agent
            .cmp(&right.agent)
            .then(left.path.cmp(&right.path))
    });
    sources
}

fn inspect_subagent_dir(home: &Path, agent: &str, relative: &str) -> NativeSkillSource {
    let root = home.join(relative);
    let mut source = NativeSkillSource {
        agent: agent.to_string(),
        path: format!("~/{relative}"),
        state: "missing".to_string(),
        reason: "未找到目录".to_string(),
        items: Vec::new(),
    };
    match fs::symlink_metadata(&root) {
        Ok(_) if !within_native_home(home, &root) || !root.is_dir() => {
            source.state = "unsafe".to_string();
            source.reason = "目录无法安全读取".to_string();
        }
        Ok(_) => match fs::read_dir(&root) {
            Ok(entries) => {
                let mut partial = false;
                for entry in entries {
                    let Ok(entry) = entry else {
                        partial = true;
                        continue;
                    };
                    let path = entry.path();
                    let definition = if path.extension().is_some_and(|ext| ext == "md") {
                        path
                    } else if path.is_dir()
                        || fs::symlink_metadata(&path)
                            .is_ok_and(|meta| meta.file_type().is_symlink())
                    {
                        path.join("agent.md")
                    } else {
                        continue;
                    };
                    let state = if !within_native_home(home, &definition) {
                        "unsafe"
                    } else {
                        match fs::metadata(&definition) {
                            Ok(metadata) if metadata.is_file() && metadata.len() > 0 => "present",
                            Ok(metadata) if metadata.is_file() => "empty",
                            Ok(_) => "unsafe",
                            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                            Err(_) => "unreadable",
                        }
                    };
                    partial |= state == "unsafe" || state == "unreadable";
                    source.items.push(skill_item(home, &definition, state));
                }
                source
                    .items
                    .sort_by(|left, right| left.path.cmp(&right.path));
                source.state = if partial {
                    "partial"
                } else if source.items.is_empty() {
                    "empty"
                } else {
                    "present"
                }
                .to_string();
                source.reason =
                    "仅检查 Markdown 文件元数据，未读取 Prompt 或验证插件启用状态".to_string();
            }
            Err(_) => {
                source.state = "unreadable".to_string();
                source.reason = "无法列出目录".to_string();
            }
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => {
            source.state = "unreadable".to_string();
            source.reason = "无法检查目录".to_string();
        }
    }
    source
}

pub(super) fn parse_native_mcp_body(
    agent: &str,
    body: &str,
) -> Result<Vec<source::McpServer>, String> {
    let mut output = Vec::new();
    if agent == "codex" {
        let value: toml::Value = body
            .parse()
            .map_err(|error| format!("parse native TOML: {error}"))?;
        if let Some(servers) = value.get("mcp_servers") {
            let servers = servers
                .as_table()
                .ok_or_else(|| "invalid MCP server map".to_string())?;
            for (id, value) in servers {
                let table = value
                    .as_table()
                    .ok_or_else(|| "invalid MCP server entry".to_string())?;
                output.push(source::McpServer {
                    id: id.clone(),
                    name: id.clone(),
                    description: String::new(),
                    transport: table
                        .get("type")
                        .and_then(toml::Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    command: table
                        .get("command")
                        .and_then(toml::Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    url: table
                        .get("url")
                        .and_then(toml::Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    args: toml_string_array(table.get("args")),
                    recipe: table
                        .get("recipe")
                        .and_then(toml::Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    runner: table
                        .get("runner")
                        .and_then(toml::Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    package: table
                        .get("package")
                        .and_then(toml::Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    env: toml_string_map(table.get("env")),
                    headers: toml_string_map(
                        table.get("http_headers").or_else(|| table.get("headers")),
                    ),
                });
            }
        }
    } else {
        let raw: Value =
            json5::from_str(body).map_err(|error| format!("parse native JSON: {error}"))?;
        let mut owned = json!({});
        jsonkeys::merge_owned_keys(&mut owned, &raw, &["mcpServers"])?;
        if let Some(servers) = owned.get("mcpServers") {
            let servers = servers
                .as_object()
                .ok_or_else(|| "invalid MCP server map".to_string())?;
            for (id, value) in servers {
                let table = value
                    .as_object()
                    .ok_or_else(|| "invalid MCP server entry".to_string())?;
                output.push(source::McpServer {
                    id: id.clone(),
                    name: id.clone(),
                    description: String::new(),
                    transport: json_string(table, "type"),
                    command: json_string(table, "command"),
                    url: ["url", "httpUrl", "serverUrl"]
                        .into_iter()
                        .map(|key| json_string(table, key))
                        .find(|value| !value.is_empty())
                        .unwrap_or_default(),
                    args: json_string_array(table.get("args")),
                    recipe: json_string(table, "recipe"),
                    runner: json_string(table, "runner"),
                    package: json_string(table, "package"),
                    env: json_string_map(table.get("env")),
                    headers: json_string_map(table.get("headers")),
                });
            }
        }
    }
    Ok(output)
}

fn toml_string_map(value: Option<&toml::Value>) -> BTreeMap<String, String> {
    value
        .and_then(toml::Value::as_table)
        .map(|table| {
            table
                .iter()
                .filter_map(|(key, value)| {
                    value.as_str().map(|value| (key.clone(), value.to_string()))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn toml_string_array(value: Option<&toml::Value>) -> Vec<String> {
    value
        .and_then(toml::Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(toml::Value::as_str)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn json_string(table: &serde_json::Map<String, Value>, key: &str) -> String {
    table
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn json_string_map(value: Option<&Value>) -> BTreeMap<String, String> {
    value
        .and_then(Value::as_object)
        .map(|table| {
            table
                .iter()
                .filter_map(|(key, value)| {
                    value.as_str().map(|value| (key.clone(), value.to_string()))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn json_string_array(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn mcp_summary(label: &str, server: &source::McpServer, source_kind: &str) -> GlobalMcpItem {
    let native = !label.is_empty();
    let transport = if server.transport.is_empty() {
        if server.url.is_empty() {
            "stdio"
        } else {
            "http"
        }
    } else {
        &server.transport
    };
    let recipe = mcp::recipe_for(server);
    GlobalMcpItem {
        id: if native {
            format!("{label}:{}", server.id)
        } else {
            server.id.clone()
        },
        name: if server.name.is_empty() {
            server.id.clone()
        } else {
            server.name.clone()
        },
        description: if !server.description.is_empty() {
            server.description.clone()
        } else if native {
            "来自本机 Agent 原生 MCP 配置，建议迁移到 canonical 后再统一应用。".to_string()
        } else {
            "来自 canonical MCP 配置的本地定义。".to_string()
        },
        transport: transport.to_string(),
        endpoint: safe_endpoint(server),
        secret_refs: secret_refs(server),
        source: source_kind.to_string(),
        scope: if source_kind == "global" {
            "global"
        } else {
            "agent"
        }
        .to_string(),
        recipe: recipe.id,
        recipe_status: recipe.status,
        credential_state: mcp::credential_state(server).to_string(),
        agents: if native {
            vec![label.to_string()]
        } else {
            Vec::new()
        },
        category: if source_kind == "global" {
            "global"
        } else {
            "agent"
        }
        .to_string(),
        can_promote: source_kind != "global",
        server_id: server.id.clone(),
        credentials: credential_items(server),
        adapters: if native {
            vec![McpAdapter {
                agent: label.to_string(),
                status: "native".to_string(),
            }]
        } else {
            Vec::new()
        },
    }
}

fn consolidate_native_mcp(
    items: Vec<GlobalMcpItem>,
    sources: &[NativeMcpSource],
) -> Vec<GlobalMcpItem> {
    let mut consolidated = Vec::new();
    for item in items {
        let key = format!(
            "{}|{}|{}|{}|{:?}|{}",
            item.server_id,
            item.transport,
            item.endpoint,
            item.recipe,
            item.secret_refs,
            item.credential_state
        );
        if let Some(existing) = consolidated
            .iter_mut()
            .find(|candidate: &&mut GlobalMcpItem| {
                format!(
                    "{}|{}|{}|{}|{:?}|{}",
                    candidate.server_id,
                    candidate.transport,
                    candidate.endpoint,
                    candidate.recipe,
                    candidate.secret_refs,
                    candidate.credential_state
                ) == key
            })
        {
            for agent in item.agents {
                if !existing.agents.contains(&agent) {
                    existing.agents.push(agent);
                }
            }
            for adapter in item.adapters {
                if !existing
                    .adapters
                    .iter()
                    .any(|candidate| candidate.agent == adapter.agent)
                {
                    existing.adapters.push(adapter);
                }
            }
            continue;
        }
        consolidated.push(item);
    }
    for item in &mut consolidated {
        if item.agents.len() > 1 {
            item.category = "shared".to_string();
        }
        if item.agents.iter().any(|agent| agent == "antigravity") {
            let ide_ready = sources
                .iter()
                .any(|source| source.agent == "antigravity-ide" && source.state == "parsed");
            let cli_ready = sources
                .iter()
                .any(|source| source.agent == "antigravity-cli" && source.state == "parsed");
            if ide_ready && cli_ready {
                item.agents.retain(|agent| agent != "antigravity");
                item.agents
                    .extend(["antigravity-ide".to_string(), "antigravity-cli".to_string()]);
                item.adapters
                    .retain(|adapter| adapter.agent != "antigravity");
                item.adapters.extend([
                    McpAdapter {
                        agent: "antigravity-ide".to_string(),
                        status: "native".to_string(),
                    },
                    McpAdapter {
                        agent: "antigravity-cli".to_string(),
                        status: "native".to_string(),
                    },
                ]);
                if sources
                    .iter()
                    .any(|source| source.agent == "antigravity" && source.state == "parsed")
                {
                    item.agents.push("antigravity".to_string());
                    item.adapters.push(McpAdapter {
                        agent: "antigravity".to_string(),
                        status: "native".to_string(),
                    });
                }
                item.agents.sort();
                item.agents.dedup();
                item.adapters
                    .sort_by(|left, right| left.agent.cmp(&right.agent));
                item.adapters
                    .dedup_by(|left, right| left.agent == right.agent);
                item.category = "shared".to_string();
            }
        }
    }
    consolidated.sort_by(|left, right| left.name.cmp(&right.name));
    consolidated
}

fn project_mcp_summary(server: &source::McpServer, agents: &[String]) -> ProjectMcpItem {
    let recipe = mcp::recipe_for(server);
    ProjectMcpItem {
        id: server.id.clone(),
        name: server.name.clone(),
        transport: if server.transport.is_empty() {
            if server.url.is_empty() {
                "stdio"
            } else {
                "http"
            }
        } else {
            &server.transport
        }
        .to_string(),
        endpoint: safe_endpoint(server),
        secret_refs: secret_refs(server),
        recipe: recipe.id,
        recipe_status: recipe.status,
        credential_state: mcp::credential_state(server).to_string(),
        credentials: credential_items(server),
        adapters: agents
            .iter()
            .map(|agent| McpAdapter {
                agent: agent.clone(),
                status: "project".to_string(),
            })
            .collect(),
    }
}

fn safe_endpoint(server: &source::McpServer) -> String {
    if !server.url.is_empty() {
        if let Ok(parsed) = Url::parse(&server.url) {
            let origin = parsed.origin().ascii_serialization();
            if origin != "null" {
                return origin;
            }
        }
        return "远程端点（已脱敏）".to_string();
    }
    if !server.command.is_empty() {
        return "本地命令（已脱敏）".to_string();
    }
    server.transport.clone()
}

fn secret_refs(server: &source::McpServer) -> Vec<String> {
    let mut refs = server
        .env
        .values()
        .chain(server.headers.values())
        .filter(|value| value.starts_with("${secret:") || value.starts_with("${env:"))
        .cloned()
        .collect::<Vec<_>>();
    refs.sort();
    refs
}

fn credential_items(server: &source::McpServer) -> Vec<McpCredential> {
    let recipe = mcp::recipe_for(server);
    let mut keys = BTreeSet::new();
    keys.extend(recipe.required_secrets);
    keys.extend(
        server
            .env
            .keys()
            .chain(server.headers.keys())
            .filter(|key| mcp::is_secret_key(key.as_str()))
            .cloned(),
    );
    keys.into_iter()
        .map(|key| {
            let value = server.env.get(&key).or_else(|| server.headers.get(&key));
            match value {
                Some(value) if value.starts_with("${secret:") => McpCredential {
                    key,
                    source: "secret-vault".to_string(),
                    status: "configured".to_string(),
                    reference: Some(value.clone()),
                },
                Some(value) if value.starts_with("${env:") => McpCredential {
                    key,
                    source: "environment".to_string(),
                    status: "configured".to_string(),
                    reference: Some(value.clone()),
                },
                Some(_) => McpCredential {
                    key,
                    source: "literal".to_string(),
                    status: "plaintext-blocked".to_string(),
                    reference: None,
                },
                None => McpCredential {
                    key,
                    source: "not-configured".to_string(),
                    status: "missing".to_string(),
                    reference: None,
                },
            }
        })
        .collect()
}

fn global_mcp_adapters(agents: &[AgentSummary]) -> Vec<McpAdapter> {
    let mut result: Vec<McpAdapter> = Vec::new();
    for agent in agents {
        if agent.id == "antigravity" {
            // antigravity 展开为两个独立行：
            // 1. Antigravity IDE （共享 mcp_config.json）
            result.push(McpAdapter {
                agent: "antigravity-ide".to_string(),
                status: match agent.capabilities.components.get("mcp").map(String::as_str) {
                    Some("full") | Some("partial") => "partial",
                    Some("unsupported") => "unsupported",
                    _ => "unverified",
                }
                .to_string(),
            });
            // 2. Antigravity 桌面 App（独立路径）
            result.push(McpAdapter {
                agent: "antigravity".to_string(),
                status: match agent.capabilities.components.get("mcp").map(String::as_str) {
                    Some("full") | Some("partial") => "partial",
                    Some("unsupported") => "unsupported",
                    _ => "unverified",
                }
                .to_string(),
            });
        } else {
            result.push(McpAdapter {
                agent: agent.id.clone(),
                status: match agent.capabilities.components.get("mcp").map(String::as_str) {
                    Some("full") => "supported",
                    Some("partial") => "partial",
                    Some("unsupported") => "unsupported",
                    _ => "unverified",
                }
                .to_string(),
            });
        }
    }
    result
}

fn canonical_agent_id(name: &str) -> &str {
    match name {
        "antigravity-ide" | "antigravity-cli" => "antigravity",
        _ => name,
    }
}

fn enabled_agent_names(agents: &BTreeMap<String, source::Agent>) -> Vec<String> {
    agents
        .iter()
        .filter(|(_, agent)| agent.enabled)
        .map(|(name, _)| name.clone())
        .collect()
}

fn agent_summary(name: &str) -> AgentSummary {
    let (components, note) = capability_report(name);
    let health = if !components.is_empty() && components.values().all(|state| state == "full") {
        "good"
    } else {
        "warn"
    };
    AgentSummary {
        id: name.to_string(),
        name: display_agent_name(name).to_string(),
        vendor: "本机适配器".to_string(),
        version: "已检测".to_string(),
        health: health.to_string(),
        sync_state: if health == "good" {
            "discovered"
        } else {
            "attention"
        }
        .to_string(),
        capabilities: CapabilityReport {
            memory: components.get("rules").cloned().unwrap_or_default(),
            components,
            note,
        },
        last_applied: "未读取应用记录".to_string(),
    }
}

fn capability_report(name: &str) -> (BTreeMap<String, String>, String) {
    let (states, note): (&[(&str, &str)], &str) = match name {
        "antigravity" => (
            &[
                ("commands", "unsupported"),
                ("hooks", "unsupported"),
                ("mcp", "partial"),
                ("rules", "full"),
                ("skills", "full"),
                ("subagents", "unsupported"),
                ("workflows", "unknown"),
            ],
            "Rules and skills are native; MCP is projected and commands, hooks, and subagents have no honest native target.",
        ),
        "codex" => (
            &[
                ("commands", "partial"),
                ("hooks", "partial"),
                ("mcp", "full"),
                ("rules", "full"),
                ("skills", "full"),
                ("subagents", "partial"),
                ("workflows", "unknown"),
            ],
            "Native project rules, skills, MCP, and memory; commands, hooks, and subagents have adapter-specific limits.",
        ),
        "cursor" => (
            &[
                ("commands", "partial"),
                ("hooks", "partial"),
                ("mcp", "full"),
                ("rules", "full"),
                ("skills", "full"),
                ("subagents", "partial"),
                ("workflows", "unknown"),
            ],
            "Native rules, skills, MCP, commands, hooks, and project subagents with documented field and scope loss.",
        ),
        "gemini" => (
            &[
                ("commands", "partial"),
                ("hooks", "partial"),
                ("mcp", "partial"),
                ("rules", "full"),
                ("skills", "unsupported"),
                ("subagents", "partial"),
                ("workflows", "unknown"),
            ],
            "Native rules and MCP; skills are unsupported because Gemini uses extensions, and several components are projected.",
        ),
        _ => (
            &[
                ("commands", "unknown"),
                ("hooks", "unknown"),
                ("mcp", "unknown"),
                ("rules", "unknown"),
                ("skills", "unknown"),
                ("subagents", "unknown"),
                ("workflows", "unknown"),
            ],
            "Capability support has not been verified for this adapter.",
        ),
    };
    (
        states
            .iter()
            .map(|(component, state)| ((*component).to_string(), (*state).to_string()))
            .collect(),
        note.to_string(),
    )
}

fn display_agent_name(name: &str) -> &str {
    match name {
        "codex" => "Codex",
        "cursor" => "Cursor",
        "gemini" => "Gemini CLI",
        "antigravity-ide" => "Antigravity IDE",
        "antigravity-cli" => "Antigravity CLI",
        "antigravity" => "Antigravity",
        _ => name,
    }
}

fn project_profile(name: &str) -> (String, Vec<String>) {
    let stack = match name {
        "makebestmusic-nextjs" => &[
            "Next.js 14.2.2 App Router",
            "React 18",
            "JavaScript/JSX plus TypeScript",
            "Tailwind CSS 3.4",
            "local shadcn/Radix UI primitives",
            "Ant Design 5",
            "Sass/SCSS",
            "Zustand + React Context + legacy Redux",
            "Supabase",
            "next-intl",
            "FFmpeg + Wavesurfer",
        ][..],
        "niew-nextjs" => &[
            "Next.js 15 App Router",
            "React 19",
            "JavaScript/JSX plus TypeScript",
            "Tailwind CSS 3.4",
            "Ant Design 5",
            "local shadcn/Radix UI primitives",
            "Sass/SCSS",
            "Zustand + React Context + legacy Redux",
            "Supabase",
            "next-intl",
            "FFmpeg + Wavesurfer",
            "LoveAI + Suno + Replicate",
            "Stripe + Paddle",
        ][..],
        "songai" => &[
            "Next.js 15 App Router",
            "React 19",
            "JavaScript/JSX plus TypeScript",
            "Tailwind CSS 3.4",
            "Ant Design 5",
            "local shadcn/Radix UI primitives",
            "Sass/SCSS",
            "React Context + Zustand + legacy Redux",
            "Supabase",
            "next-intl",
            "FFmpeg + Wavesurfer",
            "Replicate + AI providers",
            "Stripe + Paddle + USDT",
        ][..],
        _ => &[],
    };
    (
        name.to_string(),
        stack.iter().map(|item| (*item).to_string()).collect(),
    )
}

pub(super) fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .replace_nanosecond(0)
        .unwrap_or_else(|_| OffsetDateTime::now_utc())
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

#[cfg(test)]
mod tests {
    use super::{agent_summary, logical_path, now_rfc3339, parse_native_mcp_body, Core};
    use crate::core::adapter;
    use crate::core::desktop::model::Options;
    use crate::core::state::{save_project_registry, ProjectRegistry, RegisteredProject};
    use crate::core::test_support::TestDir;
    use std::fs;
    use std::path::Path;

    fn options(dir: &TestDir) -> Options {
        Options {
            home: dir.path().join("global"),
            projects_root: dir.path().join("projects"),
            native_root: dir.path().join("user"),
        }
    }

    #[test]
    fn logical_paths_use_forward_slashes() {
        assert_eq!(
            logical_path(Path::new(r".codex\rules\default.rules")),
            ".codex/rules/default.rules"
        );
    }

    #[test]
    fn snapshot_discovers_project_canonical_trees() {
        let dir = TestDir::new("snapshot-projects");
        let opts = options(&dir);
        fs::create_dir_all(opts.projects_root.join("example/.agentsync")).unwrap();
        fs::write(
            opts.projects_root.join("example/.agentsync/agentsync.toml"),
            "[agents]\ncodex = { enabled = true }\n",
        )
        .unwrap();
        let snapshot = Core::new(opts).read_snapshot().unwrap();
        assert_eq!(snapshot.schema_version, 1);
        assert_eq!(snapshot.projects.len(), 1);
        assert_eq!(snapshot.agents[0].name, "Codex");
    }

    #[test]
    fn project_mcp_is_exposed_as_a_candidate_but_not_global_mcp() {
        let dir = TestDir::new("snapshot-project-mcp-scope");
        let opts = options(&dir);
        fs::create_dir_all(opts.projects_root.join("example/.agentsync/mcp")).unwrap();
        fs::write(
            opts.projects_root.join("example/.agentsync/mcp/local.toml"),
            "[server]\nname = \"local\"\nrecipe = \"chrome-devtools\"\n",
        )
        .unwrap();
        let snapshot = Core::new(opts).read_snapshot().unwrap();
        assert!(snapshot.global.mcp.iter().all(|item| item.id != "local"));
        assert_eq!(snapshot.projects[0].mcp[0].id, "local");
        assert_eq!(snapshot.projects[0].mcp[0].recipe, "chrome-devtools");
    }

    #[test]
    fn snapshot_inventories_all_registered_global_rule_targets_without_reading_bodies() {
        let dir = TestDir::new("snapshot-native-rules");
        let opts = options(&dir);
        fs::create_dir_all(opts.native_root.join(".codex")).unwrap();
        fs::create_dir_all(opts.native_root.join(".config/opencode")).unwrap();
        fs::write(
            opts.native_root.join(".codex/AGENTS.md"),
            "secret-body-do-not-send",
        )
        .unwrap();
        fs::write(
            opts.native_root.join(".config/opencode/AGENTS.md"),
            "# Native\n",
        )
        .unwrap();

        let snapshot = Core::new(opts).read_snapshot().unwrap();
        assert!(snapshot.global.native_rules.len() >= 31);
        let codex = snapshot
            .global
            .native_rules
            .iter()
            .find(|rule| rule.agent == "codex")
            .unwrap();
        assert_eq!(codex.path, "~/.codex/AGENTS.md");
        assert_eq!(codex.state, "present");
        let opencode = snapshot
            .global
            .native_rules
            .iter()
            .find(|rule| rule.agent == "opencode")
            .unwrap();
        assert_eq!(opencode.state, "present");
        let gemini = snapshot
            .global
            .native_rules
            .iter()
            .find(|rule| rule.agent == "gemini")
            .unwrap();
        assert_eq!(gemini.state, "missing");
        let cursor = snapshot
            .global
            .native_rules
            .iter()
            .find(|rule| rule.agent == "cursor-account")
            .unwrap();
        assert_eq!(cursor.state, "unavailable");
        assert!(!serde_json::to_string(&snapshot)
            .unwrap()
            .contains("secret-body-do-not-send"));
    }

    #[cfg(unix)]
    #[test]
    fn snapshot_rejects_symlinked_global_rule_without_hiding_other_agents() {
        use std::os::unix::fs::symlink;
        let dir = TestDir::new("snapshot-rule-symlink");
        let opts = options(&dir);
        fs::create_dir_all(opts.native_root.join(".codex")).unwrap();
        fs::create_dir_all(opts.native_root.join(".gemini")).unwrap();
        symlink("/tmp/secret", opts.native_root.join(".codex/AGENTS.md")).unwrap();
        fs::write(opts.native_root.join(".gemini/GEMINI.md"), "# Safe\n").unwrap();

        let snapshot = Core::new(opts).read_snapshot().unwrap();
        assert_eq!(
            snapshot
                .global
                .native_rules
                .iter()
                .find(|rule| rule.agent == "codex")
                .unwrap()
                .state,
            "unsafe"
        );
        assert_eq!(
            snapshot
                .global
                .native_rules
                .iter()
                .find(|rule| rule.agent == "gemini")
                .unwrap()
                .state,
            "present"
        );
        assert!(!snapshot
            .global
            .rules
            .iter()
            .any(|rule| rule.id == "codex-native-rule"));
    }

    #[test]
    fn snapshot_includes_auxiliary_native_rules_without_descending_into_plugins() {
        let dir = TestDir::new("snapshot-auxiliary-rules");
        let opts = options(&dir);
        fs::create_dir_all(opts.native_root.join(".gemini/config/plugins/example")).unwrap();
        fs::create_dir_all(opts.native_root.join(".codex/rules")).unwrap();
        fs::write(
            opts.native_root.join(".gemini/config/AGENTS.md"),
            "# Agent\n",
        )
        .unwrap();
        fs::write(
            opts.native_root
                .join(".gemini/config/plugins/example/AGENTS.md"),
            "# Plugin\n",
        )
        .unwrap();
        fs::write(
            opts.native_root.join(".codex/rules/default.rules"),
            "allow command\n",
        )
        .unwrap();

        let snapshot = Core::new(opts).read_snapshot().unwrap();
        assert!(snapshot
            .global
            .native_rules
            .iter()
            .any(|rule| rule.path == "~/.gemini/config/AGENTS.md" && rule.state == "present"));
        assert!(snapshot
            .global
            .native_rules
            .iter()
            .any(|rule| rule.path == "~/.codex/rules/default.rules" && rule.state == "present"));
        assert!(!snapshot
            .global
            .native_rules
            .iter()
            .any(|rule| rule.path.contains("plugins/example")));
    }

    #[test]
    fn antigravity_ide_and_cli_are_distinct_sources_of_shared_global_rules() {
        let dir = TestDir::new("snapshot-antigravity-rules");
        let opts = options(&dir);
        fs::create_dir_all(opts.native_root.join(".gemini")).unwrap();
        fs::write(opts.native_root.join(".gemini/GEMINI.md"), "# Shared\n").unwrap();

        let snapshot = Core::new(opts).read_snapshot().unwrap();
        for agent in ["antigravity-ide", "antigravity", "antigravity-cli"] {
            let rule = snapshot
                .global
                .native_rules
                .iter()
                .find(|rule| rule.agent == agent)
                .unwrap();
            assert_eq!(rule.path, "~/.gemini/GEMINI.md");
            assert_eq!(rule.state, "present");
            assert_eq!(rule.kind, "auxiliary");
            assert!(rule.reason.contains("共享"));
        }
        assert!(snapshot
            .global
            .native_rules
            .iter()
            .any(|rule| { rule.agent == "gemini" && rule.path == "~/.gemini/GEMINI.md" }));
    }

    #[test]
    fn cursor_account_rules_are_not_misreported_as_local_files() {
        let dir = TestDir::new("snapshot-cursor-account-rules");
        let snapshot = Core::new(options(&dir)).read_snapshot().unwrap();
        assert!(snapshot.global.native_rules.iter().any(|rule| {
            rule.agent == "cursor-account"
                && rule.state == "unavailable"
                && rule.reason.contains("账户")
        }));
        assert!(snapshot.global.native_rules.iter().any(|rule| {
            rule.agent == "cursor-local"
                && rule.path == "~/.cursor/rules"
                && rule.state == "missing"
        }));
    }

    #[cfg(unix)]
    #[test]
    fn snapshot_reports_symlinked_rule_directory_instead_of_skipping_it() {
        use std::os::unix::fs::symlink;
        let dir = TestDir::new("snapshot-rule-directory-link");
        let opts = options(&dir);
        fs::create_dir_all(opts.native_root.join(".codex")).unwrap();
        symlink(dir.path(), opts.native_root.join(".codex/rules")).unwrap();

        let snapshot = Core::new(opts).read_snapshot().unwrap();
        assert!(snapshot
            .global
            .native_rules
            .iter()
            .any(|rule| rule.path == "~/.codex/rules" && rule.state == "unsafe"));
    }

    #[test]
    fn snapshot_includes_manually_imported_plain_project() {
        let dir = TestDir::new("snapshot-manual-project");
        let opts = options(&dir);
        let project = dir.path().join("outside/plain-project");
        fs::create_dir_all(&project).unwrap();
        save_project_registry(
            &opts.home.join(".state/agent-assistant/projects.json"),
            &ProjectRegistry {
                schema_version: 1,
                projects: vec![RegisteredProject {
                    path: project.clone(),
                    added_at: "2026-09-17T00:00:00Z".to_string(),
                }],
            },
        )
        .unwrap();
        let snapshot = Core::new(opts).read_snapshot().unwrap();
        assert_eq!(snapshot.projects[0].path, project.to_string_lossy());
        assert_eq!(snapshot.projects[0].sync_state, "not-configured");
    }

    #[test]
    fn snapshot_redacts_urls_and_returns_only_secret_references() {
        let dir = TestDir::new("snapshot-secrets");
        let opts = options(&dir);
        fs::create_dir_all(opts.home.join("mcp")).unwrap();
        fs::write(
            opts.home.join("mcp/remote.toml"),
            "[server]\ntype = \"http\"\nurl = \"https://example.test/mcp?token=do-not-show\"\n[server.headers]\nAuthorization = \"${env:MCP_TOKEN}\"\n",
        )
        .unwrap();
        let snapshot = Core::new(opts).read_snapshot().unwrap();
        assert_eq!(snapshot.global.mcp[0].endpoint, "https://example.test");
        assert_eq!(snapshot.global.mcp[0].secret_refs, ["${env:MCP_TOKEN}"]);
        assert!(!serde_json::to_string(&snapshot)
            .unwrap()
            .contains("do-not-show"));
    }

    #[test]
    fn snapshot_redacts_local_mcp_commands() {
        let dir = TestDir::new("snapshot-local-command");
        let opts = options(&dir);
        fs::create_dir_all(opts.home.join("mcp")).unwrap();
        fs::write(
            opts.home.join("mcp/local.toml"),
            "[server]\ntype = \"stdio\"\ncommand = \"API_TOKEN=do-not-show /usr/bin/example\"\n",
        )
        .unwrap();

        let snapshot = Core::new(opts).read_snapshot().unwrap();
        let serialized = serde_json::to_string(&snapshot).unwrap();

        assert_eq!(snapshot.global.mcp[0].endpoint, "本地命令（已脱敏）");
        assert!(!serialized.contains("do-not-show"));
    }

    #[test]
    fn malformed_native_mcp_is_isolated_and_never_exposes_file_content() {
        let dir = TestDir::new("snapshot-malformed-native-mcp");
        let opts = options(&dir);
        fs::create_dir_all(opts.native_root.join(".codex")).unwrap();
        fs::create_dir_all(opts.native_root.join(".gemini")).unwrap();
        fs::write(
            opts.native_root.join(".codex/config.toml"),
            "[mcp_servers.secret]\ncommand = 'private-value'\nnot valid toml =",
        )
        .unwrap();
        fs::write(
            opts.native_root.join(".gemini/settings.json"),
            r#"{"mcpServers":{"safe":{"command":"secret-local-command"}}}"#,
        )
        .unwrap();

        let snapshot = Core::new(opts).read_snapshot().unwrap();
        assert!(snapshot.global.native_mcp.iter().any(|source| {
            source.agent == "codex" && source.state == "invalid" && source.server_count == 0
        }));
        assert!(snapshot.global.native_mcp.iter().any(|source| {
            source.agent == "gemini" && source.state == "parsed" && source.server_count == 1
        }));
        assert!(snapshot
            .global
            .native_mcp_items
            .iter()
            .any(|server| server.server_id == "safe"
                && server.agents.contains(&"gemini".to_string())));
        let serialized = serde_json::to_string(&snapshot).unwrap();
        assert!(!serialized.contains("private-value"));
        assert!(!serialized.contains("secret-local-command"));
    }

    #[test]
    fn native_mcp_inventory_reports_known_and_unverified_agents() {
        let dir = TestDir::new("snapshot-native-mcp-inventory");
        let opts = options(&dir);
        fs::create_dir_all(opts.native_root.join(".config/opencode")).unwrap();
        fs::write(
            opts.native_root.join(".config/opencode/opencode.json"),
            "{}",
        )
        .unwrap();

        let snapshot = Core::new(opts).read_snapshot().unwrap();
        assert!(snapshot.global.native_mcp.len() >= adapter::names().len());
        assert!(snapshot.global.native_mcp.iter().any(|source| {
            source.agent == "opencode"
                && source.path == "~/.config/opencode/opencode.json"
                && source.state == "unverified"
        }));
        assert!(snapshot
            .global
            .native_mcp
            .iter()
            .any(|source| { source.agent == "jetbrains" && source.state == "unavailable" }));
    }

    #[cfg(unix)]
    #[test]
    fn antigravity_apps_and_cli_share_one_mcp_config_without_duplicate_servers() {
        use std::os::unix::fs::symlink;
        let dir = TestDir::new("snapshot-antigravity-mcp");
        let opts = options(&dir);
        fs::create_dir_all(opts.native_root.join(".gemini/config")).unwrap();
        fs::create_dir_all(opts.native_root.join(".gemini/antigravity")).unwrap();
        fs::write(
            opts.native_root.join(".gemini/config/mcp_config.json"),
            r#"{"mcpServers":{"private-server":{"command":"secret-token --start"}}}"#,
        )
        .unwrap();
        symlink(
            "../config/mcp_config.json",
            opts.native_root.join(".gemini/antigravity/mcp_config.json"),
        )
        .unwrap();

        let snapshot = Core::new(opts).read_snapshot().unwrap();
        for agent in ["antigravity-ide", "antigravity", "antigravity-cli"] {
            let source = snapshot
                .global
                .native_mcp
                .iter()
                .find(|source| source.agent == agent)
                .unwrap();
            assert_eq!(
                source.path,
                if agent == "antigravity" {
                    "~/.gemini/antigravity/mcp_config.json"
                } else {
                    "~/.gemini/config/mcp_config.json"
                }
            );
            assert_eq!(source.state, "parsed");
            assert_eq!(source.server_count, 1);
            assert!(source.reason.contains("共享"));
        }
        assert_eq!(
            snapshot
                .global
                .native_mcp_items
                .iter()
                .filter(|server| server.name.contains("private-server"))
                .count(),
            1
        );
        assert!(!serde_json::to_string(&snapshot)
            .unwrap()
            .contains("secret-token"));
    }

    #[test]
    fn antigravity_app_does_not_claim_shared_mcp_without_verified_local_link() {
        let dir = TestDir::new("snapshot-antigravity-unlinked-mcp");
        let opts = options(&dir);
        fs::create_dir_all(opts.native_root.join(".gemini/config")).unwrap();
        fs::write(
            opts.native_root.join(".gemini/config/mcp_config.json"),
            r#"{"mcpServers":{"example":{"command":"local"}}}"#,
        )
        .unwrap();
        let snapshot = Core::new(opts).read_snapshot().unwrap();
        let app = snapshot
            .global
            .native_mcp
            .iter()
            .find(|source| source.agent == "antigravity")
            .unwrap();
        assert_eq!(app.state, "unavailable");
        assert_eq!(app.server_count, 0);
        assert!(snapshot
            .global
            .native_mcp
            .iter()
            .any(|source| { source.agent == "antigravity-ide" && source.state == "parsed" }));
    }

    #[test]
    fn malformed_native_mcp_entry_is_not_silently_counted_as_parsed() {
        assert!(parse_native_mcp_body("cursor", r#"{"mcpServers":{"broken":false}}"#).is_err());
        assert!(parse_native_mcp_body("codex", "mcp_servers = [1]").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn native_mcp_inventory_rejects_symlink_without_reading_target() {
        use std::os::unix::fs::symlink;
        let dir = TestDir::new("snapshot-native-mcp-link");
        let opts = options(&dir);
        fs::create_dir_all(opts.native_root.join(".cursor")).unwrap();
        symlink("/tmp/secret", opts.native_root.join(".cursor/mcp.json")).unwrap();

        let snapshot = Core::new(opts).read_snapshot().unwrap();
        assert!(snapshot
            .global
            .native_mcp
            .iter()
            .any(|source| { source.agent == "cursor" && source.state == "unsafe" }));
        assert!(!snapshot
            .global
            .mcp
            .iter()
            .any(|server| server.id.starts_with("Cursor:")));
    }

    #[cfg(unix)]
    #[test]
    fn native_skills_include_safe_links_and_nested_skills_without_reading_bodies() {
        use std::os::unix::fs::symlink;
        let dir = TestDir::new("snapshot-native-skills");
        let opts = options(&dir);
        let shared = opts.native_root.join(".agents/skills/native");
        fs::create_dir_all(shared.join("scripts")).unwrap();
        fs::create_dir_all(opts.native_root.join(".codex/skills")).unwrap();
        fs::create_dir_all(opts.native_root.join(".cursor/skills/category/review")).unwrap();
        fs::write(shared.join("SKILL.md"), "secret-in-skill-body").unwrap();
        fs::write(shared.join("scripts/command.sh"), "echo secret").unwrap();
        fs::write(
            opts.native_root
                .join(".cursor/skills/category/review/SKILL.md"),
            "# Review",
        )
        .unwrap();
        symlink(&shared, opts.native_root.join(".codex/skills/shared")).unwrap();
        symlink(
            "/tmp/external",
            opts.native_root.join(".codex/skills/unsafe"),
        )
        .unwrap();

        let snapshot = Core::new(opts).read_snapshot().unwrap();
        assert!(snapshot.global.native_skills.iter().any(|source| {
            source.agent == "codex"
                && source
                    .items
                    .iter()
                    .any(|item| item.name == "shared" && item.state == "present")
                && source
                    .items
                    .iter()
                    .any(|item| item.name == "unsafe" && item.state == "unsafe")
        }));
        assert!(snapshot.global.native_skills.iter().any(|source| {
            source.agent == "cursor"
                && source.items.iter().any(|item| {
                    item.path == "~/.cursor/skills/category/review" && item.state == "present"
                })
        }));
        assert!(snapshot
            .global
            .native_skills
            .iter()
            .any(|source| { source.agent == "claude" && source.state == "missing" }));
        assert!(!serde_json::to_string(&snapshot)
            .unwrap()
            .contains("secret-in-skill-body"));
    }

    #[test]
    fn cursor_inventories_documented_global_compatibility_skills() {
        let dir = TestDir::new("snapshot-cursor-compatible-skills");
        let opts = options(&dir);
        for (root, name) in [
            (".agents/skills", "shared"),
            (".claude/skills", "claude"),
            (".codex/skills", "codex"),
        ] {
            let skill = opts.native_root.join(root).join(name);
            fs::create_dir_all(&skill).unwrap();
            fs::write(skill.join("SKILL.md"), "private-skill-instructions").unwrap();
        }

        let snapshot = Core::new(opts).read_snapshot().unwrap();
        for (root, name) in [
            (".agents/skills", "shared"),
            (".claude/skills", "claude"),
            (".codex/skills", "codex"),
        ] {
            let source = snapshot
                .global
                .native_skills
                .iter()
                .find(|source| source.agent == "cursor" && source.path == format!("~/{root}"))
                .unwrap();
            assert_eq!(source.state, "present");
            assert!(source
                .items
                .iter()
                .any(|item| item.name == name && item.state == "present"));
        }
        assert!(!serde_json::to_string(&snapshot)
            .unwrap()
            .contains("private-skill-instructions"));
    }

    #[cfg(unix)]
    #[test]
    fn antigravity_app_ide_and_cli_skill_roots_are_separate() {
        use std::os::unix::fs::symlink;
        let dir = TestDir::new("snapshot-antigravity-skills");
        let opts = options(&dir);
        let shared = opts.native_root.join(".gemini/config/skills/review");
        fs::create_dir_all(&shared).unwrap();
        fs::write(shared.join("SKILL.md"), "private-instructions").unwrap();
        fs::create_dir_all(
            opts.native_root
                .join(".gemini/antigravity/global_skills/ide-only"),
        )
        .unwrap();
        fs::write(
            opts.native_root
                .join(".gemini/antigravity/global_skills/ide-only/SKILL.md"),
            "# IDE",
        )
        .unwrap();
        symlink(
            "../config/skills",
            opts.native_root.join(".gemini/antigravity/skills"),
        )
        .unwrap();
        let cli = opts
            .native_root
            .join(".gemini/antigravity-cli/skills/cli-only");
        fs::create_dir_all(&cli).unwrap();
        fs::write(cli.join("SKILL.md"), "# CLI").unwrap();

        let snapshot = Core::new(opts).read_snapshot().unwrap();
        for (agent, path, skill) in [
            ("antigravity", "~/.gemini/config/skills", "review"),
            ("antigravity-ide", "~/.gemini/antigravity/skills", "review"),
            (
                "antigravity-ide",
                "~/.gemini/antigravity/global_skills",
                "ide-only",
            ),
            (
                "antigravity-cli",
                "~/.gemini/antigravity-cli/skills",
                "cli-only",
            ),
        ] {
            assert!(
                snapshot.global.native_skills.iter().any(|source| {
                    source.agent == agent
                        && source.path == path
                        && source
                            .items
                            .iter()
                            .any(|item| item.name == skill && item.state == "present")
                }),
                "missing {agent} at {path}"
            );
        }
        assert!(!serde_json::to_string(&snapshot)
            .unwrap()
            .contains("private-instructions"));
    }

    #[test]
    fn native_workflows_only_attribute_ide_global_markdown_files() {
        let dir = TestDir::new("snapshot-native-workflows");
        let opts = options(&dir);
        let root = opts
            .native_root
            .join(".gemini/antigravity/global_workflows");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("review.md"), "private-workflow-body").unwrap();
        fs::write(root.join("notes.txt"), "not a workflow").unwrap();

        let snapshot = Core::new(opts).read_snapshot().unwrap();
        let ide = snapshot
            .global
            .native_workflows
            .iter()
            .find(|source| source.agent == "antigravity-ide")
            .unwrap();
        assert_eq!(ide.state, "present");
        assert_eq!(ide.path, "~/.gemini/antigravity/global_workflows");
        assert_eq!(ide.items.len(), 1);
        assert_eq!(ide.items[0].name, "review.md");
        for agent in ["antigravity", "antigravity-cli"] {
            let source = snapshot
                .global
                .native_workflows
                .iter()
                .find(|source| source.agent == agent)
                .unwrap();
            assert_eq!(source.state, "unavailable");
            assert!(source.items.is_empty());
        }
        assert!(!serde_json::to_string(&snapshot)
            .unwrap()
            .contains("private-workflow-body"));
    }

    #[test]
    fn native_hooks_distinguish_antigravity_apps_and_cli_plugin_sources() {
        let dir = TestDir::new("snapshot-native-hooks");
        let opts = options(&dir);
        let shared = opts.native_root.join(".gemini/config/hooks.json");
        fs::create_dir_all(shared.parent().unwrap()).unwrap();
        fs::write(
            &shared,
            r#"{"lint":{"PreToolUse":[{"command":"private-hook-command"}]}}"#,
        )
        .unwrap();
        let plugin = opts
            .native_root
            .join(".gemini/antigravity-cli/plugins/review");
        fs::create_dir_all(&plugin).unwrap();
        fs::write(plugin.join("hooks.json"), "private-plugin-hook").unwrap();
        fs::write(
            opts.native_root
                .join(".gemini/antigravity-cli/settings.json"),
            r#"{"hooks":{"BeforeTool":[{"command":"private-setting"}]}}"#,
        )
        .unwrap();

        let snapshot = Core::new(opts).read_snapshot().unwrap();
        for agent in ["antigravity", "antigravity-ide"] {
            assert!(snapshot.global.native_hooks.iter().any(|source| {
                source.agent == agent
                    && source.path == "~/.gemini/config/hooks.json"
                    && source.state == "present"
            }));
        }
        assert!(snapshot.global.native_hooks.iter().any(|source| {
            source.agent == "antigravity-cli"
                && source.path == "~/.gemini/antigravity-cli/plugins/review/hooks.json"
                && source.state == "present"
        }));
        assert!(snapshot.global.native_hooks.iter().any(|source| {
            source.agent == "antigravity-cli"
                && source.path == "~/.gemini/antigravity-cli/settings.json"
                && source.state == "unverified"
        }));
        let serialized = serde_json::to_string(&snapshot).unwrap();
        assert!(!serialized.contains("private-hook-command"));
        assert!(!serialized.contains("private-plugin-hook"));
        assert!(!serialized.contains("private-setting"));
    }

    #[test]
    fn native_hook_settings_distinguish_absent_hooks_from_invalid_configuration() {
        let dir = TestDir::new("snapshot-native-hook-settings");
        let opts = options(&dir);
        fs::create_dir_all(opts.native_root.join(".claude")).unwrap();
        fs::create_dir_all(opts.native_root.join(".gemini")).unwrap();
        fs::write(
            opts.native_root.join(".claude/settings.json"),
            r#"{"env":{"TOKEN":"private-value"},"hooks":{}}"#,
        )
        .unwrap();
        fs::write(opts.native_root.join(".gemini/settings.json"), "{hooks:").unwrap();

        let snapshot = Core::new(opts).read_snapshot().unwrap();
        assert!(snapshot
            .global
            .native_hooks
            .iter()
            .any(|source| { source.agent == "claude" && source.state == "no-hooks" }));
        assert!(snapshot
            .global
            .native_hooks
            .iter()
            .any(|source| { source.agent == "gemini" && source.state == "invalid" }));
        assert!(!serde_json::to_string(&snapshot)
            .unwrap()
            .contains("private-value"));
    }

    #[cfg(unix)]
    #[test]
    fn native_hooks_reject_symlinked_files() {
        use std::os::unix::fs::symlink;
        let dir = TestDir::new("snapshot-native-hooks-symlink");
        let opts = options(&dir);
        let parent = opts.native_root.join(".gemini/config");
        fs::create_dir_all(&parent).unwrap();
        symlink("/tmp/private-hooks", parent.join("hooks.json")).unwrap();
        let snapshot = Core::new(opts).read_snapshot().unwrap();
        assert!(snapshot
            .global
            .native_hooks
            .iter()
            .any(|source| { source.agent == "antigravity" && source.state == "unsafe" }));
    }

    #[test]
    fn native_subagents_list_shared_and_plugin_definitions_without_reading_prompts() {
        let dir = TestDir::new("snapshot-native-subagents");
        let opts = options(&dir);
        let shared = opts.native_root.join(".gemini/config/agents");
        fs::create_dir_all(shared.join("nested")).unwrap();
        fs::write(shared.join("review.md"), "private-agent-prompt").unwrap();
        fs::write(shared.join("nested/agent.md"), "private-nested-prompt").unwrap();
        let desktop_plugin = opts.native_root.join(".gemini/config/plugins/extra/agents");
        fs::create_dir_all(&desktop_plugin).unwrap();
        fs::write(desktop_plugin.join("plugin.md"), "private-plugin-prompt").unwrap();
        let cli_plugin = opts
            .native_root
            .join(".gemini/antigravity-cli/plugins/cli-only/agents");
        fs::create_dir_all(&cli_plugin).unwrap();
        fs::write(cli_plugin.join("cli.md"), "private-cli-prompt").unwrap();
        let cursor = opts.native_root.join(".cursor/agents");
        fs::create_dir_all(&cursor).unwrap();
        fs::write(cursor.join("cursor.md"), "private-cursor-prompt").unwrap();

        let snapshot = Core::new(opts).read_snapshot().unwrap();
        for agent in ["antigravity", "antigravity-cli"] {
            assert!(snapshot.global.native_subagents.iter().any(|source| {
                source.agent == agent
                    && source.path == "~/.gemini/config/agents"
                    && source.items.len() == 2
            }));
        }
        assert!(snapshot
            .global
            .native_subagents
            .iter()
            .any(|source| { source.agent == "antigravity-ide" && source.state == "unavailable" }));
        for (agent, name) in [
            ("antigravity", "plugin.md"),
            ("antigravity-cli", "cli.md"),
            ("cursor", "cursor.md"),
        ] {
            assert!(snapshot.global.native_subagents.iter().any(|source| {
                source.agent == agent && source.items.iter().any(|item| item.name == name)
            }));
        }
        let serialized = serde_json::to_string(&snapshot).unwrap();
        assert!(!serialized.contains("private-agent-prompt"));
        assert!(!serialized.contains("private-cli-prompt"));
        assert!(!serialized.contains("private-cursor-prompt"));
    }

    #[test]
    fn cursor_inventories_compatible_global_subagent_directories() {
        let dir = TestDir::new("snapshot-cursor-compatible-subagents");
        let opts = options(&dir);
        for (root, name) in [
            (".claude/agents", "claude.md"),
            (".codex/agents", "codex.md"),
        ] {
            let path = opts.native_root.join(root);
            fs::create_dir_all(&path).unwrap();
            fs::write(path.join(name), "private-agent-prompt").unwrap();
        }

        let snapshot = Core::new(opts).read_snapshot().unwrap();
        for (root, name) in [
            (".claude/agents", "claude.md"),
            (".codex/agents", "codex.md"),
        ] {
            let source = snapshot
                .global
                .native_subagents
                .iter()
                .find(|source| source.agent == "cursor" && source.path == format!("~/{root}"))
                .unwrap();
            assert_eq!(source.state, "present");
            assert!(source
                .items
                .iter()
                .any(|item| item.name == name && item.state == "present"));
        }
        assert!(!serde_json::to_string(&snapshot)
            .unwrap()
            .contains("private-agent-prompt"));
    }

    #[cfg(unix)]
    #[test]
    fn native_subagents_flag_unsafe_links_without_following_them() {
        use std::os::unix::fs::symlink;
        let dir = TestDir::new("snapshot-native-subagents-link");
        let opts = options(&dir);
        let root = opts.native_root.join(".gemini/config/agents");
        fs::create_dir_all(&root).unwrap();
        symlink("/tmp/private-agent.md", root.join("external.md")).unwrap();
        let plugins = opts.native_root.join(".gemini/config/plugins");
        fs::create_dir_all(&plugins).unwrap();
        symlink("/tmp/private-plugin", plugins.join("external")).unwrap();
        let snapshot = Core::new(opts).read_snapshot().unwrap();
        assert!(snapshot.global.native_subagents.iter().any(|source| {
            source.agent == "antigravity"
                && source
                    .items
                    .iter()
                    .any(|item| item.name == "external.md" && item.state == "unsafe")
        }));
        assert!(snapshot.global.native_subagents.iter().any(|source| {
            source.agent == "antigravity"
                && source.path == "~/.gemini/config/plugins/external"
                && source.state == "unsafe"
        }));
    }

    #[test]
    fn unverified_agent_capabilities_are_explicitly_unknown() {
        let summary = agent_summary("claude");

        assert_eq!(summary.health, "warn");
        assert_eq!(summary.sync_state, "attention");
        assert_eq!(summary.capabilities.memory, "unknown");
        assert_eq!(summary.capabilities.components["rules"], "unknown");
        assert_eq!(summary.capabilities.components["workflows"], "unknown");
    }

    #[test]
    fn generated_timestamp_is_rfc3339_utc() {
        let value = now_rfc3339();
        assert_eq!(value.len(), 20);
        assert!(value.ends_with('Z'));
    }
}
