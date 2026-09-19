import type { ApplyPreview, WorkspaceSnapshot } from "../core/model";
import { ProvenanceChain } from "../components/ProvenanceChain";
import { StatusBadge } from "../components/StatusBadge";

interface OverviewViewProps {
  snapshot: WorkspaceSnapshot;
  preview: ApplyPreview | null;
  onPreview: () => void;
}

export function OverviewView({
  snapshot,
  preview,
  onPreview,
}: OverviewViewProps) {
  const attentionAgents = snapshot.agents.filter(
    (agent) => agent.health !== "good",
  );

  return (
    <div className="view-stack">
      <section className="welcome-row">
        <div>
          <p className="eyebrow">工作区状态</p>
          <h1>治理工作台</h1>
          <p className="lede">
            从一个来源管理全局规则、Agent 适配和项目 Profile。
          </p>
        </div>
        <button
          className="button button-primary"
          type="button"
          onClick={onPreview}
        >
          <span aria-hidden="true">↗</span> 预览应用
        </button>
      </section>

      <div className="metric-grid" aria-label="工作区指标">
        <Metric
          label="已接入 Agent"
          value={snapshot.metrics.agents}
          detail="全部可检测"
          tone="blue"
        />
        <Metric
          label="项目 Profile"
          value={snapshot.metrics.projects}
          detail="均已纳入治理"
          tone="green"
        />
        <Metric
          label="托管组件"
          value={snapshot.metrics.components}
          detail="跨所有作用域"
          tone="purple"
        />
        <Metric
          label="待处理"
          value={snapshot.metrics.attention}
          detail="能力差异或漂移"
          tone="amber"
        />
      </div>

      <div className="content-grid overview-grid">
        <section className="panel panel-main">
          <div className="panel-header">
            <div>
              <p className="eyebrow">运行状态</p>
              <h2>Agent 健康度</h2>
            </div>
            <button
              className="button button-quiet"
              type="button"
              onClick={onPreview}
            >
              查看应用预览 <span aria-hidden="true">→</span>
            </button>
          </div>
          <div className="agent-health-list">
            {snapshot.agents.map((agent) => (
              <div className="agent-health-row" key={agent.id}>
                <div className="agent-avatar" aria-hidden="true">
                  {agent.name.slice(0, 1)}
                </div>
                <div className="agent-health-copy">
                  <strong>{agent.name}</strong>
                  <span>
                    {agent.vendor} · {agent.lastApplied}
                  </span>
                </div>
                <StatusBadge value={agent.health} />
              </div>
            ))}
          </div>
        </section>

        <section className="panel panel-side">
          <div className="panel-header">
            <div>
              <p className="eyebrow">配置来源</p>
              <h2>一条可追溯的链</h2>
            </div>
          </div>
          <ProvenanceChain />
          <p className="panel-note">
            每次应用都会显示配置来自哪里，以及哪些内容因 Agent 能力被降级。
          </p>
          <div className="source-stat">
            <span>全局规则</span>
            <strong>{snapshot.global.rules.length} 条</strong>
          </div>
          <div className="source-stat">
            <span>共享 MCP</span>
            <strong>{snapshot.global.mcp.length} 个</strong>
          </div>
        </section>
      </div>

      {attentionAgents.length > 0 && (
        <section className="notice notice-warning" aria-live="polite">
          <span className="notice-icon" aria-hidden="true">
            !
          </span>
          <div>
            <strong>{attentionAgents.length} 个 Agent 需要处理</strong>
            <p>请查看能力矩阵，确认不支持的组件是否要跳过或单独配置。</p>
          </div>
          <button
            className="button button-quiet"
            type="button"
            onClick={onPreview}
          >
            检查差异 <span aria-hidden="true">→</span>
          </button>
        </section>
      )}

      {preview && (
        <section className="panel preview-panel" aria-live="polite">
          <div className="panel-header">
            <div>
              <p className="eyebrow">应用前检查</p>
              <h2>本次将检查约 {preview.files} 个托管项</h2>
            </div>
            <span className="preview-state">预览模式</span>
          </div>
          <div className="preview-summary">
            <span>
              <strong>{preview.partial}</strong> 部分支持
            </span>
            <span>
              <strong>{preview.unsupported}</strong> 不支持
            </span>
          </div>
          <ul className="warning-list">
            {preview.warnings.map((warning) => (
              <li key={warning}>{warning}</li>
            ))}
          </ul>
        </section>
      )}
    </div>
  );
}

function Metric({
  label,
  value,
  detail,
  tone,
}: {
  label: string;
  value: number;
  detail: string;
  tone: string;
}) {
  return (
    <div className={`metric-card metric-${tone}`}>
      <span>{label}</span>
      <strong>{value}</strong>
      <small>{detail}</small>
    </div>
  );
}
