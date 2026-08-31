import { useEffect, useMemo, useState } from 'react';
import type { CoreClient } from '../core/client';
import type { RuleRequest, RuleTarget, RuleWorkspace, WorkspaceSnapshot } from '../core/model';
import { toggleAgentSelection } from '../core/rule-state';
import { ProvenanceChain } from '../components/ProvenanceChain';

type GlobalTab = 'rules' | 'mcp' | 'skills' | 'hooks' | 'subagents';

const tabs: Array<{ id: GlobalTab; label: string }> = [
    { id: 'rules', label: '规则' }, { id: 'mcp', label: 'MCP' }, { id: 'skills', label: 'Skills' }, { id: 'hooks', label: 'Hooks' }, { id: 'subagents', label: 'Subagents' },
];

interface GlobalViewProps {
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
                        {tab === 'rules' && <RuleEditor snapshot={snapshot} client={client} onSnapshotRefresh={onSnapshotRefresh} />}
                        {tab === 'mcp' && <><div className="panel-header"><div><p className="eyebrow">{count} 个条目</p><h2>MCP</h2></div></div><div className="mcp-list">{snapshot.global.mcp.map((server) => <div className="mcp-row" key={server.id}><div className="mcp-mark" aria-hidden="true">⌁</div><div className="mcp-copy"><strong>{server.name}</strong><p>{server.description}</p><code>{server.endpoint}</code></div><div className="mcp-meta"><span className="scope-chip">{server.source === 'global' ? '全局' : server.source === 'agent' ? 'Agent 原生' : '项目覆盖'}</span><small>{server.secretRefs.length ? `${server.secretRefs.length} 个引用` : '敏感值已隐藏'}</small></div></div>)}</div></>}
                        {tab !== 'rules' && tab !== 'mcp' && <div className="empty-state"><span className="empty-icon" aria-hidden="true">◌</span><strong>{tabs.find((item) => item.id === tab)?.label} 尚未进入当前迭代</strong><p>Rule 完成并独立提交后，才会开始下一类配置。</p></div>}
                    </div>
                </section>
                {tab !== 'rules' && <aside className="panel panel-side source-panel"><p className="eyebrow">基线来源</p><h2>公共配置</h2><ProvenanceChain active="global" /><p className="panel-note">全局配置先进入每个 Agent，再叠加项目 Profile。任何不支持的能力都会在应用前显示。</p><div className="source-file"><span>Canonical source</span><code>{snapshot.global.canonicalPath}</code></div></aside>}
            </div>
        </div>
    );
}

