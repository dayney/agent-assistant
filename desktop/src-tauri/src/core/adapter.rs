use crate::core::source::Memory;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy)]
struct AdapterSpec {
    name: &'static str,
    user_memory: Option<&'static str>,
    project_memory: &'static str,
    user_reason: Option<&'static str>,
}

const SPECS: &[AdapterSpec] = &[
    AdapterSpec { name: "amazonq", user_memory: None, project_memory: ".amazonq/rules/agentsync.md", user_reason: None },
    AdapterSpec { name: "amp", user_memory: Some(".config/amp/AGENTS.md"), project_memory: "AGENTS.md", user_reason: None },
    AdapterSpec { name: "antigravity", user_memory: None, project_memory: "AGENTS.md", user_reason: None },
    AdapterSpec { name: "augmentcode", user_memory: Some(".augment/rules/agentsync.md"), project_memory: ".augment/rules/agentsync.md", user_reason: None },
    AdapterSpec { name: "claude", user_memory: Some(".claude/CLAUDE.md"), project_memory: "CLAUDE.md", user_reason: None },
    AdapterSpec { name: "cline", user_memory: None, project_memory: ".clinerules/agentsync.md", user_reason: Some("Cline global rules live in ~/Documents/Cline/ (a non-XDG app path agentsync does not target); memory projects at project scope only (.clinerules/)") },
    AdapterSpec { name: "codex", user_memory: Some(".codex/AGENTS.md"), project_memory: "AGENTS.md", user_reason: None },
    AdapterSpec { name: "continue", user_memory: Some(".continue/rules/agentsync.md"), project_memory: ".continue/rules/agentsync.md", user_reason: None },
    AdapterSpec { name: "copilot", user_memory: None, project_memory: ".github/copilot-instructions.md", user_reason: None },
    AdapterSpec { name: "copilot-cli", user_memory: None, project_memory: "AGENTS.md", user_reason: None },
    AdapterSpec { name: "crush", user_memory: None, project_memory: "AGENTS.md", user_reason: None },
    AdapterSpec { name: "cursor", user_memory: None, project_memory: "AGENTS.md", user_reason: Some("Cursor stores user-level rules in app-local storage; no filesystem projection target (use project scope for AGENTS.md)") },
    AdapterSpec { name: "factory", user_memory: None, project_memory: "AGENTS.md", user_reason: None },
    AdapterSpec { name: "firebase", user_memory: None, project_memory: ".idx/airules.md", user_reason: None },
    AdapterSpec { name: "gemini", user_memory: Some(".gemini/GEMINI.md"), project_memory: "GEMINI.md", user_reason: None },
    AdapterSpec { name: "goose", user_memory: None, project_memory: ".goosehints", user_reason: None },
    AdapterSpec { name: "jetbrains", user_memory: None, project_memory: ".aiassistant/rules/agentsync.md", user_reason: None },
    AdapterSpec { name: "jules", user_memory: None, project_memory: "AGENTS.md", user_reason: None },
    AdapterSpec { name: "junie", user_memory: None, project_memory: "AGENTS.md", user_reason: None },
    AdapterSpec { name: "kilocode", user_memory: None, project_memory: ".kilocode/rules/agentsync.md", user_reason: None },
    AdapterSpec { name: "kiro", user_memory: Some(".kiro/steering/agentsync.md"), project_memory: ".kiro/steering/agentsync.md", user_reason: None },
    AdapterSpec { name: "mistral", user_memory: None, project_memory: "AGENTS.md", user_reason: None },
    AdapterSpec { name: "opencode", user_memory: Some(".config/opencode/AGENTS.md"), project_memory: "AGENTS.md", user_reason: None },
    AdapterSpec { name: "openhands", user_memory: None, project_memory: "AGENTS.md", user_reason: None },
    AdapterSpec { name: "pi", user_memory: Some(".pi/agent/AGENTS.md"), project_memory: "AGENTS.md", user_reason: None },
    AdapterSpec { name: "qwen", user_memory: Some(".qwen/QWEN.md"), project_memory: "QWEN.md", user_reason: None },
    AdapterSpec { name: "roo", user_memory: Some(".roo/rules/agentsync.md"), project_memory: ".roo/rules/agentsync.md", user_reason: None },
    AdapterSpec { name: "trae", user_memory: None, project_memory: ".trae/rules/project_rules.md", user_reason: None },
    AdapterSpec { name: "warp", user_memory: None, project_memory: "WARP.md", user_reason: None },
    AdapterSpec { name: "windsurf", user_memory: Some(".codeium/windsurf/memories/global_rules.md"), project_memory: ".windsurf/rules/agentsync.md", user_reason: None },
    AdapterSpec { name: "zed", user_memory: Some(".config/zed/AGENTS.md"), project_memory: "AGENTS.md", user_reason: None },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Scope {
    User,
    Project,
}

impl Scope {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Project => "project",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct MemoryTarget {
    pub(crate) path: Option<PathBuf>,
    pub(crate) reason: Option<String>,
}

pub(crate) fn names() -> Vec<&'static str> {
    SPECS.iter().map(|spec| spec.name).collect()
}

pub(crate) fn memory_target(
    agent: &str,
    scope: Scope,
    native_root: &Path,
    project: Option<&Path>,
) -> Result<MemoryTarget, String> {
    let spec = find_spec(agent)?;
    match scope {
        Scope::User => {
            Ok(match spec.user_memory {
                Some(relative) => MemoryTarget {
                    path: Some(native_root.join(relative)),
                    reason: None,
                },
                None => MemoryTarget {
                    path: None,
                    reason: Some(spec.user_reason.map(str::to_string).unwrap_or_else(|| {
                        format!("{} memory has no user-scope target", spec.name)
                    })),
                },
            })
        }
        Scope::Project => {
            let project = project.ok_or_else(|| "project path is required".to_string())?;
            let path = project.join(spec.project_memory);
            crate::core::paths::reject_symlinks_below(project, &path)?;
            Ok(MemoryTarget {
                path: Some(path),
                reason: None,
            })
        }
    }
}

pub(crate) fn render_memory(
    agent: &str,
    scope: Scope,
    path: &Path,
    memory: &Memory,
    banner: bool,
) -> Result<Vec<u8>, String> {
    find_spec(agent)?;
    let destination_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| format!("Rule destination has no file name: {}", path.display()))?;
    let body = crate::core::source::render_managed_memory(
        &memory.body,
        &memory.fragments,
        destination_name,
        banner,
    );
    if agent == "windsurf" && scope == Scope::Project {
        return Ok(format!("---\ntrigger: always_on\n---\n\n{body}").into_bytes());
    }
    Ok(body.into_bytes())
}

