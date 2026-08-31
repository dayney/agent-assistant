import type { RuleTarget } from './model';

export function blockedRuleTargets(targets: RuleTarget[]): RuleTarget[] {
    return targets.filter((target) => target.blocked);
}

export function toggleAgentSelection(selected: string[], agent: string, checked: boolean): string[] {
    const next = new Set(selected);
    if (checked) next.add(agent);
    else next.delete(agent);
    return [...next].sort();
}
