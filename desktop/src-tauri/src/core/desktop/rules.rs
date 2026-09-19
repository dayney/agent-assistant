use super::model::{
    ImportNativeRuleRequest, ProjectRuleImport, ProjectRuleSource, RuleBackup, RuleDocument,
    RuleProposal, RuleRequest, RuleScope, RuleSyncResult, RuleTarget, RuleWorkspace,
    SaveRuleRequest, SyncRulesRequest,
};
use super::snapshot::now_rfc3339;
use super::Core;
use crate::core::{adapter, drift, hash, iox, paths, source, state};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

impl Core {
    pub(crate) fn preview_native_rule(&self, agent: &str, path: &str) -> Result<String, String> {
        let discovered = super::snapshot::discover_native_rules(&self.opts.native_root);
        if !discovered
            .iter()
            .any(|source| source.agent == agent && source.path == path && source.state == "present")
        {
            return Err("native Rule source is not a readable discovered file".to_string());
        }
        let relative = path.strip_prefix("~/").ok_or("invalid native Rule path")?;
        let file = self.opts.native_root.join(relative);
        paths::reject_symlinks_below(&self.opts.native_root, &file)?;
        let metadata =
            fs::symlink_metadata(&file).map_err(|error| format!("inspect native Rule: {error}"))?;
        if !metadata.file_type().is_file() || metadata.len() > 1024 * 1024 {
            return Err(
                "native Rule is not a regular file under the 1 MiB preview limit".to_string(),
            );
        }
        fs::read_to_string(file).map_err(|error| format!("read native Rule: {error}"))
    }

    pub(crate) fn get_rules(&self, request: RuleRequest) -> Result<RuleWorkspace, String> {
        self.preview_rules(&request)
    }

    pub(crate) fn save_rule(&self, request: SaveRuleRequest) -> Result<RuleDocument, String> {
        if request.body.trim().is_empty() {
            return Err("rule mother template cannot be empty".to_string());
        }
        self.with_lock(|| {
            let (home, project_path) = self.canonical_home(&request.rule)?;
            let mut canonical =
                source::load(&home).map_err(|error| format!("load canonical rule: {error}"))?;
            let mut config_changed = false;
            for agent in selected_agents_from_request(&request.rule.agents) {
                ensure_known_agent(&agent)?;
                config_changed |= canonical.config.insert_enabled_agent(&agent);
            }
            if !canonical.config.existed() || config_changed {
                source::write_config(&home, &canonical.config)
                    .map_err(|error| format!("write canonical config: {error}"))?;
            }
            canonical.memory.body = request.body;
            source::write_memory(&home, &canonical.memory)
                .map_err(|error| format!("write rule mother template: {error}"))?;
            Ok(rule_document(
                &request.rule,
                &home,
                project_path.as_deref(),
                &canonical.memory,
                selected_agents(&canonical.config, &request.rule.agents),
            ))
        })
    }

    pub(crate) fn sync_rules(&self, request: SyncRulesRequest) -> Result<RuleSyncResult, String> {
        self.with_lock(|| {
            let preview = self.preview_rules(&request.rule)?;
            let mut result = RuleSyncResult {
                preview,
                applied: false,
                backups: Vec::new(),
            };
            if result.preview.targets.is_empty() {
                return Err("no target Agents selected for Rule synchronization".to_string());
            }
            if result.preview.blocked && request.resolution != "backup-overwrite" {
                return Ok(result);
            }
            for target in &result.preview.targets {
                if target.blocked && !target.supported {
                    return Err(format!(
                        "agent {} cannot receive this Rule at {} scope; deselect it before synchronizing",
                        target.agent,
                        request.rule.scope.as_str()
                    ));
                }
            }
            let (home, project_path) = self.canonical_home(&request.rule)?;
            let canonical = source::load(&home)?;
            let scope = adapter_scope(request.rule.scope);
            let mut targets = state::load_targets(&self.targets_state_path())?;
            let apply_result = (|| {
                for target in &result.preview.targets {
                    if !target.supported || canonical.memory.body.trim().is_empty() {
                        continue;
                    }
                    let path = PathBuf::from(&target.path);
                    let content = adapter::render_memory(
                        &target.agent,
                        scope,
                        &path,
                        &canonical.memory,
                        canonical.config.memory_banner,
                    )?;
                    let key = self.state_key(
                        &target.agent,
                        scope,
                        project_path.as_deref(),
                        &path,
                    );
                    let owned_hash = targets.files.get(&key).map(|entry| entry.sha256.as_str());
                    if let Some(backup) = self.write_rule_target(
                        target,
                        &content,
                        owned_hash,
                        project_path.as_deref(),
                    )? {
                        result.backups.push(backup);
                    }
                    targets.files.insert(
                        key,
                        state::FileEntry {
                            sha256: hash::sha256_hex(&content),
                            mode: 0o644,
                            applied_at: now_rfc3339(),
                            source_id: "memory/AGENTS.md".to_string(),
                        },
                    );
                }
                Ok::<(), String>(())
            })();
            let state_result = state::save_targets(&self.targets_state_path(), &targets);
            apply_result?;
            state_result?;
            result.applied = true;
            result.preview = self.preview_rules(&request.rule)?;
            Ok(result)
        })
    }

