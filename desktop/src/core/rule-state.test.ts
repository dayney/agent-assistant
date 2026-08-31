import { describe, expect, it } from 'vitest';
import type { RuleTarget } from './model';
import { blockedRuleTargets, toggleAgentSelection } from './rule-state';

describe('Rule UI state', () => {
    it('returns only targets that require an explicit resolution', () => {
        const targets: RuleTarget[] = [
            { agent: 'codex', path: '/tmp/AGENTS.md', supported: true, status: 'clean', blocked: false, willWrite: false },
            { agent: 'claude', path: '/tmp/CLAUDE.md', supported: true, status: 'drift', blocked: true, willWrite: true, diff: '-old\n+new\n' },
        ];

        expect(blockedRuleTargets(targets).map((target) => target.agent)).toEqual(['claude']);
    });

    it('keeps Agent selection unique and deterministic', () => {
        expect(toggleAgentSelection(['gemini', 'codex'], 'cursor', true)).toEqual(['codex', 'cursor', 'gemini']);
        expect(toggleAgentSelection(['codex', 'cursor'], 'codex', false)).toEqual(['cursor']);
    });
});
