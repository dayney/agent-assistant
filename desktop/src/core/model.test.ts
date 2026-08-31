import { describe, expect, it } from 'vitest';
import { createDemoSnapshot } from './demo-client';

describe('workspace model', () => {
    it('marks the first snapshot as demo data', () => {
        expect(createDemoSnapshot().mode).toBe('demo');
    });

    it('keeps explicit capability states for every agent', () => {
        const snapshot = createDemoSnapshot();
        const states = new Set(['full', 'partial', 'unsupported']);

        expect(snapshot.agents).toHaveLength(4);
        for (const agent of snapshot.agents) {
            for (const state of Object.values(agent.capabilities.components)) {
                expect(states.has(state)).toBe(true);
            }
        }
    });

    it('preserves provenance and secret references without secret values', () => {
        const snapshot = createDemoSnapshot();
        const github = snapshot.global.mcp.find((item) => item.id === 'github');

        expect(github?.source).toBe('global');
        expect(github?.secretRefs).toContain('${secret:GITHUB_TOKEN}');
    });
});
