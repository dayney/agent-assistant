export type ScopeKind = "global" | "agent" | "project" | "rendered";

export type CapabilityState = "full" | "partial" | "unsupported" | "unknown";

export type CoreMode = "demo" | "real";

export type HealthState = "good" | "warn" | "bad";

export type ManagedComponent =
  "rules" | "skills" | "mcp" | "commands" | "hooks" | "subagents" | "workflows";

export interface CapabilityReport {
  memory: CapabilityState;
  components: Record<ManagedComponent, CapabilityState>;
  note?: string;
}

export interface AgentSummary {
  id: string;
  name: string;
  vendor: string;
  version: string;
  health: HealthState;
  syncState: "synced" | "discovered" | "attention" | "not-configured";
  capabilities: CapabilityReport;
  lastApplied: string;
}

export interface GlobalRuleItem {
  id: string;
  name: string;
  description: string;
  enabled: boolean;
  source: ScopeKind;
}

export interface NativeRuleSource {
  agent: string;
  kind: "primary" | "auxiliary";
  path: string;
  state: "present" | "directory" | "empty" | "missing" | "unavailable" | "unsafe" | "unreadable";
  reason: string;
}

export interface GlobalMcpItem {
  id: string;
  name: string;
  description: string;
  transport: "stdio" | "sse" | "http";
  endpoint: string;
  secretRefs: string[];
  source: ScopeKind;
  scope: "global" | "agent" | "project";
  recipe: string;
  recipeStatus: "ready" | "missing-secret" | "plaintext-secret" | "legacy" | "unknown-recipe" | "missing-recipe";
  credentialState: "configured" | "not-required" | "plaintext-blocked";
  agents: string[];
  category: "global" | "shared" | "agent";
  canPromote: boolean;
  serverId: string;
  credentials: McpCredential[];
  adapters: McpAdapter[];
}

export interface McpCredential {
  key: string;
  source: "secret-vault" | "environment" | "literal" | "not-configured";
  status: "configured" | "missing" | "plaintext-blocked";
  reference?: string;
}

export interface McpAdapter {
  agent: string;
  status:
    | "supported"
    | "partial"
    | "unsupported"
    | "unverified"
    | "native"
    | "project";
}

export interface ProjectMcpItem {
  id: string;
  name: string;
  transport: "stdio" | "sse" | "http";
  endpoint: string;
  secretRefs: string[];
  recipe: string;
  recipeStatus: GlobalMcpItem["recipeStatus"];
  credentialState: GlobalMcpItem["credentialState"];
  credentials: McpCredential[];
  adapters: McpAdapter[];
}

export interface NativeMcpSource {
  agent: string;
  path: string;
  state: "parsed" | "unverified" | "empty" | "missing" | "unavailable" | "unsafe" | "unreadable" | "invalid" | "oversized";
  reason: string;
  serverCount: number;
}

export interface NativeSkillItem {
  name: string;
  path: string;
  state: "present" | "empty" | "unsafe" | "unreadable" | "unverified";
}

export interface NativeSkillSource {
  agent: string;
  path: string;
  state: "present" | "partial" | "empty" | "missing" | "unavailable" | "unsafe" | "unreadable";
  reason: string;
  items: NativeSkillItem[];
}

export interface NativeWorkflowSource {
  agent: string;
  path: string;
  state: NativeSkillSource["state"];
  reason: string;
  items: NativeSkillItem[];
}

export interface NativeHookSource {
  agent: string;
  path: string;
  state: "present" | "unverified" | "no-hooks" | "invalid" | "oversized" | "empty" | "missing" | "unavailable" | "unsafe" | "unreadable";
  reason: string;
}

export interface ProjectSummary {
  id: string;
  name: string;
  path: string;
  profile: string;
  stack: string[];
  agents: string[];
  mcp: ProjectMcpItem[];
  syncState: "synced" | "discovered" | "attention" | "not-configured";
  updatedAt: string;
}

export interface ActivityItem {
  id: string;
  title: string;
  detail: string;
  timestamp: string;
  tone: "success" | "warning" | "info";
}

export interface WorkspaceSnapshot {
  schemaVersion: number;
  mode: CoreMode;
  generatedAt: string;
  metrics: {
    agents: number;
    projects: number;
    components: number;
    attention: number;
  };
  global: {
    canonicalState: "ready" | "missing" | "empty";
    canonicalPath: string;
    rules: GlobalRuleItem[];
    nativeRules: NativeRuleSource[];
    mcp: GlobalMcpItem[];
    nativeMcpItems: GlobalMcpItem[];
    nativeMcp: NativeMcpSource[];
    skills: number;
    nativeSkills: NativeSkillSource[];
    hooks: number;
    nativeHooks: NativeHookSource[];
    subagents: number;
    nativeSubagents: NativeSkillSource[];
    workflows: number;
    nativeWorkflows: NativeWorkflowSource[];
  };
  agents: AgentSummary[];
  projects: ProjectSummary[];
  activity: ActivityItem[];
}

export interface ApplyPreview {
  files: number;
  partial: number;
  unsupported: number;
  warnings: string[];
}

export interface McpPromotionResult {
  id: string;
  path: string;
  status: "promoted" | "already-global" | "force-imported";
}

export type RuleScope = "global" | "project";

export interface RuleRequest {
  scope: RuleScope;
  projectPath?: string;
  agents?: string[];
}

export interface RuleDocument {
  scope: RuleScope;
  projectPath?: string;
  canonicalPath: string;
  body: string;
  fragments: string[];
  agents: string[];
}

export interface RuleTarget {
  agent: string;
  path?: string;
  supported: boolean;
  status: string;
  blocked: boolean;
  willWrite: boolean;
  diff?: string;
  reason?: string;
}

export interface RuleWorkspace {
  document: RuleDocument;
  targets: RuleTarget[];
  availableAgents: string[];
  blocked: boolean;
}

export interface RuleSyncResult {
  preview: RuleWorkspace;
  applied: boolean;
  backups: Array<{ agent: string; sourcePath: string; backupPath: string }>;
}

export interface ProjectRuleSource {
  path: string;
  agents: string[];
  body: string;
}

export interface ProjectRuleImport {
  path: string;
  sources: ProjectRuleSource[];
  needsAnalysis: boolean;
}

export interface RuleProposal {
  body: string;
  notes: string[];
  analyzer: string;
  requiresAI: boolean;
}

export interface SaveMcpRequest {
  mcpId: string;
  name: string;
  /** 用户自定义功能简介 */
  description: string;
  recipe: string;
  /** stdio 端点 */
  command: string;
  /** SSE/HTTP 端点 */
  url: string;
  /** 凭据字段映射：key → reference（如 ${secret:X}） */
  env: Record<string, string>;
}

export interface SaveMcpResult {
  id: string;
  path: string;
}

export interface PushMcpToAgentRequest {
  mcpId: string;
  agent: string;
}

export interface PushMcpToAgentResult {
  agent: string;
  path: string;
  physicalPath: string;
  sharedTargetId: string;
  /** "written" | "updated" | "unsupported" */
  status: string;
}

export interface VerifyMcpInAgentRequest {
  mcpId: string;
  agent: string;
  runtimeCheck?: boolean;
}

export interface VerifyMcpInAgentResult {
  agent: string;
  path: string;
  physicalPath: string;
  sharedTargetId: string;
  mismatchedFields: string[];
  runtimeStatus: string;
  /** "found" | "missing" | "file-missing" | "parse-error" | "unsupported" */
  status: string;
}
