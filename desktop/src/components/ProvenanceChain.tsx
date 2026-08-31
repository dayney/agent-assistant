import type { ScopeKind } from '../core/model';

const labels: Record<ScopeKind, string> = {
    global: '全局',
    agent: 'Agent',
    project: '项目',
    rendered: '已渲染',
};

export function ProvenanceChain({ active = 'rendered' }: { active?: ScopeKind }) {
    const scopes: ScopeKind[] = ['global', 'agent', 'project', 'rendered'];
    const activeIndex = scopes.indexOf(active);

    return (
        <div className="provenance-chain" aria-label="配置来源链">
            {scopes.map((scope, index) => (
                <span className="provenance-step" key={scope}>
                    <span className={index <= activeIndex ? 'provenance-node is-active' : 'provenance-node'} />
                    <span>{labels[scope]}</span>
                    {index < scopes.length - 1 && <span className="provenance-line" aria-hidden="true" />}
                </span>
            ))}
        </div>
    );
}
