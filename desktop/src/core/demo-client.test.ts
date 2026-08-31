import { describe, expect, it } from 'vitest';
import { createDemoApplyPreview, createDemoSnapshot } from './demo-client';

describe('demo governance client', () => {
    it('returns an explicitly marked demo snapshot with capability states', () => {
        const snapshot = createDemoSnapshot();

        expect(snapshot.mode).toBe('demo');
        expect(snapshot.agents).toHaveLength(4);
        expect(snapshot.agents[2].capabilities.components.skills).toBe('unsupported');
    });

    it('returns warnings before an apply can be approved', () => {
        const preview = createDemoApplyPreview();

        expect(preview.files).toBeGreaterThan(0);
        expect(preview.unsupported).toBe(1);
        expect(preview.warnings).toHaveLength(2);
    });
});
