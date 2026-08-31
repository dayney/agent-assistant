export type ScopeKind = 'global' | 'agent' | 'project' | 'rendered';

export type CapabilityState = 'full' | 'partial' | 'unsupported';

export type CoreMode = 'demo' | 'real';

export type HealthState = 'good' | 'warn' | 'bad';

export type ManagedComponent =
    | 'rules'
    | 'skills'
    | 'mcp'
    | 'commands'
    | 'hooks'
    | 'subagents';

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
    syncState: 'synced' | 'discovered' | 'attention' | 'not-configured';
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

export interface GlobalMcpItem {
    id: string;
    name: string;
    description: string;
    transport: 'stdio' | 'sse' | 'http';
    endpoint: string;
    secretRefs: string[];
    source: ScopeKind;
}

export interface ProjectSummary {
    id: string;
    name: string;
    path: string;
    profile: string;
    stack: string[];
    agents: string[];
    syncState: 'synced' | 'discovered' | 'attention' | 'not-configured';
    updatedAt: string;
}

export interface ActivityItem {
    id: string;
    title: string;
    detail: string;
    timestamp: string;
    tone: 'success' | 'warning' | 'info';
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
        canonicalState: 'ready' | 'missing' | 'empty';
        canonicalPath: string;
        rules: GlobalRuleItem[];
        mcp: GlobalMcpItem[];
        skills: number;
        hooks: number;
        subagents: number;
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

export type RuleScope = 'global' | 'project';

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
