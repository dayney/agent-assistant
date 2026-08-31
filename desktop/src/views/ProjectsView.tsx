import { useState } from 'react';
import type { ProjectSummary, WorkspaceSnapshot } from '../core/model';
import { ProvenanceChain } from '../components/ProvenanceChain';
import { StatusBadge } from '../components/StatusBadge';

export function ProjectsView({ snapshot }: { snapshot: WorkspaceSnapshot }) {
    const [selectedId, setSelectedId] = useState(snapshot.projects[0]?.id ?? '');
    const selected = snapshot.projects.find((project) => project.id === selectedId) ?? snapshot.projects[0];

    return (
        <div className="view-stack">
            <section className="page-heading page-heading-row"><div><p className="eyebrow">Project Profiles</p><h1>项目</h1><p className="lede">项目 Profile 只补充项目特性，不覆盖公共治理原则。</p></div><button className="button button-primary" type="button"><span aria-hidden="true">＋</span> 添加项目</button></section>
            <div className="content-grid project-grid">
                <section className="panel panel-main project-list-panel"><div className="panel-header"><div><p className="eyebrow">已纳管</p><h2>{snapshot.projects.length} 个项目</h2></div></div><div className="project-list">{snapshot.projects.map((project) => <ProjectRow key={project.id} project={project} active={project.id === selected?.id} onClick={() => setSelectedId(project.id)} />)}</div></section>
                {selected && <aside className="panel panel-side project-detail"><div className="detail-heading"><div className="project-icon" aria-hidden="true">{selected.name.slice(0, 1)}</div><div><p className="eyebrow">当前项目</p><h2>{selected.name}</h2><code>{selected.path}</code></div></div><StatusBadge value={selected.syncState} /><div className="detail-section"><span className="detail-label">Profile</span><strong>{selected.profile}</strong></div><div className="detail-section"><span className="detail-label">技术栈</span><div className="tag-list">{selected.stack.map((item) => <span className="tag" key={item}>{item}</span>)}</div></div><div className="detail-section"><span className="detail-label">绑定 Agent</span><div className="agent-chip-list">{selected.agents.map((agent) => <span className="agent-chip" key={agent}><span className="agent-avatar mini" aria-hidden="true">{agent.slice(0, 1)}</span>{agent}</span>)}</div></div><div className="detail-section detail-source"><span className="detail-label">生效路径</span><ProvenanceChain /><p className="panel-note">最后更新 {selected.updatedAt}</p></div><button className="button button-primary button-wide" type="button">打开项目 Profile <span aria-hidden="true">→</span></button></aside>}
            </div>
        </div>
    );
}

function ProjectRow({ project, active, onClick }: { project: ProjectSummary; active: boolean; onClick: () => void }) {
    return <button className={active ? 'project-row is-active' : 'project-row'} type="button" onClick={onClick}><span className="project-icon" aria-hidden="true">{project.name.slice(0, 1)}</span><span className="project-row-copy"><strong>{project.name}</strong><small>{project.path}</small></span><StatusBadge value={project.syncState} /><span className="row-chevron" aria-hidden="true">›</span></button>;
}
