import { useMemo, useState } from 'react';
import type { CoreClient } from '../core/client';
import type { ProjectRuleImport, ProjectSummary, RuleProposal, WorkspaceSnapshot } from '../core/model';
import { toggleAgentSelection } from '../core/rule-state';
import { ProvenanceChain } from '../components/ProvenanceChain';
import { StatusBadge } from '../components/StatusBadge';

interface ProjectsViewProps {
    snapshot: WorkspaceSnapshot;
    client: CoreClient;
    onSnapshotRefresh: () => Promise<void>;
}

export function ProjectsView({ snapshot, client, onSnapshotRefresh }: ProjectsViewProps) {
    const [selectedId, setSelectedId] = useState(snapshot.projects[0]?.id ?? '');
    const [projectPath, setProjectPath] = useState('');
    const [imported, setImported] = useState<ProjectRuleImport | null>(null);
    const [proposal, setProposal] = useState<RuleProposal | null>(null);
    const [proposalBody, setProposalBody] = useState('');
    const [selectedAgents, setSelectedAgents] = useState<string[]>([]);
    const [busy, setBusy] = useState(false);
    const [message, setMessage] = useState('');
    const [error, setError] = useState('');
    const selected = snapshot.projects.find((item) => item.id === selectedId) ?? snapshot.projects[0];
    const detectedAgents = useMemo(() => imported ? [...new Set(imported.sources.flatMap((source) => source.agents))].sort() : [], [imported]);

    async function run(action: () => Promise<void>) {
        setBusy(true);
        setError('');
        setMessage('');
        try {
            await action();
        } catch (cause) {
            setError(cause instanceof Error ? cause.message : String(cause));
        } finally {
            setBusy(false);
        }
    }

    function handleImport() {
        void run(async () => {
            const result = await client.importProject(projectPath.trim());
            setImported(result);
            setProjectPath(result.path);
            const preferred = snapshot.agents.map((agent) => agent.id).filter((agent) => result.sources.some((source) => source.agents.includes(agent)));
            setSelectedAgents(preferred.length ? preferred : result.sources[0]?.agents.slice(0, 1) ?? []);
            setProposal(null);
            setProposalBody('');
            setMessage(`已登记项目并发现 ${result.sources.length} 个原生 Rule 文件。`);
            await onSnapshotRefresh();
        });
    }

    function handleAnalyze() {
        if (!imported) return;
        void run(async () => {
            const result = await client.analyzeProjectRules(imported.path);
            setProposal(result);
            setProposalBody(result.body);
            setMessage(result.analyzer === 'codex-cli' ? 'Codex 已生成候选，请人工审核。' : '原生 Rule 内容一致，已生成确定性候选。');
        });
    }

    function handleSaveProposal() {
        if (!imported || !proposalBody.trim()) return;
        void run(async () => {
            if (selectedAgents.length === 0) throw new Error('至少选择一个项目目标 Agent。');
            await client.saveRule({ scope: 'project', projectPath: imported.path, agents: selectedAgents, body: proposalBody });
            setMessage('项目母版已保存。请到“全局配置 / 规则”预览并同步。');
            await onSnapshotRefresh();
        });
    }

    return (
        <div className="view-stack">
            <section className="page-heading page-heading-row"><div><p className="eyebrow">Project Profiles</p><h1>项目</h1><p className="lede">手动导入项目，审查原生 Rule，再建立项目母版。</p></div></section>
            <section className="panel project-import-panel">
                <div className="project-import-form"><label><span>本机项目路径</span><input value={projectPath} onChange={(event) => setProjectPath(event.target.value)} placeholder="/Users/name/git/work/project" disabled={busy} /></label><button className="button button-primary" type="button" onClick={handleImport} disabled={busy || !projectPath.trim()}>导入项目</button></div>
                {error && <div className="inline-alert is-error" role="alert">{error}</div>}
                {message && <div className="inline-alert" role="status">{message}</div>}
                {imported && <div className="import-analysis">
                    <div className="source-evidence"><div className="section-heading"><div><p className="eyebrow">Native Evidence</p><h2>原生 Rule 来源</h2></div><span className="target-count">{imported.sources.length}</span></div>{imported.sources.length ? imported.sources.map((source) => <div className="evidence-row" key={source.path}><code>{source.path}</code><span>{source.agents.join('、')}</span></div>) : <p className="panel-note">未发现已知 Agent 的项目 Rule 文件。</p>}</div>
                    <div className="proposal-pane"><div className="section-heading"><div><p className="eyebrow">Reviewed Proposal</p><h2>项目母版候选</h2></div>{imported.sources.length > 0 && <button className="button button-secondary" type="button" onClick={handleAnalyze} disabled={busy}>{imported.needsAnalysis ? '使用本机 Codex 分析' : '生成一致候选'}</button>}</div>{proposal ? <><textarea className="proposal-textarea" value={proposalBody} onChange={(event) => setProposalBody(event.target.value)} spellCheck={false} aria-label="项目母版候选" /><div className="proposal-meta"><span>分析器：{proposal.analyzer}</span>{proposal.notes.map((note) => <p key={note}>{note}</p>)}</div><div className="agent-selector compact">{detectedAgents.map((agent) => <label key={agent}><input type="checkbox" checked={selectedAgents.includes(agent)} onChange={(event) => setSelectedAgents((current) => toggleAgentSelection(current, agent, event.target.checked))} /><span>{agent}</span></label>)}</div><div className="rule-actions"><button className="button button-primary" type="button" onClick={handleSaveProposal} disabled={busy}>保存为项目母版</button></div></> : <div className="proposal-empty"><strong>尚未生成候选</strong><p>分析不会写入项目；保存前可以完整修改候选内容。</p></div>}</div>
                </div>}
            </section>
            <div className="content-grid project-grid">
                <section className="panel panel-main project-list-panel"><div className="panel-header"><div><p className="eyebrow">已纳管</p><h2>{snapshot.projects.length} 个项目</h2></div></div><div className="project-list">{snapshot.projects.map((item) => <ProjectRow key={item.path} project={item} active={item.id === selected?.id} onClick={() => setSelectedId(item.id)} />)}</div></section>
                {selected && <aside className="panel panel-side project-detail"><div className="detail-heading"><div className="project-icon" aria-hidden="true">{selected.name.slice(0, 1)}</div><div><p className="eyebrow">当前项目</p><h2>{selected.name}</h2><code>{selected.path}</code></div></div><StatusBadge value={selected.syncState} /><div className="detail-section"><span className="detail-label">Profile</span><strong>{selected.profile}</strong></div><div className="detail-section"><span className="detail-label">技术栈</span><div className="tag-list">{selected.stack.map((item) => <span className="tag" key={item}>{item}</span>)}</div></div><div className="detail-section"><span className="detail-label">绑定 Agent</span><div className="agent-chip-list">{selected.agents.map((agent) => <span className="agent-chip" key={agent}><span className="agent-avatar mini" aria-hidden="true">{agent.slice(0, 1)}</span>{agent}</span>)}</div></div><div className="detail-section detail-source"><span className="detail-label">生效路径</span><ProvenanceChain /><p className="panel-note">最后更新 {selected.updatedAt}</p></div></aside>}
            </div>
        </div>
    );
}

function ProjectRow({ project, active, onClick }: { project: ProjectSummary; active: boolean; onClick: () => void }) {
    return <button className={active ? 'project-row is-active' : 'project-row'} type="button" onClick={onClick}><span className="project-icon" aria-hidden="true">{project.name.slice(0, 1)}</span><span className="project-row-copy"><strong>{project.name}</strong><small>{project.path}</small></span><StatusBadge value={project.syncState} /><span className="row-chevron" aria-hidden="true">›</span></button>;
}
