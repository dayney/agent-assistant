import type { AgentSummary, ManagedComponent, WorkspaceSnapshot } from '../core/model';
import { StatusBadge } from '../components/StatusBadge';

const components: Array<{ id: ManagedComponent; label: string }> = [
    { id: 'rules', label: '规则' }, { id: 'skills', label: 'Skills' }, { id: 'mcp', label: 'MCP' }, { id: 'commands', label: 'Commands' }, { id: 'hooks', label: 'Hooks' }, { id: 'subagents', label: 'Subagents' },
];

export function AgentsView({ snapshot }: { snapshot: WorkspaceSnapshot }) {
    return (
        <div className="view-stack">
            <section className="page-heading"><p className="eyebrow">Agent Adapters</p><h1>Agent</h1><p className="lede">对比每个适配器的原生能力，应用前明确看到完整、部分支持和不支持。</p></section>
            <section className="panel matrix-panel">
                <div className="panel-header"><div><p className="eyebrow">Capability Matrix</p><h2>{snapshot.agents.length} 个适配器</h2></div><button className="button button-quiet" type="button"><span aria-hidden="true">↻</span> 重新检测</button></div>
                <div className="matrix-wrap"><table className="capability-table"><thead><tr><th>Agent</th><th>状态</th>{components.map((component) => <th key={component.id}>{component.label}</th>)}<th>最近应用</th></tr></thead><tbody>{snapshot.agents.map((agent) => <AgentRow agent={agent} key={agent.id} />)}</tbody></table></div>
                <div className="matrix-legend"><span><i className="legend-dot dot-full" />完整</span><span><i className="legend-dot dot-partial" />部分支持</span><span><i className="legend-dot dot-unsupported" />不支持</span></div>
            </section>
        </div>
    );
}

function AgentRow({ agent }: { agent: AgentSummary }) {
    return <tr><td><div className="table-agent"><span className="agent-avatar" aria-hidden="true">{agent.name.slice(0, 1)}</span><div><strong>{agent.name}</strong><small>{agent.vendor} · {agent.version}</small></div></div></td><td><StatusBadge value={agent.syncState} /></td>{components.map((component) => <td key={component.id}><StatusBadge value={agent.capabilities.components[component.id]} /></td>)}<td className="last-applied">{agent.lastApplied}</td></tr>;
}
