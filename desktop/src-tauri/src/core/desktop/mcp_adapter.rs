use super::snapshot::NATIVE_MCP_TARGETS;
use crate::core::paths;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum McpNativeFormat {
    Json,
    Toml,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum McpTarget {
    Direct {
        relative: &'static str,
        shared_target_id: &'static str,
    },
    VerifiedSharedAlias {
        logical: &'static str,
        physical: &'static str,
        shared_target_id: &'static str,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct McpAdapterSpec {
    pub(crate) format: McpNativeFormat,
    target: McpTarget,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ResolvedMcpTarget {
    pub(crate) logical_path: PathBuf,
    pub(crate) physical_path: PathBuf,
    pub(crate) logical_relative: String,
    pub(crate) physical_relative: String,
    pub(crate) shared_target_id: String,
    pub(crate) format: McpNativeFormat,
}

pub(crate) fn spec_for(agent: &str) -> Result<McpAdapterSpec, String> {
    match agent {
        "antigravity-ide" | "antigravity-cli" => Ok(McpAdapterSpec {
            format: McpNativeFormat::Json,
            target: McpTarget::Direct {
                relative: ".gemini/config/mcp_config.json",
                shared_target_id: "gemini-shared-mcp-config",
            },
        }),
        "antigravity" => Ok(McpAdapterSpec {
            format: McpNativeFormat::Json,
            target: McpTarget::VerifiedSharedAlias {
                logical: ".gemini/antigravity/mcp_config.json",
                physical: ".gemini/config/mcp_config.json",
                shared_target_id: "gemini-shared-mcp-config",
            },
        }),
        "codex" => Ok(McpAdapterSpec {
            format: McpNativeFormat::Toml,
            target: McpTarget::Direct {
                relative: ".codex/config.toml",
                shared_target_id: "codex-mcp-config",
            },
        }),
        agent => {
            let relative = NATIVE_MCP_TARGETS
                .iter()
                .find(|(name, _)| *name == agent)
                .map(|(_, relative)| *relative)
                .ok_or_else(|| format!("no native MCP path for agent {agent}"))?;
            Ok(McpAdapterSpec {
                format: McpNativeFormat::Json,
                target: McpTarget::Direct {
                    relative,
                    shared_target_id: "native-mcp-config",
                },
            })
        }
    }
}

pub(crate) fn resolve_target(
    native_root: &Path,
    agent: &str,
) -> Result<(McpAdapterSpec, ResolvedMcpTarget), String> {
    let spec = spec_for(agent)?;
    let resolved = match spec.target {
        McpTarget::Direct {
            relative,
            shared_target_id,
        } => {
            let path = native_root.join(relative);
            paths::reject_symlinks_below(native_root, &path)?;
            ResolvedMcpTarget {
                logical_path: path.clone(),
                physical_path: path,
                logical_relative: relative.to_string(),
                physical_relative: relative.to_string(),
                shared_target_id: if shared_target_id == "native-mcp-config" {
                    agent.to_string()
                } else {
                    shared_target_id.to_string()
                },
                format: spec.format,
            }
        }
        McpTarget::VerifiedSharedAlias {
            logical,
            physical,
            shared_target_id,
        } => {
            let logical_path = native_root.join(logical);
            let physical_path = native_root.join(physical);
            let metadata = fs::symlink_metadata(&logical_path).map_err(|error| {
                format!(
                    "cannot inspect shared MCP alias {}: {error}",
                    logical_path.display()
                )
            })?;
            if !metadata.file_type().is_symlink() {
                return Err(format!(
                    "refusing unverified Antigravity MCP alias {}",
                    logical_path.display()
                ));
            }
            let alias_target = fs::canonicalize(&logical_path).map_err(|error| {
                format!(
                    "cannot resolve shared MCP alias {}: {error}",
                    logical_path.display()
                )
            })?;
            let expected_target = fs::canonicalize(&physical_path).map_err(|error| {
                format!(
                    "cannot resolve shared MCP target {}: {error}",
                    physical_path.display()
                )
            })?;
            if alias_target != expected_target {
                return Err(format!(
                    "refusing unverified Antigravity MCP alias {} -> {}",
                    logical_path.display(),
                    alias_target.display()
                ));
            }
            paths::reject_symlinks_below(native_root, &physical_path)?;
            ResolvedMcpTarget {
                logical_path,
                physical_path,
                logical_relative: logical.to_string(),
                physical_relative: physical.to_string(),
                shared_target_id: shared_target_id.to_string(),
                format: spec.format,
            }
        }
    };
    Ok((spec, resolved))
}
