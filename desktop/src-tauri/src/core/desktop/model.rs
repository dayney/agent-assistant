use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub(crate) struct Options {
    pub(crate) home: PathBuf,
    pub(crate) projects_root: PathBuf,
    pub(crate) native_root: PathBuf,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkspaceSnapshot {
    pub(crate) schema_version: u32,
    pub(crate) mode: String,
    pub(crate) generated_at: String,
    pub(crate) metrics: Metrics,
    pub(crate) global: Global,
    pub(crate) agents: Vec<AgentSummary>,
    pub(crate) projects: Vec<ProjectSummary>,
    pub(crate) activity: Vec<ActivityItem>,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Metrics {
    pub(crate) agents: usize,
    pub(crate) projects: usize,
    pub(crate) components: usize,
    pub(crate) attention: usize,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Global {
    pub(crate) canonical_state: String,
    pub(crate) canonical_path: String,
    pub(crate) rules: Vec<GlobalRuleItem>,
    pub(crate) native_rules: Vec<NativeRuleSource>,
    pub(crate) mcp: Vec<GlobalMcpItem>,
    pub(crate) native_mcp_items: Vec<GlobalMcpItem>,
    pub(crate) native_mcp: Vec<NativeMcpSource>,
    pub(crate) skills: usize,
    pub(crate) native_skills: Vec<NativeSkillSource>,
    pub(crate) hooks: usize,
    pub(crate) native_hooks: Vec<NativeHookSource>,
    pub(crate) subagents: usize,
    pub(crate) native_subagents: Vec<NativeSkillSource>,
    pub(crate) workflows: usize,
    pub(crate) native_workflows: Vec<NativeWorkflowSource>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NativeRuleSource {
    pub(crate) agent: String,
    pub(crate) kind: String,
    pub(crate) path: String,
    pub(crate) state: String,
    pub(crate) reason: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GlobalRuleItem {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) enabled: bool,
    pub(crate) source: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GlobalMcpItem {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) transport: String,
    pub(crate) endpoint: String,
    pub(crate) secret_refs: Vec<String>,
    pub(crate) source: String,
    pub(crate) scope: String,
    pub(crate) recipe: String,
    pub(crate) recipe_status: String,
    pub(crate) credential_state: String,
    pub(crate) agents: Vec<String>,
    pub(crate) category: String,
    pub(crate) can_promote: bool,
    pub(crate) server_id: String,
    pub(crate) credentials: Vec<McpCredential>,
    pub(crate) adapters: Vec<McpAdapter>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct McpCredential {
    pub(crate) key: String,
    pub(crate) source: String,
    pub(crate) status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) reference: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct McpAdapter {
    pub(crate) agent: String,
    pub(crate) status: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProjectMcpItem {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) transport: String,
    pub(crate) endpoint: String,
    pub(crate) secret_refs: Vec<String>,
    pub(crate) recipe: String,
    pub(crate) recipe_status: String,
    pub(crate) credential_state: String,
    pub(crate) credentials: Vec<McpCredential>,
    pub(crate) adapters: Vec<McpAdapter>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NativeMcpSource {
    pub(crate) agent: String,
    pub(crate) path: String,
    pub(crate) state: String,
    pub(crate) reason: String,
    pub(crate) server_count: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NativeSkillSource {
    pub(crate) agent: String,
    pub(crate) path: String,
    pub(crate) state: String,
    pub(crate) reason: String,
    pub(crate) items: Vec<NativeSkillItem>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NativeSkillItem {
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) state: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NativeWorkflowSource {
    pub(crate) agent: String,
    pub(crate) path: String,
    pub(crate) state: String,
    pub(crate) reason: String,
    pub(crate) items: Vec<NativeSkillItem>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NativeHookSource {
    pub(crate) agent: String,
    pub(crate) path: String,
    pub(crate) state: String,
    pub(crate) reason: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CapabilityReport {
    pub(crate) memory: String,
    pub(crate) components: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) note: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentSummary {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) vendor: String,
    pub(crate) version: String,
    pub(crate) health: String,
    pub(crate) sync_state: String,
    pub(crate) capabilities: CapabilityReport,
    pub(crate) last_applied: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProjectSummary {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) profile: String,
    pub(crate) stack: Vec<String>,
    pub(crate) agents: Vec<String>,
    pub(crate) mcp: Vec<ProjectMcpItem>,
    pub(crate) sync_state: String,
    pub(crate) updated_at: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProjectMcpPromotionRequest {
    pub(crate) project_path: String,
    pub(crate) mcp_id: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NativeMcpPromotionRequest {
    pub(crate) agent: String,
    pub(crate) mcp_id: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct McpPromotionResult {
    pub(crate) id: String,
    pub(crate) path: String,
    pub(crate) status: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ActivityItem {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) detail: String,
    pub(crate) timestamp: String,
    pub(crate) tone: String,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ApplyPreview {
    pub(crate) files: usize,
    pub(crate) partial: usize,
    pub(crate) unsupported: usize,
    pub(crate) warnings: Vec<String>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum RuleScope {
    #[default]
    Global,
    Project,
}

impl RuleScope {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::Project => "project",
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RuleRequest {
    #[serde(default)]
    pub(crate) scope: RuleScope,
    #[serde(default)]
    pub(crate) project_path: String,
    #[serde(default)]
    pub(crate) agents: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaveRuleRequest {
    #[serde(flatten)]
    pub(crate) rule: RuleRequest,
    pub(crate) body: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SyncRulesRequest {
    #[serde(flatten)]
    pub(crate) rule: RuleRequest,
    #[serde(default)]
    pub(crate) resolution: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImportNativeRuleRequest {
    #[serde(flatten)]
    pub(crate) rule: RuleRequest,
    pub(crate) agent: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RuleDocument {
    pub(crate) scope: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) project_path: String,
    pub(crate) canonical_path: String,
    pub(crate) body: String,
    pub(crate) fragments: Vec<String>,
    pub(crate) agents: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RuleTarget {
    pub(crate) agent: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) path: String,
    pub(crate) supported: bool,
    pub(crate) status: String,
    pub(crate) blocked: bool,
    pub(crate) will_write: bool,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) diff: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) reason: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RuleWorkspace {
    pub(crate) document: RuleDocument,
    pub(crate) targets: Vec<RuleTarget>,
    pub(crate) available_agents: Vec<String>,
    pub(crate) blocked: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RuleBackup {
    pub(crate) agent: String,
    pub(crate) source_path: String,
    pub(crate) backup_path: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RuleSyncResult {
    pub(crate) preview: RuleWorkspace,
    pub(crate) applied: bool,
    pub(crate) backups: Vec<RuleBackup>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProjectRuleSource {
    pub(crate) path: String,
    pub(crate) agents: Vec<String>,
    pub(crate) body: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProjectRuleImport {
    pub(crate) path: String,
    pub(crate) sources: Vec<ProjectRuleSource>,
    pub(crate) needs_analysis: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct RuleProposal {
    pub(crate) body: String,
    pub(crate) notes: Vec<String>,
    pub(crate) analyzer: String,
    pub(crate) requires_ai: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct ProjectPathRequest {
    pub(crate) path: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaveMcpRequest {
    pub(crate) mcp_id: String,
    pub(crate) name: String,
    /// 用户自定义功能简介
    pub(crate) description: String,
    pub(crate) recipe: String,
    /// stdio 端点：command
    pub(crate) command: String,
    /// SSE/HTTP 端点：url
    pub(crate) url: String,
    /// 凭据字段映射：key → reference（如 ${secret:X}）
    pub(crate) env: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaveMcpResult {
    pub(crate) id: String,
    pub(crate) path: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PushMcpToAgentRequest {
    /// global MCP 的 serverId
    pub(crate) mcp_id: String,
    /// 目标 Agent 标识，如 "cursor" / "antigravity" / "gemini"
    pub(crate) agent: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PushMcpToAgentResult {
    pub(crate) agent: String,
    pub(crate) path: String,
    pub(crate) physical_path: String,
    pub(crate) shared_target_id: String,
    /// "written" | "updated" | "unsupported"
    pub(crate) status: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VerifyMcpInAgentRequest {
    pub(crate) mcp_id: String,
    pub(crate) agent: String,
    #[serde(default)]
    pub(crate) runtime_check: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VerifyMcpInAgentResult {
    pub(crate) agent: String,
    pub(crate) path: String,
    pub(crate) physical_path: String,
    pub(crate) shared_target_id: String,
    pub(crate) mismatched_fields: Vec<String>,
    pub(crate) runtime_status: String,
    /// "found" | "missing" | "file-missing" | "parse-error" | "unsupported"
    pub(crate) status: String,
}
