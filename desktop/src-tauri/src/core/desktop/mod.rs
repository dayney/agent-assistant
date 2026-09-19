mod analyzer;
mod mcp;
mod mcp_adapter;
mod model;
mod rules;
mod snapshot;

use std::path::PathBuf;

pub(crate) use analyzer::CodexRuleAnalyzer;
pub(crate) use model::{
    ApplyPreview, ImportNativeRuleRequest, McpPromotionResult, NativeMcpPromotionRequest, Options,
    ProjectMcpPromotionRequest, ProjectPathRequest, ProjectRuleImport, PushMcpToAgentRequest,
    PushMcpToAgentResult, RuleDocument, RuleProposal, RuleRequest, RuleSyncResult, RuleWorkspace,
    SaveMcpRequest, SaveMcpResult, SaveRuleRequest, SyncRulesRequest, VerifyMcpInAgentRequest,
    VerifyMcpInAgentResult, WorkspaceSnapshot,
};

pub(crate) trait RuleAnalyzer: Send + Sync {
    fn analyze(
        &self,
        project_path: &std::path::Path,
        sources: &[model::ProjectRuleSource],
    ) -> Result<RuleProposal, String>;
}

pub(crate) struct Core {
    pub(super) opts: Options,
    pub(super) analyzer: Option<Box<dyn RuleAnalyzer>>,
}

impl Core {
    pub(crate) fn new(opts: Options) -> Self {
        Self {
            opts,
            analyzer: None,
        }
    }

    pub(crate) fn from_env() -> Result<Self, String> {
        let native_root = crate::core::paths::home_dir_from(std::env::var_os("HOME"))?;
        let home = std::env::var_os("AGENTSYNC_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| crate::core::paths::agentsync_home(&native_root));
        let projects_root = std::env::var_os("AGENT_ASSISTANT_PROJECTS_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|| native_root.join("git/work"));
        let mut core = Self::new(Options {
            home,
            projects_root,
            native_root,
        });
        core.analyzer = Some(Box::new(CodexRuleAnalyzer));
        Ok(core)
    }
}