function RuleEditor({ snapshot, client, onSnapshotRefresh }: GlobalViewProps) {
    const [scope, setScope] = useState<'global' | 'project'>('global');
    const [projectPath, setProjectPath] = useState(snapshot.projects[0]?.path ?? '');
    const [workspace, setWorkspace] = useState<RuleWorkspace | null>(null);
    const [body, setBody] = useState('');
    const [selectedAgents, setSelectedAgents] = useState<string[]>([]);
    const [conflict, setConflict] = useState<RuleTarget | null>(null);
    const [message, setMessage] = useState('');
    const [error, setError] = useState('');
    const [busy, setBusy] = useState(false);
    const request = useMemo<RuleRequest>(() => ({ scope, projectPath: scope === 'project' ? projectPath : undefined }), [scope, projectPath]);

    useEffect(() => {
        let cancelled = false;
        setBusy(true);
        setError('');
        client.getRules(request).then((next) => {
            if (cancelled) return;
            setWorkspace(next);
            setBody(next.document.body);
            setSelectedAgents(next.document.agents);
        }).catch((cause: unknown) => {
            if (!cancelled) setError(errorText(cause));
        }).finally(() => {
            if (!cancelled) setBusy(false);
        });
        return () => { cancelled = true; };
    }, [client, request]);

    const dirty = workspace !== null && body !== workspace.document.body;
    const availableAgents = workspace?.availableAgents ?? [];

    async function saveMother(): Promise<void> {
        if (!body.trim()) throw new Error('母版 Rule 不能为空。');
        if (selectedAgents.length === 0) throw new Error('至少选择一个目标 Agent。');
        await client.saveRule({ ...request, agents: selectedAgents, body });
        const next = await client.getRules({ ...request, agents: selectedAgents });
        setWorkspace(next);
        setBody(next.document.body);
        await onSnapshotRefresh();
    }

    async function run(action: () => Promise<void>) {
        setBusy(true);
        setError('');
        setMessage('');
        try {
            await action();
        } catch (cause) {
            setError(errorText(cause));
        } finally {
            setBusy(false);
        }
    }

    function handleSave() {
        void run(async () => {
            await saveMother();
            setMessage('母版 Rule 已保存。');
        });
    }

    function handlePreview() {
        void run(async () => {
            if (selectedAgents.length === 0) throw new Error('至少选择一个目标 Agent。');
            if (body.trim()) await saveMother();
            const next = await client.getRules({ ...request, agents: selectedAgents });
            setWorkspace(next);
            setConflict(next.targets.find((target) => target.blocked) ?? null);
            setMessage(next.blocked ? '发现原生修改，已停止同步。' : '预览完成，可以安全同步。');
        });
    }

    function handleSync() {
        void run(async () => {
            await saveMother();
            const result = await client.syncRules({ ...request, agents: selectedAgents });
            setWorkspace(result.preview);
            if (!result.applied) {
                setConflict(result.preview.targets.find((target) => target.blocked) ?? null);
                setMessage('同步已停止，请先处理原生文件差异。');
                return;
            }
            setMessage('Rule 已同步到所选 Agent。');
        });
    }

    function handleImportNative() {
        if (!conflict) return;
        void run(async () => {
            const agent = conflict.agent;
            const document = await client.importNativeRule({ ...request, agents: selectedAgents, agent });
            setBody(document.body);
            const next = await client.getRules({ ...request, agents: selectedAgents });
            setWorkspace(next);
            setConflict(null);
            setMessage(`已将 ${agent} 的原生 Rule 导入母版，请审核后再同步。`);
        });
    }

    function handleBackupOverwrite() {
        void run(async () => {
            const result = await client.syncRules({ ...request, agents: selectedAgents, resolution: 'backup-overwrite' });
            setWorkspace(result.preview);
            setConflict(null);
            setMessage(`已备份并覆盖 ${result.backups.length} 个原生文件。`);
        });
    }

    return (
        <div className="rule-workbench">
            <div className="rule-toolbar">
                <div className="segmented" aria-label="Rule 范围">
                    <button type="button" className={scope === 'global' ? 'is-active' : ''} onClick={() => setScope('global')}>全局母版</button>
                    <button type="button" className={scope === 'project' ? 'is-active' : ''} disabled={snapshot.projects.length === 0} onClick={() => setScope('project')}>项目母版</button>
                </div>
                {scope === 'project' && <label className="field-inline"><span>项目</span><select value={projectPath} onChange={(event) => setProjectPath(event.target.value)}>{snapshot.projects.map((project) => <option value={project.path} key={project.path}>{project.name}</option>)}</select></label>}
                <code className="canonical-path">{workspace?.document.canonicalPath ?? '读取中…'}</code>
            </div>

            {error && <div className="inline-alert is-error" role="alert">{error}</div>}
            {message && <div className="inline-alert" role="status">{message}</div>}

            <div className="rule-editor-grid">
                <section className="rule-editor-pane">
                    <div className="section-heading"><div><p className="eyebrow">Mother Rule</p><h2>母版 Rule</h2></div><span className={dirty ? 'edit-state is-dirty' : 'edit-state'}>{dirty ? '未保存' : '已保存'}</span></div>
                    <textarea className="rule-textarea" value={body} onChange={(event) => setBody(event.target.value)} spellCheck={false} aria-label="母版 Rule Markdown" disabled={busy} />
                    {workspace?.document.fragments.length ? <p className="fragment-note">保留 fragments：{workspace.document.fragments.join('、')}</p> : null}
                    <div className="rule-actions"><button className="button button-secondary" type="button" disabled={busy} onClick={handleSave}>保存母版</button><button className="button button-secondary" type="button" disabled={busy} onClick={handlePreview}>预览同步</button><button className="button button-primary" type="button" disabled={busy} onClick={handleSync}>同步到 Agent</button></div>
                </section>

                <aside className="rule-target-pane">
                    <div className="section-heading"><div><p className="eyebrow">Targets</p><h2>目标 Agent</h2></div><span className="target-count">{selectedAgents.length}</span></div>
                    <div className="agent-selector">{availableAgents.map((agent) => <label key={agent}><input type="checkbox" checked={selectedAgents.includes(agent)} onChange={(event) => setSelectedAgents((current) => toggleAgentSelection(current, agent, event.target.checked))} /><span>{agent}</span></label>)}</div>
                    <div className="target-preview">{workspace?.targets.map((target) => <button type="button" className={target.blocked ? 'target-row is-blocked' : 'target-row'} key={target.agent} onClick={() => target.blocked && setConflict(target)}><span className="target-agent">{target.agent}</span><span className={`target-status status-${target.status}`}>{statusLabel(target.status)}</span><code>{target.path || target.reason || '无目标路径'}</code></button>)}</div>
                </aside>
            </div>

            {conflict && <div className="modal-backdrop" role="presentation"><section className="conflict-dialog" role="dialog" aria-modal="true" aria-labelledby="conflict-title"><div className="dialog-heading"><div><p className="eyebrow">Sync Blocked</p><h2 id="conflict-title">{conflict.agent} 存在未同步修改</h2></div><button className="icon-button" type="button" aria-label="关闭" onClick={() => setConflict(null)}>×</button></div><p className="dialog-copy">同步已经停止。请选择把原生内容导入母版，或先备份原生文件再以母版覆盖。</p><code className="dialog-path">{conflict.path}</code><pre className="diff-view">{conflict.diff || conflict.reason}</pre><div className="dialog-actions"><button className="button button-secondary" type="button" onClick={() => setConflict(null)}>取消</button>{conflict.supported && <button className="button button-secondary" type="button" onClick={handleImportNative}>导入到母版</button>}{conflict.supported && <button className="button button-danger" type="button" onClick={handleBackupOverwrite}>备份后覆盖</button>}</div></section></div>}
        </div>
    );
}

function statusLabel(status: string): string {
    const labels: Record<string, string> = { clean: '已同步', pending: '待更新', new: '新建', converged: '已收敛', drift: '原生修改', conflict: '双向冲突', 'foreign-collision': '未托管文件', 'native-only': '仅原生存在', unsupported: '不支持', empty: '无母版' };
    return labels[status] ?? status;
}

function errorText(cause: unknown): string {
    return cause instanceof Error ? cause.message : String(cause);
}
