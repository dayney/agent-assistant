import type { WorkspaceSnapshot } from '../core/model';

export function ActivityView({ snapshot }: { snapshot: WorkspaceSnapshot }) {
    return <div className="view-stack"><section className="page-heading"><p className="eyebrow">Audit Trail</p><h1>活动记录</h1><p className="lede">每次应用、能力差异和 Profile 更新都留在这里。</p></section><section className="panel activity-panel"><div className="panel-header"><div><p className="eyebrow">最近事件</p><h2>变更时间线</h2></div><button className="button button-quiet" type="button"><span aria-hidden="true">⇩</span> 导出记录</button></div><div className="activity-list">{snapshot.activity.map((item) => <article className="activity-row" key={item.id}><span className={`activity-marker marker-${item.tone}`} aria-hidden="true" /><div><strong>{item.title}</strong><p>{item.detail}</p></div><time>{item.timestamp}</time></article>)}</div></section></div>;
}