    pub(crate) fn import_native_rule(
        &self,
        request: ImportNativeRuleRequest,
    ) -> Result<RuleDocument, String> {
        if request.agent.trim().is_empty() {
            return Err("agent is required".to_string());
        }
        self.with_lock(|| {
            let workspace = self.preview_rules(&request.rule)?;
            let target = workspace
                .targets
                .iter()
                .find(|target| target.agent == request.agent)
                .filter(|target| !target.path.is_empty())
                .ok_or_else(|| {
                    format!(
                        "agent {} has no Rule destination at {} scope",
                        request.agent,
                        request.rule.scope.as_str()
                    )
                })?;
            let data = fs::read_to_string(&target.path)
                .map_err(|error| format!("read native Rule {}: {error}", target.path))?;
            let body = source::strip_managed_banner(&data);
            let memory =
                source::collapse_memory_markers(&body)?.unwrap_or_else(|| source::Memory {
                    body,
                    fragments: BTreeMap::new(),
                });
            let (home, project_path) = self.canonical_home(&request.rule)?;
            source::write_memory(&home, &memory)?;
            let canonical = source::load(&home)?;
            Ok(rule_document(
                &request.rule,
                &home,
                project_path.as_deref(),
                &canonical.memory,
                selected_agents(&canonical.config, &request.rule.agents),
            ))
        })
    }

    pub(crate) fn import_project(&self, path: &str) -> Result<ProjectRuleImport, String> {
        let project = validate_project_root(path)?;
        self.with_lock(|| {
            let registry_path = self.project_registry_path();
            let mut registry = state::load_project_registry(&registry_path)?;
            if !registry.projects.iter().any(|item| item.path == project) {
                registry.projects.push(state::RegisteredProject {
                    path: project.clone(),
                    added_at: now_rfc3339(),
                });
                registry
                    .projects
                    .sort_by(|left, right| left.path.cmp(&right.path));
                state::save_project_registry(&registry_path, &registry)?;
            }
            let sources = self.discover_project_rule_sources(&project)?;
            Ok(ProjectRuleImport {
                path: project.to_string_lossy().to_string(),
                needs_analysis: unique_rule_bodies(&sources) > 1,
                sources,
            })
        })
    }

    pub(crate) fn analyze_project_rules(&self, path: &str) -> Result<RuleProposal, String> {
        let project = validate_project_root(path)?;
        let sources = self.discover_project_rule_sources(&project)?;
        if sources.is_empty() {
            return Err("no native project Rule files were found".to_string());
        }
        if unique_rule_bodies(&sources) == 1 {
            return Ok(RuleProposal {
                body: sources[0].body.clone(),
                notes: vec![
                    "All discovered native Rule files are semantically identical by content."
                        .to_string(),
                ],
                analyzer: "deterministic".to_string(),
                requires_ai: false,
            });
        }
        let Some(analyzer) = &self.analyzer else {
            return Err("native Rule files differ and no AI analyzer is available".to_string());
        };
        let proposal = analyzer.analyze(&project, &sources)?;
        if proposal.body.trim().is_empty() {
            return Err("AI analyzer returned an empty Rule proposal".to_string());
        }
        Ok(proposal)
    }

