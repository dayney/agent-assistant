import type {
  ApplyPreview,
  McpPromotionResult,
  ProjectRuleImport,
  PushMcpToAgentRequest,
  PushMcpToAgentResult,
  RuleDocument,
  RuleProposal,
  RuleRequest,
  RuleSyncResult,
  RuleWorkspace,
  SaveMcpRequest,
  SaveMcpResult,
  VerifyMcpInAgentRequest,
  VerifyMcpInAgentResult,
  WorkspaceSnapshot,
} from "./model";

export interface CoreClient {
  getSnapshot(): Promise<WorkspaceSnapshot>;
  previewApply(): Promise<ApplyPreview>;
  getRules(request: RuleRequest): Promise<RuleWorkspace>;
  previewNativeRule(agent: string, path: string): Promise<string>;
  saveRule(request: RuleRequest & { body: string }): Promise<RuleDocument>;
  syncRules(
    request: RuleRequest & { resolution?: "backup-overwrite" },
  ): Promise<RuleSyncResult>;
  importNativeRule(
    request: RuleRequest & { agent: string },
  ): Promise<RuleDocument>;
  importProject(path: string): Promise<ProjectRuleImport>;
  analyzeProjectRules(path: string): Promise<RuleProposal>;
  promoteProjectMcp(projectPath: string, mcpId: string): Promise<McpPromotionResult>;
  promoteNativeMcp(agent: string, mcpId: string): Promise<McpPromotionResult>;
  importNativeMcpForce(agent: string, mcpId: string): Promise<McpPromotionResult>;
  saveGlobalMcp(request: SaveMcpRequest): Promise<SaveMcpResult>;
  pushMcpToAgent(request: PushMcpToAgentRequest): Promise<PushMcpToAgentResult>;
  verifyMcpInAgent(request: VerifyMcpInAgentRequest): Promise<VerifyMcpInAgentResult>;
}
