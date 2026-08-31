import { useState } from 'react';
import type { WorkspaceSnapshot } from '../core/model';
import { ProvenanceChain } from '../components/ProvenanceChain';

type GlobalTab = 'rules' | 'mcp' | 'skills' | 'hooks' | 'subagents';

const tabs: Array<{ id: GlobalTab; label: string }> = [
    { id: 'rules', label: '规则' }, { id: 'mcp', label: 'MCP' }, { id: 'skills', label: 'Skills' }, { id: 'hooks', label: 'Hooks' }, { id: 'subagents', label: 'Subagents' },
];

export function GlobalView({ snapshot }: { snapshot: WorkspaceSnapshot }) {
    const [tab, setTab] = useState<GlobalTab>('rules');
    const count = tab === 'rules' ? snapshot.global.rules.length : tab === 'mcp' ? snapshot.global.mcp.length : snapshot.global[tab];

    return (
        <div className="view-stack">
            <section className="page-heading"><p className="eyebrow">Global Scope</p><h1>全局配置</h1><p className="lede">这里是所有 Agent 和项目共享的治理基线。</p></section>
            <div className="content-grid global-grid">
                <section className="panel panel-main">
                    <div className="tab-list" role="tablist" aria-label="全局组件类型">
                        {tabs.map((item) => <button key={item.id} className={tab === item.id ? 'tab is-active' : 'tab'} type="button" role="tab" aria-selected={tab === item.id} onClick={() => setTab(item.id)}>{item.label}<span>{item.id === 'rules' ? snapshot.global.rules.length : item.id === 'mcp' ? snapshot.global.mcp.length : snapshot.global[item.id]}</span></button>)}
                    </div>
                    <div className="tab-content">
                        <div className="panel-header"><div><p className="eyebrow">{count} 个条目</p><h2>{tabs.find((item) => item.id === tab)?.label}</h2></div><button className="button button-primary" type="button"><span aria-hidden="true">＋</span> 新建</button></div>
                        {tab === 'rules' && <div className="rule-list">{snapshot.global.rules.map((rule) => <div className="rule-row" key={rule.id}><div className="rule-switch" aria-hidden="true">{rule.enabled ? '✓' : '−'}</div><div><strong>{rule.name}</strong><p>{rule.description}</p></div><span className="scope-chip">全局</span><button className="icon-button" aria-label={`打开 ${rule.name}`} title={`打开 ${rule.name}`} type="button">•••</button></div>)}</div>}
                        {tab === 'mcp' && <div className="mcp-list">{snapshot.global.mcp.map((server) => <div className="mcp-row" key={server.id}><div className="mcp-mark" aria-hidden="true">⌁</div><div className="mcp-copy"><strong>{server.name}</strong><p>{server.description}</p><code>{server.endpoint}</code></div><div className="mcp-meta"><span className="scope-chip">{server.source === 'global' ? '全局' : server.source === 'agent' ? 'Agent 原生' : '项目覆盖'}</span><small>{server.secretRefs.length ? `${server.secretRefs.length} 个引用` : '敏感值已隐藏'}</small></div></div>)}</div>}
                        {tab !== 'rules' && tab !== 'mcp' && <div className="empty-state"><span className="empty-icon" aria-hidden="true">◌</span><strong>{tabs.find((item) => item.id === tab)?.label} 已纳入基线</strong><p>当前工作区有 {count} 个条目，详细编辑将在 Core 连接后开放。</p></div>}
                    </div>
                </section>
                <aside className="panel panel-side source-panel"><p className="eyebrow">基线来源</p><h2>公共配置</h2><ProvenanceChain active="global" />{snapshot.global.canonicalState === 'missing' ? <div className="source-alert"><strong>Canonical 尚未初始化</strong><p>下面标记为「Agent 原生」的内容来自本机客户端文件，尚未纳入统一托管。</p></div> : <p className="panel-note">全局配置优先进入每个 Agent，再叠加项目 Profile。任何 Agent 不支持的能力都会在应用前显示。</p>}<div className="source-file"><span>Canonical source</span><code>{snapshot.global.canonicalPath}</code></div></aside>
            </div>
        </div>
    );
}
