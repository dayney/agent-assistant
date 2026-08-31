import { invoke } from '@tauri-apps/api/core';
import type { CoreClient } from './client';
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

export class TauriCoreClient implements CoreClient {
    async getSnapshot(): Promise<WorkspaceSnapshot> {
        try {
            return await invoke<WorkspaceSnapshot>('get_workspace_snapshot');
        } catch (error) {
            throw new Error(String(error));
        }
    }

    async previewApply(): Promise<ApplyPreview> {
        try {
            return await invoke<ApplyPreview>('preview_apply');
        } catch (error) {
            throw new Error(String(error));
        }
    }

    async getRules(request: RuleRequest): Promise<RuleWorkspace> {
        return invokeCore<RuleWorkspace>('rules_get', request);
    }

    async saveRule(request: RuleRequest & { body: string }): Promise<RuleDocument> {
        return invokeCore<RuleDocument>('rules_save', request);
    }

    async syncRules(request: RuleRequest & { resolution?: 'backup-overwrite' }): Promise<RuleSyncResult> {
        return invokeCore<RuleSyncResult>('rules_sync', request);
    }

    async importNativeRule(request: RuleRequest & { agent: string }): Promise<RuleDocument> {
        return invokeCore<RuleDocument>('rules_import_native', request);
    }

    async importProject(path: string): Promise<ProjectRuleImport> {
        return invokeCore<ProjectRuleImport>('project_import', { path });
    }

    async analyzeProjectRules(path: string): Promise<RuleProposal> {
        return invokeCore<RuleProposal>('project_analyze_rules', { path });
    }
}

async function invokeCore<T>(command: string, request: unknown): Promise<T> {
    try {
        return await invoke<T>(command, { request });
    } catch (error) {
        throw new Error(String(error));
    }
}
