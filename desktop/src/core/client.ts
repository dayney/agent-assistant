import type {
    ApplyPreview,
    ProjectRuleImport,
    RuleDocument,
    RuleProposal,
    RuleRequest,
    RuleSyncResult,
    RuleWorkspace,
    WorkspaceSnapshot,
} from './model';

export interface CoreClient {
    getSnapshot(): Promise<WorkspaceSnapshot>;
    previewApply(): Promise<ApplyPreview>;
    getRules(request: RuleRequest): Promise<RuleWorkspace>;
    saveRule(request: RuleRequest & { body: string }): Promise<RuleDocument>;
    syncRules(request: RuleRequest & { resolution?: 'backup-overwrite' }): Promise<RuleSyncResult>;
    importNativeRule(request: RuleRequest & { agent: string }): Promise<RuleDocument>;
    importProject(path: string): Promise<ProjectRuleImport>;
    analyzeProjectRules(path: string): Promise<RuleProposal>;
}
