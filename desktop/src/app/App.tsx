import { useEffect, useMemo, useState } from 'react';
import type { CoreClient } from '../core/client';
import { DemoClient } from '../core/demo-client';
import type { ApplyPreview, WorkspaceSnapshot } from '../core/model';
import { TauriCoreClient } from '../core/tauri-client';
import { navigationItems, type ViewId } from './navigation';
import { ActivityView } from '../views/ActivityView';
import { AgentsView } from '../views/AgentsView';
import { GlobalView } from '../views/GlobalView';
import { OverviewView } from '../views/OverviewView';
import { ProjectsView } from '../views/ProjectsView';

// Real Core is the safe default. Demo data is opt-in for design review only.
const requestedMode = import.meta.env.VITE_CORE_MODE ?? 'real';

export function App() {
    const [view, setView] = useState<ViewId>('overview');
    const [snapshot, setSnapshot] = useState<WorkspaceSnapshot | null>(null);
    const [preview, setPreview] = useState<ApplyPreview | null>(null);
    const [error, setError] = useState<string | null>(null);
    const [isLoading, setIsLoading] = useState(true);
    const client = useMemo<CoreClient>(() => requestedMode === 'demo' ? new DemoClient() : new TauriCoreClient(), []);
    const isDemo = requestedMode === 'demo';
    const activeNav = navigationItems.find((item) => item.id === view) ?? navigationItems[0];

    useEffect(() => {
        let cancelled = false;
        setIsLoading(true);
        client.getSnapshot().then((nextSnapshot) => {
            if (!cancelled) setSnapshot(nextSnapshot);
        }).catch((cause: unknown) => {
            if (!cancelled) setError(cause instanceof Error ? cause.message : String(cause));
        }).finally(() => {
            if (!cancelled) setIsLoading(false);
        });
        return () => { cancelled = true; };
    }, [client]);

    async function handlePreview() {
        try {
            setPreview(await client.previewApply());
            setError(null);
        } catch (cause: unknown) {
            setError(cause instanceof Error ? cause.message : String(cause));
        }
    }

    return (
        <div className="app-shell">
            <aside className="sidebar">
                <div className="brand"><span className="brand-mark" aria-hidden="true">a</span><div><strong>agent-assistant</strong><small>治理工作台</small></div></div>
                <nav className="primary-nav" aria-label="主导航">
                    <span className="nav-label">工作区</span>
                    {navigationItems.map((item) => <button key={item.id} className={view === item.id ? 'nav-item is-active' : 'nav-item'} type="button" aria-current={view === item.id ? 'page' : undefined} onClick={() => { setView(item.id); setPreview(null); }}><span className="nav-icon" aria-hidden="true">{item.icon}</span><span>{item.label}</span>{item.id === 'overview' && snapshot?.metrics.attention ? <b>{snapshot.metrics.attention}</b> : null}</button>)}
                </nav>
                <div className="sidebar-footer"><div className="connection-dot"><span aria-hidden="true" /> Core {isDemo ? '演示模式' : 'IPC 模式'}</div><small>v0.1.0 · 本地工作区</small></div>
            </aside>

            <main className="main-content">
                <header className="topbar"><div className="breadcrumbs"><span>工作区</span><span aria-hidden="true">/</span><strong>{activeNav.label}</strong></div><div className="topbar-actions"><button className="icon-button" type="button" aria-label="搜索" title="搜索">⌕</button><button className="avatar-button" type="button" aria-label="打开账户菜单" title="账户菜单">K</button></div></header>
                {isDemo && <div className="demo-banner" role="status"><span aria-hidden="true">◈</span><span><strong>演示模式</strong> 当前数据来自本地 Demo，连接 Go Agentsync Core 后才会读取真实配置。</span><button type="button" onClick={() => setError('请先启动 Go Agentsync Core sidecar，再将 VITE_CORE_MODE 设置为 real。')}>了解真实模式</button></div>}
                {!isDemo && snapshot && <div className="core-banner" role="status"><span aria-hidden="true">●</span><span><strong>已连接本机 Core</strong> 数据来自 `~/.agentsync`、项目 Profile 和已检测的 Agent 原生配置。</span></div>}
                {error && <div className="error-banner" role="alert"><span aria-hidden="true">!</span><span>{error}</span><button className="icon-button" type="button" aria-label="关闭错误" title="关闭错误" onClick={() => setError(null)}>×</button></div>}
                <div className="content-area">
                    {isLoading && <div className="loading-state" role="status"><span className="loading-spinner" aria-hidden="true" />正在读取工作区…</div>}
                    {!isLoading && snapshot && <ViewContent view={view} snapshot={snapshot} preview={preview} onPreview={handlePreview} />}
                    {!isLoading && !snapshot && !error && <div className="empty-state page-empty"><span className="empty-icon" aria-hidden="true">◌</span><strong>暂无工作区数据</strong><p>连接 Core 后会在这里显示配置。</p></div>}
                </div>
            </main>
        </div>
    );
}

function ViewContent({ view, snapshot, preview, onPreview }: { view: ViewId; snapshot: WorkspaceSnapshot; preview: ApplyPreview | null; onPreview: () => void }) {
    switch (view) {
        case 'global': return <GlobalView snapshot={snapshot} />;
        case 'agents': return <AgentsView snapshot={snapshot} />;
        case 'projects': return <ProjectsView snapshot={snapshot} />;
        case 'activity': return <ActivityView snapshot={snapshot} />;
        default: return <OverviewView snapshot={snapshot} preview={preview} onPreview={onPreview} />;
    }
}