fn find_spec(agent: &str) -> Result<&'static AdapterSpec, String> {
    SPECS
        .iter()
        .find(|spec| spec.name == agent)
        .ok_or_else(|| format!("unknown Agent {agent:?}"))
}

#[cfg(test)]
mod tests {
    use super::{memory_target, names, render_memory, Scope};
    use crate::core::source::Memory;
    use crate::core::test_support::TestDir;
    use std::collections::BTreeMap;

    #[test]
    fn registry_matches_go_baseline() {
        assert_eq!(
            names(),
            vec![
                "amazonq",
                "amp",
                "antigravity",
                "augmentcode",
                "claude",
                "cline",
                "codex",
                "continue",
                "copilot",
                "copilot-cli",
                "crush",
                "cursor",
                "factory",
                "firebase",
                "gemini",
                "goose",
                "jetbrains",
                "jules",
                "junie",
                "kilocode",
                "kiro",
                "mistral",
                "opencode",
                "openhands",
                "pi",
                "qwen",
                "roo",
                "trae",
                "warp",
                "windsurf",
                "zed",
            ]
        );
    }

    #[test]
    fn global_and_project_targets_match_go_baseline() {
        let dir = TestDir::new("adapter-targets");
        let home = dir.path().join("home");
        let project = dir.path().join("project");
        let cases = [
            ("codex", Some(".codex/AGENTS.md"), "AGENTS.md"),
            ("claude", Some(".claude/CLAUDE.md"), "CLAUDE.md"),
            ("gemini", Some(".gemini/GEMINI.md"), "GEMINI.md"),
            ("cursor", None, "AGENTS.md"),
            ("cline", None, ".clinerules/agentsync.md"),
            (
                "windsurf",
                Some(".codeium/windsurf/memories/global_rules.md"),
                ".windsurf/rules/agentsync.md",
            ),
            ("amazonq", None, ".amazonq/rules/agentsync.md"),
            ("qwen", Some(".qwen/QWEN.md"), "QWEN.md"),
            ("zed", Some(".config/zed/AGENTS.md"), "AGENTS.md"),
        ];
        for (agent, global, project_relative) in cases {
            let user_target = memory_target(agent, Scope::User, &home, None).unwrap();
            assert_eq!(
                user_target.path,
                global.map(|relative| home.join(relative)),
                "global target for {agent}"
            );
            let project_target =
                memory_target(agent, Scope::Project, &home, Some(&project)).unwrap();
            assert_eq!(
                project_target.path,
                Some(project.join(project_relative)),
                "project target for {agent}"
            );
        }
    }

