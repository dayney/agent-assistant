import type { RuleTarget } from './model';

export interface RulePreviewIdentity {
    scope: 'global' | 'project';
    projectPath?: string;
    body: string;
    selectedAgents: string[];
}

export interface RuleCommandContext extends RulePreviewIdentity {
    savedBody: string;
    previewKey: string | null;
    targets: RuleTarget[];
    busy: boolean;
}

export interface RuleCommandState {
    dirty: boolean;
    previewFresh: boolean;
    canSave: boolean;
    canPreview: boolean;
    canSync: boolean;
    hasBlockedTargets: boolean;
    primaryAction: 'save' | 'preview' | 'sync' | 'none';
}

export function createRulePreviewKey(input: RulePreviewIdentity): string {
    return JSON.stringify({
        scope: input.scope,
        projectPath: input.projectPath,
        body: input.body,
        selectedAgents: [...input.selectedAgents].sort(),
    });
}

export function deriveRuleCommandState(input: RuleCommandContext): RuleCommandState {
    const dirty = input.body !== input.savedBody;
    const hasBlockedTargets = blockedRuleTargets(input.targets).length > 0;
    const previewFresh = input.previewKey !== null && input.previewKey === createRulePreviewKey(input);
    const hasTargets = input.targets.length > 0;
    const canSave = !input.busy && dirty && input.body.trim().length > 0;
    const canPreview = !input.busy && !dirty && hasTargets;
    const canSync = !input.busy && previewFresh && hasTargets && !hasBlockedTargets;

    return {
        dirty,
        previewFresh,
        canSave,
        canPreview,
        canSync,
        hasBlockedTargets,
        primaryAction: canSync ? 'sync' : canSave ? 'save' : canPreview ? 'preview' : 'none',
    };
}

export function blockedRuleTargets(targets: RuleTarget[]): RuleTarget[] {
    return targets.filter((target) => target.blocked);
}

export function toggleAgentSelection(selected: string[], agent: string, checked: boolean): string[] {
    const next = new Set(selected);
    if (checked) next.add(agent);
    else next.delete(agent);
    return [...next].sort();
}