    fn preview_rules(&self, request: &RuleRequest) -> Result<RuleWorkspace, String> {
        let (home, project_path) = self.canonical_home(request)?;
        let canonical =
            source::load(&home).map_err(|error| format!("load canonical Rule: {error}"))?;
        let agents = selected_agents(&canonical.config, &request.agents);
        for agent in &agents {
            ensure_known_agent(agent)?;
        }
        let scope = adapter_scope(request.scope);
        let targets_state = state::load_targets(&self.targets_state_path())?;
        let document = rule_document(
            request,
            &home,
            project_path.as_deref(),
            &canonical.memory,
            agents.clone(),
        );
        let mut workspace = RuleWorkspace {
            document,
            targets: Vec::with_capacity(agents.len()),
            available_agents: adapter::names().into_iter().map(str::to_string).collect(),
            blocked: false,
        };
        for agent in agents {
            let destination = adapter::memory_target(
                &agent,
                scope,
                &self.opts.native_root,
                project_path.as_deref(),
            )?;
            let Some(path) = destination.path else {
                workspace.blocked = true;
                workspace.targets.push(RuleTarget {
                    agent,
                    path: String::new(),
                    supported: false,
                    status: "unsupported".to_string(),
                    blocked: true,
                    will_write: false,
                    diff: String::new(),
                    reason: destination.reason.unwrap_or_default(),
                });
                continue;
            };
            if canonical.memory.body.trim().is_empty() {
                let actual = read_optional_regular_file(&path)?;
                let native_only = actual
                    .as_deref()
                    .is_some_and(|content| !content.trim().is_empty());
                workspace.blocked |= native_only;
                workspace.targets.push(RuleTarget {
                    agent,
                    path: path.to_string_lossy().to_string(),
                    supported: true,
                    status: if native_only { "native-only" } else { "empty" }.to_string(),
                    blocked: native_only,
                    will_write: false,
                    diff: actual
                        .filter(|content| !content.trim().is_empty())
                        .map(|content| drift::line_diff(&content, ""))
                        .unwrap_or_default(),
                    reason: String::new(),
                });
                continue;
            }
            let desired = adapter::render_memory(
                &agent,
                scope,
                &path,
                &canonical.memory,
                canonical.config.memory_banner,
            )?;
            let actual_file = read_optional_regular_file(&path)?;
            let actual = actual_file.clone().unwrap_or_default();
            let source_hash = hash::sha256_hex(&desired);
            let destination_hash = actual_file
                .as_deref()
                .map(|content| hash::sha256_hex(content.as_bytes()))
                .unwrap_or_default();
            let state_key = self.state_key(&agent, scope, project_path.as_deref(), &path);
            let applied_hash = targets_state
                .files
                .get(&state_key)
                .map(|entry| entry.sha256.as_str())
                .unwrap_or_default();
            let class = drift::classify(&source_hash, applied_hash, &destination_hash);
            let blocked = !drift::safe_for_auto_apply(class);
            workspace.blocked |= blocked;
            workspace.targets.push(RuleTarget {
                agent,
                path: path.to_string_lossy().to_string(),
                supported: true,
                status: class.as_str().to_string(),
                blocked,
                will_write: source_hash != destination_hash,
                diff: drift::line_diff(&actual, &String::from_utf8_lossy(&desired)),
                reason: String::new(),
            });
        }
        Ok(workspace)
    }

    fn canonical_home(&self, request: &RuleRequest) -> Result<(PathBuf, Option<PathBuf>), String> {
        match request.scope {
            RuleScope::Global => Ok((self.opts.home.clone(), None)),
            RuleScope::Project => {
                let project = validate_project_root(&request.project_path)?;
                let home = paths::project_home(&project);
                paths::reject_symlinks_below(&project, &home.join("memory/AGENTS.md"))?;
                paths::reject_symlinks_below(&project, &home.join("memory/fragments"))?;
                Ok((home, Some(project)))
            }
        }
    }

    fn discover_project_rule_sources(
        &self,
        project: &Path,
    ) -> Result<Vec<ProjectRuleSource>, String> {
        let mut by_path: BTreeMap<PathBuf, Vec<String>> = BTreeMap::new();
        for agent in adapter::names() {
            let target = adapter::memory_target(
                agent,
                adapter::Scope::Project,
                &self.opts.native_root,
                Some(project),
            )?;
            if let Some(path) = target.path {
                by_path.entry(path).or_default().push(agent.to_string());
            }
        }
        let mut sources = Vec::new();
        for (path, mut agents) in by_path {
            let Some(body) = read_optional_regular_file(&path)? else {
                continue;
            };
            agents.sort();
            sources.push(ProjectRuleSource {
                path: path.to_string_lossy().to_string(),
                agents,
                body: flatten_rendered_rule(&source::strip_managed_banner(&body)),
            });
        }
        Ok(sources)
    }

    fn targets_state_path(&self) -> PathBuf {
        self.opts.home.join(".state/targets.json")
    }

