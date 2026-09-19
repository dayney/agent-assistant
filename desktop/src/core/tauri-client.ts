import { invoke } from "@tauri-apps/api/core";
import type { CoreClient } from "./client";
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

export class TauriCoreClient implements CoreClient {
  async getSnapshot(): Promise<WorkspaceSnapshot> {
    try {
      return await invoke<WorkspaceSnapshot>("get_workspace_snapshot");
    } catch (error) {
      throw new Error(String(error));
    }
  }

  async previewApply(): Promise<ApplyPreview> {
    try {
      return await invoke<ApplyPreview>("preview_apply");
    } catch (error) {
      throw new Error(String(error));
    }
  }

  async getRules(request: RuleRequest): Promise<RuleWorkspace> {
    return invokeCore<RuleWorkspace>("rules_get", request);
  }

  async previewNativeRule(agent: string, path: string): Promise<string> {
    try {
      return await invoke<string>("rules_preview_native", { agent, path });
    } catch (error) {
      throw new Error(String(error));
    }
  }

  async saveRule(
    request: RuleRequest & { body: string },
  ): Promise<RuleDocument> {
    return invokeCore<RuleDocument>("rules_save", request);
  }

  async syncRules(
    request: RuleRequest & { resolution?: "backup-overwrite" },
  ): Promise<RuleSyncResult> {
    return invokeCore<RuleSyncResult>("rules_sync", request);
  }

  async importNativeRule(
    request: RuleRequest & { agent: string },
  ): Promise<RuleDocument> {
    return invokeCore<RuleDocument>("rules_import_native", request);
  }

  async importProject(path: string): Promise<ProjectRuleImport> {
    return invokeCore<ProjectRuleImport>("project_import", { path });
  }

  async analyzeProjectRules(path: string): Promise<RuleProposal> {
    return invokeCore<RuleProposal>("project_analyze_rules", { path });
  }

  async promoteProjectMcp(projectPath: string, mcpId: string): Promise<McpPromotionResult> {
    return invokeCore<McpPromotionResult>("mcp_promote_project", {
      projectPath,
      mcpId,
    });
  }

  async promoteNativeMcp(agent: string, mcpId: string): Promise<McpPromotionResult> {
    return invokeCore<McpPromotionResult>("mcp_promote_native", { agent, mcpId });
  }

  async importNativeMcpForce(agent: string, mcpId: string): Promise<McpPromotionResult> {
    return invokeCore<McpPromotionResult>("mcp_import_native_force", { agent, mcpId });
  }

  async saveGlobalMcp(request: SaveMcpRequest): Promise<SaveMcpResult> {
    return invokeCore<SaveMcpResult>("mcp_save", request);
  }

  async pushMcpToAgent(request: PushMcpToAgentRequest): Promise<PushMcpToAgentResult> {
    return invokeCore<PushMcpToAgentResult>("mcp_push_to_agent", request);
  }

  async verifyMcpInAgent(request: VerifyMcpInAgentRequest): Promise<VerifyMcpInAgentResult> {
    return invokeCore<VerifyMcpInAgentResult>("mcp_verify_in_agent", request);
  }
}

async function invokeCore<T>(command: string, request: unknown): Promise<T> {
  try {
    return await invoke<T>(command, { request });
  } catch (error) {
    throw new Error(String(error));
  }
}