    #[test]
    fn every_registered_agent_has_a_project_rule_target() {
        let dir = TestDir::new("adapter-project-targets");
        for agent in names() {
            let target = memory_target(agent, Scope::Project, dir.path(), Some(dir.path()))
                .unwrap_or_else(|error| panic!("{agent}: {error}"));
            assert!(target.path.is_some(), "project target missing for {agent}");
        }
    }

    #[test]
    fn windsurf_frontmatter_stays_before_managed_banner() {
        let memory = Memory {
            body: "# Rules\n".to_string(),
            fragments: BTreeMap::new(),
        };
        let output = render_memory(
            "windsurf",
            Scope::Project,
            std::path::Path::new("/repo/.windsurf/rules/agentsync.md"),
            &memory,
            true,
        )
        .unwrap();
        let output = String::from_utf8(output).unwrap();
        assert!(output.starts_with("---\ntrigger: always_on\n---\n\n<!-- agentsync:managed"));
    }

    #[test]
    fn representative_rule_outputs_match_go_goldens_byte_for_byte() {
        let memory = Memory {
            body: "# Shared\n\n@import ./fragments/style.md\n".to_string(),
            fragments: BTreeMap::from([(
                "style.md".to_string(),
                "Use existing styles.\n".to_string(),
            )]),
        };
        let cases = [
            (
                "codex",
                Scope::User,
                "/home/example/.codex/AGENTS.md",
                include_bytes!("fixtures/codex-agents.golden.md").as_slice(),
            ),
            (
                "claude",
                Scope::User,
                "/home/example/.claude/CLAUDE.md",
                include_bytes!("fixtures/claude-agents.golden.md").as_slice(),
            ),
            (
                "gemini",
                Scope::Project,
                "/repo/GEMINI.md",
                include_bytes!("fixtures/gemini-agents.golden.md").as_slice(),
            ),
            (
                "windsurf",
                Scope::Project,
                "/repo/.windsurf/rules/agentsync.md",
                include_bytes!("fixtures/windsurf-agents.golden.md").as_slice(),
            ),
            (
                "qwen",
                Scope::User,
                "/home/example/.qwen/QWEN.md",
                include_bytes!("fixtures/qwen-agents.golden.md").as_slice(),
            ),
        ];
        for (agent, scope, path, expected) in cases {
            let actual = render_memory(agent, scope, std::path::Path::new(path), &memory, true)
                .unwrap_or_else(|error| panic!("{agent}: {error}"));
            assert_eq!(actual, expected, "golden mismatch for {agent}");
        }
    }

    #[test]
    fn unknown_adapter_is_an_error() {
        let error =
            memory_target("unknown", Scope::User, std::path::Path::new("/tmp"), None).unwrap_err();
        assert!(error.contains("unknown Agent"));
    }
}
