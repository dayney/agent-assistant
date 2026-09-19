use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::core::iox;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct FileEntry {
    pub(crate) sha256: String,
    pub(crate) mode: u32,
    pub(crate) applied_at: String,
    pub(crate) source_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct Targets {
    #[serde(default)]
    pub(crate) schema_version: u32,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) files: BTreeMap<String, FileEntry>,
    #[serde(flatten)]
    extra: BTreeMap<String, serde_json::Value>,
}

impl Default for Targets {
    fn default() -> Self {
        Self {
            schema_version: 1,
            files: BTreeMap::new(),
            extra: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct ProjectRegistry {
    pub(crate) schema_version: u32,
    pub(crate) projects: Vec<RegisteredProject>,
}

impl Default for ProjectRegistry {
    fn default() -> Self {
        Self {
            schema_version: 1,
            projects: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct RegisteredProject {
    pub(crate) path: PathBuf,
    pub(crate) added_at: String,
}

pub(crate) fn load_targets(path: &Path) -> Result<Targets, String> {
    let body = match fs::read(path) {
        Ok(body) => body,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Targets::default()),
        Err(error) => return Err(format!("read {}: {error}", path.display())),
    };
    let mut targets: Targets = serde_json::from_slice(&body)
        .map_err(|error| format!("parse {}: {error}", path.display()))?;
    if targets.schema_version == 0 {
        if targets.files.is_empty() && targets.extra.is_empty() {
            return Err(format!(
                "state file {} is empty or corrupt (no schema_version and no entries)",
                path.display()
            ));
        }
        targets.schema_version = 1;
    }
    if targets.schema_version != 1 {
        return Err(format!(
            "state schema_version={} is unsupported",
            targets.schema_version
        ));
    }
    Ok(targets)
}

pub(crate) fn save_targets(path: &Path, targets: &Targets) -> Result<(), String> {
    write_json(path, targets)
}

pub(crate) fn load_project_registry(path: &Path) -> Result<ProjectRegistry, String> {
    let body = match fs::read(path) {
        Ok(body) => body,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(ProjectRegistry::default())
        }
        Err(error) => return Err(format!("read {}: {error}", path.display())),
    };
    let registry: ProjectRegistry = serde_json::from_slice(&body)
        .map_err(|error| format!("parse {}: {error}", path.display()))?;
    if registry.schema_version != 1 {
        return Err(format!(
            "unsupported project registry schema {}",
            registry.schema_version
        ));
    }
    Ok(registry)
}

pub(crate) fn save_project_registry(path: &Path, registry: &ProjectRegistry) -> Result<(), String> {
    let mut registry = registry.clone();
    registry.schema_version = 1;
    let mut body = serde_json::to_vec_pretty(&registry)
        .map_err(|error| format!("serialize {}: {error}", path.display()))?;
    body.push(b'\n');
    iox::atomic_write_private(path, &body)
        .map_err(|error| format!("write {}: {error}", path.display()))
}

pub(crate) fn home_relative(home: &Path, path: &Path) -> String {
    if home.as_os_str().is_empty() || path.as_os_str().is_empty() {
        return path.to_string_lossy().to_string();
    }
    let Ok(relative) = path.strip_prefix(home) else {
        return path.to_string_lossy().to_string();
    };
    if relative.as_os_str().is_empty() {
        return "${HOME}".to_string();
    }
    format!(
        "${{HOME}}/{}",
        relative.to_string_lossy().replace('\\', "/")
    )
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let mut body = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("serialize {}: {error}", path.display()))?;
    body.push(b'\n');
    iox::atomic_write(path, &body).map_err(|error| format!("write {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::{
        home_relative, load_project_registry, load_targets, save_project_registry, save_targets,
        FileEntry, ProjectRegistry, RegisteredProject,
    };
    use crate::core::test_support::TestDir;

    #[test]
    fn missing_state_files_return_initialized_values() {
        let dir = TestDir::new("state-empty");
        let targets = load_targets(&dir.path().join("targets.json")).unwrap();
        assert_eq!(targets.schema_version, 1);
        assert!(targets.files.is_empty());
        let projects = load_project_registry(&dir.path().join("projects.json")).unwrap();
        assert_eq!(projects.schema_version, 1);
        assert!(projects.projects.is_empty());
    }

    #[test]
    fn state_round_trips_atomically() {
        let dir = TestDir::new("state-roundtrip");
        let path = dir.path().join("state/targets.json");
        let mut targets = load_targets(&path).unwrap();
        targets.files.insert(
            "codex:user::${HOME}/.codex/AGENTS.md".to_string(),
            FileEntry {
                sha256: "abc".to_string(),
                mode: 0o644,
                applied_at: "2026-09-17T00:00:00Z".to_string(),
                source_id: "memory/AGENTS.md".to_string(),
            },
        );
        save_targets(&path, &targets).unwrap();
        assert_eq!(load_targets(&path).unwrap().files.len(), 1);
    }

    #[test]
    fn legacy_state_without_schema_version_is_migrated() {
        let dir = TestDir::new("state-legacy");
        let path = dir.path().join("targets.json");
        std::fs::write(
            &path,
            r#"{
  "files": {
    "codex:user::${HOME}/.codex/AGENTS.md": {
      "sha256": "abc",
      "mode": 420,
      "applied_at": "2026-09-17T00:00:00Z",
      "source_id": "memory/AGENTS.md"
    }
  }
}"#,
        )
        .unwrap();

        let targets = load_targets(&path).unwrap();

        assert_eq!(targets.schema_version, 1);
        assert_eq!(targets.files.len(), 1);
    }

    #[test]
    fn project_registry_round_trips() {
        let dir = TestDir::new("project-registry");
        let path = dir.path().join("projects.json");
        let registry = ProjectRegistry {
            schema_version: 1,
            projects: vec![RegisteredProject {
                path: dir.path().join("repo"),
                added_at: "2026-09-17T00:00:00Z".to_string(),
            }],
        };
        save_project_registry(&path, &registry).unwrap();
        assert_eq!(load_project_registry(&path).unwrap().projects.len(), 1);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let mode = std::fs::metadata(path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        }
    }

    #[test]
    fn paths_under_home_are_portable() {
        assert_eq!(
            home_relative(
                std::path::Path::new("/Users/example"),
                std::path::Path::new("/Users/example/.codex/AGENTS.md")
            ),
            "${HOME}/.codex/AGENTS.md"
        );
        assert_eq!(
            home_relative(
                std::path::Path::new("/Users/example"),
                std::path::Path::new("/tmp/project")
            ),
            "/tmp/project"
        );
    }
}