    fn state_key(
        &self,
        agent: &str,
        scope: adapter::Scope,
        project: Option<&Path>,
        destination: &Path,
    ) -> String {
        format!(
            "{agent}:{}:{}:{}",
            scope.as_str(),
            state::home_relative(
                &self.opts.native_root,
                project.unwrap_or_else(|| Path::new(""))
            ),
            state::home_relative(&self.opts.native_root, destination)
        )
    }

    pub(super) fn with_lock<T>(
        &self,
        operation: impl FnOnce() -> Result<T, String>,
    ) -> Result<T, String> {
        iox::with_exclusive_lock(&self.opts.home.join(".state/agentsync.lock"), operation)
    }

    fn backup_rule(&self, path: &Path) -> Result<PathBuf, String> {
        let metadata = fs::symlink_metadata(path)
            .map_err(|error| format!("inspect {} for backup: {error}", path.display()))?;
        if !metadata.file_type().is_file() {
            return Err(format!(
                "refusing to back up {}: not a regular file",
                path.display()
            ));
        }
        let data = fs::read(path)
            .map_err(|error| format!("read {} for backup: {error}", path.display()))?;
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let mut destination = self
            .opts
            .home
            .join(".state/backups")
            .join(stamp.to_string());
        for component in path.components() {
            if let Component::Normal(value) = component {
                destination.push(value);
            }
        }
        iox::atomic_write_private(&destination, &data)
            .map_err(|error| format!("backup {}: {error}", path.display()))?;
        Ok(destination)
    }

    fn write_rule_target(
        &self,
        target: &RuleTarget,
        content: &[u8],
        owned_hash: Option<&str>,
        project_root: Option<&Path>,
    ) -> Result<Option<RuleBackup>, String> {
        let path = Path::new(&target.path);
        if let Some(project_root) = project_root {
            paths::reject_symlinks_below(project_root, path)?;
        }
        let current = read_optional_regular_file(path)?;
        if current
            .as_ref()
            .is_some_and(|body| body.as_bytes() == content)
        {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;

                let permissions = fs::metadata(path)
                    .map_err(|error| format!("inspect native Rule {}: {error}", path.display()))?
                    .permissions();
                if permissions.mode() & 0o777 != 0o644 {
                    fs::set_permissions(path, fs::Permissions::from_mode(0o644)).map_err(
                        |error| format!("set native Rule permissions {}: {error}", path.display()),
                    )?;
                }
            }
            return Ok(None);
        }
        let backup = if current.as_ref().is_some_and(|body| {
            !body.is_empty() && owned_hash != Some(hash::sha256_hex(body.as_bytes()).as_str())
        }) {
            let destination = self.backup_rule(path)?;
            Some(RuleBackup {
                agent: target.agent.clone(),
                source_path: target.path.clone(),
                backup_path: destination.to_string_lossy().to_string(),
            })
        } else {
            None
        };
        iox::atomic_write(path, content)
            .map_err(|error| format!("write native Rule {}: {error}", path.display()))?;
        Ok(backup)
    }
}

fn adapter_scope(scope: RuleScope) -> adapter::Scope {
    match scope {
        RuleScope::Global => adapter::Scope::User,
        RuleScope::Project => adapter::Scope::Project,
    }
}

fn selected_agents(config: &source::Config, requested: &[String]) -> Vec<String> {
    let requested = selected_agents_from_request(requested);
    if !requested.is_empty() {
        return requested;
    }
    config
        .agents
        .iter()
        .filter(|(_, agent)| agent.enabled)
        .map(|(name, _)| name.clone())
        .collect()
}

fn selected_agents_from_request(requested: &[String]) -> Vec<String> {
    requested
        .iter()
        .map(|name| name.trim())
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn ensure_known_agent(agent: &str) -> Result<(), String> {
    if adapter::names().contains(&agent) {
        Ok(())
    } else {
        Err(format!("unknown Agent {agent:?}"))
    }
}

fn rule_document(
    request: &RuleRequest,
    home: &Path,
    project_path: Option<&Path>,
    memory: &source::Memory,
    agents: Vec<String>,
) -> RuleDocument {
    RuleDocument {
        scope: request.scope.as_str().to_string(),
        project_path: project_path
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_default(),
        canonical_path: home.join("memory/AGENTS.md").to_string_lossy().to_string(),
        body: memory.body.clone(),
        fragments: memory.fragments.keys().cloned().collect(),
        agents,
    }
}

fn read_optional_regular_file(path: &Path) -> Result<Option<String>, String> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("inspect native Rule {}: {error}", path.display())),
    };
    if !metadata.file_type().is_file() {
        return Err(format!(
            "refusing to read native Rule {}: not a regular file",
            path.display()
        ));
    }
    fs::read_to_string(path)
        .map(Some)
        .map_err(|error| format!("read native Rule {}: {error}", path.display()))
}

