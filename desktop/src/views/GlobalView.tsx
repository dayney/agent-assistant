import { useState } from 'react';
import type { CoreClient } from '../core/client';
import type { WorkspaceSnapshot } from '../core/model';
import { ProvenanceChain } from '../components/ProvenanceChain';
import { RulesView } from './RulesView';

type GlobalTab = 'rules' | 'mcp' | 'skills' | 'hooks' | 'subagents';

const tabs: Array<{ id: GlobalTab; label: string }> = [
    { id: 'rules', label: '规则' }, { id: 'mcp', label: 'MCP' }, { id: 'skills', label: 'Skills' }, { id: 'hooks', label: 'Hooks' }, { id: 'subagents', label: 'Subagents' },
];

export interface GlobalViewProps {
    snapshot: WorkspaceSnapshot;
    client: CoreClient;
    onSnapshotRefresh: () => Promise<void>;
}

export function GlobalView({ snapshot, client, onSnapshotRefresh }: GlobalViewProps) {
    const [tab, setTab] = useState<GlobalTab>('rules');
    const count = tab === 'rules' ? snapshot.global.rules.length : tab === 'mcp' ? snapshot.global.mcp.length : snapshot.global[tab];

    return (
        <div className="view-stack">
            <section className="page-heading"><p className="eyebrow">Global Scope</p><h1>全局配置</h1><p className="lede">母版是唯一编辑源，各 Agent 文件由确定性适配器生成。</p></section>
            <div className={tab === 'rules' ? 'content-grid rule-layout' : 'content-grid global-grid'}>
                <section className="panel panel-main">
                    <div className="tab-list" role="tablist" aria-label="全局组件类型">
                        {tabs.map((item) => <button key={item.id} className={tab === item.id ? 'tab is-active' : 'tab'} type="button" role="tab" aria-selected={tab === item.id} onClick={() => setTab(item.id)}>{item.label}<span>{item.id === 'rules' ? snapshot.global.rules.length : item.id === 'mcp' ? snapshot.global.mcp.length : snapshot.global[item.id]}</span></button>)}
                    </div>
                    <div className="tab-content">
                        {tab === 'rules' && <RulesView snapshot={snapshot} client={client} onSnapshotRefresh={onSnapshotRefresh} />}
                        {tab === 'mcp' && <><div className="panel-header"><div><p className="eyebrow">{count} 个条目</p><h2>MCP</h2></div></div><div className="mcp-list">{snapshot.global.mcp.map((server) => <div className="mcp-row" key={server.id}><div className="mcp-mark" aria-hidden="true">⌁</div><div className="mcp-copy"><strong>{server.name}</strong><p>{server.description}</p><code>{server.endpoint}</code></div><div className="mcp-meta"><span className="scope-chip">{server.source === 'global' ? '全局' : server.source === 'agent' ? 'Agent 原生' : '项目覆盖'}</span><small>{server.secretRefs.length ? `${server.secretRefs.length} 个引用` : '敏感值已隐藏'}</small></div></div>)}</div></>}
                        {tab !== 'rules' && tab !== 'mcp' && <div className="empty-state"><span className="empty-icon" aria-hidden="true">◌</span><strong>{tabs.find((item) => item.id === tab)?.label} 尚未进入当前迭代</strong><p>Rule 完成并独立提交后，才会开始下一类配置。</p></div>}
                    </div>
                </section>
                {tab !== 'rules' && <aside className="panel panel-side source-panel"><p className="eyebrow">基线来源</p><h2>公共配置</h2><ProvenanceChain active="global" /><p className="panel-note">全局配置先进入每个 Agent，再叠加项目 Profile。任何不支持的能力都会在应用前显示。</p><div className="source-file"><span>Canonical source</span><code>{snapshot.global.canonicalPath}</code></div></aside>}
            </div>
        </div>
    );
}