pub(super) fn validate_project_root(path: &str) -> Result<PathBuf, String> {
    if path.trim().is_empty() {
        return Err("project path is required".to_string());
    }
    let path = PathBuf::from(path);
    let absolute = if path.is_absolute() {
        path
    } else {
        std::env::current_dir()
            .map_err(|error| format!("resolve project path: {error}"))?
            .join(path)
    };
    let metadata = fs::metadata(&absolute)
        .map_err(|error| format!("inspect project path {}: {error}", absolute.display()))?;
    if !metadata.is_dir() {
        return Err(format!(
            "project path {} is not a directory",
            absolute.display()
        ));
    }
    Ok(absolute)
}

fn flatten_rendered_rule(body: &str) -> String {
    let body = body
        .lines()
        .filter(|line| {
            let line = line.trim();
            !line.starts_with("<!-- agentsync:fragment ")
                && !line.starts_with("<!-- /agentsync:fragment ")
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("{}\n", body.trim())
}

fn unique_rule_bodies(sources: &[ProjectRuleSource]) -> usize {
    sources
        .iter()
        .map(|source| source.body.trim())
        .collect::<BTreeSet<_>>()
        .len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::desktop::{Options, RuleAnalyzer};
    use crate::core::test_support::TestDir;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    fn options(dir: &TestDir) -> Options {
        Options {
            home: dir.path().join("agentsync-home"),
            projects_root: dir.path().join("projects"),
            native_root: dir.path().join("user-home"),
        }
    }

    fn write_config(home: &Path, agents: &[&str]) {
        fs::create_dir_all(home).unwrap();
        let mut body = "[agents]\n".to_string();
        for agent in agents {
            body.push_str(&format!("{agent} = {{ enabled = true }}\n"));
        }
        fs::write(home.join("agentsync.toml"), body).unwrap();
    }

    fn request(agents: &[&str]) -> RuleRequest {
        RuleRequest {
            scope: RuleScope::Global,
            project_path: String::new(),
            agents: agents.iter().map(|item| (*item).to_string()).collect(),
        }
    }

    #[test]
    fn save_preserves_fragments_and_previews_safe_targets() {
        let dir = TestDir::new("rules-save");
        let opts = options(&dir);
        write_config(&opts.home, &["codex", "gemini"]);
        source::write_memory(
            &opts.home,
            &source::Memory {
                body: "# Global\n\n@import ./fragments/style.md\n".to_string(),
                fragments: BTreeMap::from([(
                    "style.md".to_string(),
                    "Use existing styles.\n".to_string(),
                )]),
            },
        )
        .unwrap();
        let core = Core::new(opts);
        let document = core
            .save_rule(SaveRuleRequest {
                rule: request(&["codex", "gemini"]),
                body:
                    "# Global\n\nPrefer current project patterns.\n\n@import ./fragments/style.md\n"
                        .to_string(),
            })
            .unwrap();
        assert_eq!(document.fragments, ["style.md"]);
        let workspace = core.get_rules(request(&["codex", "gemini"])).unwrap();
        assert!(!workspace.blocked);
        assert_eq!(workspace.targets.len(), 2);
        assert!(workspace
            .targets
            .iter()
            .all(|target| target.status == "new" && !target.blocked));
    }

    #[test]
    fn save_does_not_rewrite_unchanged_config() {
        let dir = TestDir::new("rules-config-preserve");
        let opts = options(&dir);
        fs::create_dir_all(&opts.home).unwrap();
        let config = "# Keep this project note.\n\n[agents.codex]\nenabled = true\n";
        fs::write(opts.home.join("agentsync.toml"), config).unwrap();
        Core::new(opts.clone())
            .save_rule(SaveRuleRequest {
                rule: request(&["codex"]),
                body: "# Updated Rule\n".to_string(),
            })
            .unwrap();
        assert_eq!(
            fs::read_to_string(opts.home.join("agentsync.toml")).unwrap(),
            config
        );
    }

    #[test]
    fn empty_collections_serialize_as_arrays() {
        let dir = TestDir::new("rules-empty-arrays");
        let workspace = Core::new(options(&dir))
            .get_rules(RuleRequest::default())
            .unwrap();
        let value = serde_json::to_value(workspace).unwrap();
        assert!(value["document"]["agents"].is_array());
        assert!(value["targets"].is_array());
        assert!(value["availableAgents"].is_array());
    }

    #[test]
    fn empty_native_file_does_not_block() {
        let dir = TestDir::new("rules-empty-native");
        let opts = options(&dir);
        write_config(&opts.home, &["codex"]);
        fs::create_dir_all(opts.native_root.join(".codex")).unwrap();
        fs::write(opts.native_root.join(".codex/AGENTS.md"), "").unwrap();
        let workspace = Core::new(opts).get_rules(request(&["codex"])).unwrap();
        assert!(!workspace.blocked);
        assert_eq!(workspace.targets[0].status, "empty");
    }

    #[test]
    fn sync_empty_mother_template_does_not_create_native_rule() {
        let dir = TestDir::new("rules-sync-empty");
        let opts = options(&dir);
        write_config(&opts.home, &["codex"]);
        let core = Core::new(opts);
        let workspace = core.get_rules(request(&["codex"])).unwrap();
        let target = PathBuf::from(&workspace.targets[0].path);
        assert!(!workspace.targets[0].will_write);

        let result = core
            .sync_rules(SyncRulesRequest {
                rule: request(&["codex"]),
                resolution: String::new(),
            })
            .unwrap();

        assert!(result.applied);
        assert!(!target.exists());
    }

    #[test]
    fn sync_adopts_converged_native_rule_without_rewriting_it() {
        let dir = TestDir::new("rules-converged");
        let opts = options(&dir);
        write_config(&opts.home, &["codex"]);
        let core = Core::new(opts.clone());
        core.save_rule(SaveRuleRequest {
            rule: request(&["codex"]),
            body: "# Original\n".to_string(),
        })
        .unwrap();
        core.sync_rules(SyncRulesRequest {
            rule: request(&["codex"]),
            resolution: String::new(),
        })
        .unwrap();
        core.save_rule(SaveRuleRequest {
            rule: request(&["codex"]),
            body: "# Updated\n".to_string(),
        })
        .unwrap();
        let target = opts.native_root.join(".codex/AGENTS.md");
        let canonical = source::load(&opts.home).unwrap();
        let rendered = adapter::render_memory(
            "codex",
            adapter::Scope::User,
            &target,
            &canonical.memory,
            canonical.config.memory_banner,
        )
        .unwrap();
        fs::write(&target, rendered).unwrap();
        let preview = core.get_rules(request(&["codex"])).unwrap();
        assert_eq!(preview.targets[0].status, "converged");
        assert!(!preview.targets[0].will_write);

        let applied = core
            .sync_rules(SyncRulesRequest {
                rule: request(&["codex"]),
                resolution: String::new(),
            })
            .unwrap();
        assert!(applied.applied);
        assert_eq!(applied.preview.targets[0].status, "clean");
        fs::write(&target, "# Local edit\n").unwrap();
        assert_eq!(
            core.get_rules(request(&["codex"])).unwrap().targets[0].status,
            "drift"
        );
    }

    #[cfg(unix)]
    #[test]
    fn sync_records_first_success_when_later_target_write_fails() {
        use std::os::unix::fs::PermissionsExt;

        let dir = TestDir::new("rules-partial-write");
        let opts = options(&dir);
        write_config(&opts.home, &["codex", "gemini"]);
        source::write_memory(
            &opts.home,
            &source::Memory {
                body: "# Shared\n".to_string(),
                fragments: BTreeMap::new(),
            },
        )
        .unwrap();
        let locked_dir = opts.native_root.join(".gemini");
        fs::create_dir_all(&locked_dir).unwrap();
        fs::set_permissions(&locked_dir, fs::Permissions::from_mode(0o555)).unwrap();
        let core = Core::new(opts.clone());
        let result = core.sync_rules(SyncRulesRequest {
            rule: request(&["codex", "gemini"]),
            resolution: String::new(),
        });
        fs::set_permissions(&locked_dir, fs::Permissions::from_mode(0o755)).unwrap();

        assert!(result.is_err());
        assert!(opts.native_root.join(".codex/AGENTS.md").is_file());
        let preview = core.get_rules(request(&["codex", "gemini"])).unwrap();
        assert_eq!(preview.targets[0].status, "clean");
        assert_eq!(preview.targets[1].status, "new");
    }

    #[cfg(unix)]
    #[test]
    fn project_rule_rejects_symlinked_canonical_directory() {
        use std::os::unix::fs::symlink;

        let dir = TestDir::new("rules-project-canonical-symlink");
        let opts = options(&dir);
        let project = dir.path().join("project");
        let outside = dir.path().join("outside");
        fs::create_dir_all(&project).unwrap();
        fs::create_dir_all(&outside).unwrap();
        symlink(&outside, project.join(".agentsync")).unwrap();
        let result = Core::new(opts).save_rule(SaveRuleRequest {
            rule: RuleRequest {
                scope: RuleScope::Project,
                project_path: project.to_string_lossy().to_string(),
                agents: vec!["codex".to_string()],
            },
            body: "# Project\n".to_string(),
        });

        assert!(result.unwrap_err().contains("symlink"));
        assert!(!outside.join("agentsync.toml").exists());
    }

    #[cfg(unix)]
    #[test]
    fn project_rule_rejects_symlinked_native_parent() {
        use std::os::unix::fs::symlink;

        let dir = TestDir::new("rules-project-native-symlink");
        let opts = options(&dir);
        let project = dir.path().join("project");
        let outside = dir.path().join("outside");
        fs::create_dir_all(&project).unwrap();
        fs::create_dir_all(&outside).unwrap();
        symlink(&outside, project.join(".windsurf")).unwrap();
        source::write_memory(
            &project.join(".agentsync"),
            &source::Memory {
                body: "# Project\n".to_string(),
                fragments: BTreeMap::new(),
            },
        )
        .unwrap();
        let request = RuleRequest {
            scope: RuleScope::Project,
            project_path: project.to_string_lossy().to_string(),
            agents: vec!["windsurf".to_string()],
        };

        assert!(Core::new(opts)
            .get_rules(request)
            .unwrap_err()
            .contains("symlink"));
        assert!(!outside.join("rules/agentsync.md").exists());
    }

    #[test]
    fn write_rechecks_and_backs_up_foreign_content_created_after_preview() {
        let dir = TestDir::new("rules-late-collision");
        let opts = options(&dir);
        write_config(&opts.home, &["codex"]);
        source::write_memory(
            &opts.home,
            &source::Memory {
                body: "# Canonical\n".to_string(),
                fragments: BTreeMap::new(),
            },
        )
        .unwrap();
        let core = Core::new(opts);
        let target = core
            .get_rules(request(&["codex"]))
            .unwrap()
            .targets
            .remove(0);
        assert_eq!(target.status, "new");
        let path = PathBuf::from(&target.path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "# Created after preview\n").unwrap();

        let backup = core
            .write_rule_target(&target, b"# Canonical\n", None, None)
            .unwrap()
            .expect("foreign content must be preserved");

        assert_eq!(
            fs::read_to_string(&backup.backup_path).unwrap(),
            "# Created after preview\n"
        );
        assert_eq!(fs::read_to_string(path).unwrap(), "# Canonical\n");
    }

    #[test]
    fn sync_blocks_drift_then_backs_up_and_overwrites() {
        let dir = TestDir::new("rules-sync-drift");
        let opts = options(&dir);
        write_config(&opts.home, &["codex"]);
        source::write_memory(
            &opts.home,
            &source::Memory {
                body: "# Rule\n\nCanonical.\n".to_string(),
                fragments: BTreeMap::new(),
            },
        )
        .unwrap();
        let core = Core::new(opts);
        let first = core
            .sync_rules(SyncRulesRequest {
                rule: request(&["codex"]),
                resolution: String::new(),
            })
            .unwrap();
        assert!(first.applied);
        assert!(!first.preview.blocked);
        let target = PathBuf::from(&first.preview.targets[0].path);
        fs::write(&target, "# Hand edit\n\nKeep this.\n").unwrap();
        let blocked = core
            .sync_rules(SyncRulesRequest {
                rule: request(&["codex"]),
                resolution: String::new(),
            })
            .unwrap();
        assert!(!blocked.applied);
        assert!(blocked.preview.blocked);
        assert!(!blocked.preview.targets[0].diff.is_empty());
        let forced = core
            .sync_rules(SyncRulesRequest {
                rule: request(&["codex"]),
                resolution: "backup-overwrite".to_string(),
            })
            .unwrap();
        assert!(forced.applied);
        assert_eq!(forced.backups.len(), 1);
        assert!(fs::read_to_string(&forced.backups[0].backup_path)
            .unwrap()
            .contains("Keep this"));
        let destination = fs::read_to_string(target).unwrap();
        assert!(destination.contains("Canonical"));
        assert!(!destination.contains("Keep this"));
    }

    #[test]
    fn native_rule_imports_into_mother_template() {
        let dir = TestDir::new("rules-import-native");
        let opts = options(&dir);
        write_config(&opts.home, &["codex"]);
        let core = Core::new(opts);
        let workspace = core.get_rules(request(&["codex"])).unwrap();
        let path = PathBuf::from(&workspace.targets[0].path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "# Imported\n\nNative decision.\n").unwrap();
        let document = core
            .import_native_rule(ImportNativeRuleRequest {
                rule: request(&["codex"]),
                agent: "codex".to_string(),
            })
            .unwrap();
        assert!(document.body.contains("Native decision"));
    }

    #[test]
    fn preview_native_rule_reads_only_a_discovered_source_without_writing_canonical() {
        let dir = TestDir::new("rules-preview-native");
        let opts = options(&dir);
        fs::create_dir_all(opts.native_root.join(".gemini")).unwrap();
        fs::write(
            opts.native_root.join(".gemini/GEMINI.md"),
            "# Shared Rule\n",
        )
        .unwrap();
        let core = Core::new(opts.clone());
        let preview = core
            .preview_native_rule("antigravity-ide", "~/.gemini/GEMINI.md")
            .unwrap();
        assert_eq!(preview, "# Shared Rule\n");
        assert!(!opts.home.join("memory/AGENTS.md").exists());
        assert!(core
            .preview_native_rule("antigravity-ide", "~/private.txt")
            .is_err());
        assert!(core
            .preview_native_rule("cursor-account", "~/.gemini/GEMINI.md")
            .is_err());
    }

    #[cfg(unix)]
    #[test]
    fn preview_native_rule_rejects_symlinks_even_after_discovery() {
        use std::os::unix::fs::symlink;
        let dir = TestDir::new("rules-preview-native-symlink");
        let opts = options(&dir);
        fs::create_dir_all(opts.native_root.join(".gemini")).unwrap();
        symlink(dir.path(), opts.native_root.join(".gemini/GEMINI.md")).unwrap();
        assert!(Core::new(opts)
            .preview_native_rule("antigravity-ide", "~/.gemini/GEMINI.md")
            .is_err());
    }

    #[test]
    fn project_import_registers_plain_project_without_initializing_it() {
        let dir = TestDir::new("rules-import-project");
        let opts = options(&dir);
        let project = dir.path().join("outside/existing-project");
        fs::create_dir_all(&project).unwrap();
        fs::write(project.join("AGENTS.md"), "# Existing project rule\n").unwrap();
        let result = Core::new(opts.clone())
            .import_project(project.to_str().unwrap())
            .unwrap();
        assert_eq!(result.path, project.to_string_lossy());
        assert!(!result.sources.is_empty());
        assert!(!project.join(".agentsync").exists());
        assert!(opts
            .home
            .join(".state/agent-assistant/projects.json")
            .is_file());
    }

    struct StubAnalyzer {
        calls: Arc<AtomicUsize>,
    }

    impl RuleAnalyzer for StubAnalyzer {
        fn analyze(
            &self,
            _project_path: &Path,
            _sources: &[ProjectRuleSource],
        ) -> Result<RuleProposal, String> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            Ok(RuleProposal {
                body: "# Unified\n".to_string(),
                notes: vec!["Kept both policies.".to_string()],
                analyzer: "test-ai".to_string(),
                requires_ai: false,
            })
        }
    }

    #[test]
    fn project_analysis_returns_candidate_without_writing_it() {
        let dir = TestDir::new("rules-analyze-project");
        let opts = options(&dir);
        let project = dir.path().join("mixed-project");
        fs::create_dir_all(&project).unwrap();
        fs::write(project.join("AGENTS.md"), "# Shared\n").unwrap();
        fs::write(project.join("CLAUDE.md"), "# Claude\n").unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let mut core = Core::new(opts);
        core.analyzer = Some(Box::new(StubAnalyzer {
            calls: Arc::clone(&calls),
        }));
        core.import_project(project.to_str().unwrap()).unwrap();
        let proposal = core
            .analyze_project_rules(project.to_str().unwrap())
            .unwrap();
        assert_eq!(proposal.body, "# Unified\n");
        assert_eq!(calls.load(Ordering::Relaxed), 1);
        assert!(!project.join(".agentsync/memory/AGENTS.md").exists());
    }
}
