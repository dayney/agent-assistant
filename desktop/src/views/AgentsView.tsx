import type {
  AgentSummary,
  ManagedComponent,
  WorkspaceSnapshot,
} from "../core/model";
import { StatusBadge } from "../components/StatusBadge";

const components: Array<{ id: ManagedComponent; label: string }> = [
  { id: "rules", label: "规则" },
  { id: "skills", label: "Skills" },
  { id: "mcp", label: "MCP" },
  { id: "commands", label: "Commands" },
  { id: "hooks", label: "Hooks" },
  { id: "subagents", label: "Subagents" },
  { id: "workflows", label: "Workflows" },
];

const productNames: Record<string, string> = {
  antigravity: "Antigravity",
  "antigravity-ide": "Antigravity IDE",
  "antigravity-cli": "Antigravity CLI",
  claude: "Claude",
  codex: "Codex",
  cursor: "Cursor",
  gemini: "Gemini CLI",
};

const priorityProducts = ["cursor", "antigravity", "antigravity-ide", "antigravity-cli"];

function productId(agent: string): string {
  return agent === "cursor-account" || agent === "cursor-local" ? "cursor" : agent;
}

export function AgentsView({ snapshot, onRefresh }: { snapshot: WorkspaceSnapshot; onRefresh: () => Promise<void> }) {
  const [refreshing, setRefreshing] = useState(false);
  const [refreshError, setRefreshError] = useState<string | null>(null);

  async function refresh() {
    setRefreshing(true);
    setRefreshError(null);
    try {
      await onRefresh();
    } catch (error) {
      setRefreshError(error instanceof Error ? error.message : String(error));
    } finally {
      setRefreshing(false);
    }
  }

  const sources = [
    ...snapshot.global.nativeRules,
    ...snapshot.global.nativeMcp,
    ...snapshot.global.nativeSkills,
    ...snapshot.global.nativeWorkflows,
    ...snapshot.global.nativeHooks,
    ...snapshot.global.nativeSubagents,
  ];
  const products = [...new Set(sources.map((source) => productId(source.agent)).filter((id) => id !== "shared"))]
    .sort((left, right) => {
      const leftPriority = priorityProducts.indexOf(left);
      const rightPriority = priorityProducts.indexOf(right);
      if (leftPriority !== -1 || rightPriority !== -1) {
        return (leftPriority === -1 ? priorityProducts.length : leftPriority)
          - (rightPriority === -1 ? priorityProducts.length : rightPriority);
      }
      return left.localeCompare(right);
    });

  return (
    <div className="view-stack">
      <section className="page-heading">
        <p className="eyebrow">Agent Adapters</p>
        <h1>Agent</h1>
        <p className="lede">查看已配置适配器的能力，并核对本机产品配置来源。</p>
      </section>
      <section className="agent-source-section" aria-labelledby="native-products-title">
        <div className="section-heading">
          <div>
            <p className="eyebrow">Native Products</p>
            <h2 id="native-products-title">本机产品来源</h2>
          </div>
        </div>
        <p className="native-source-note">配置来源，不代表已安装或已同步。共享文件在各产品下分别标注，详情见左侧各配置页。</p>
        <div className="agent-source-list">
          {products.map((id) => {
            const productSources = sources.filter((source) => productId(source.agent) === id);
            const found = new Set(productSources.filter((source) =>
              source.path && ("items" in source
                ? source.items.some((item) => item.state === "present")
                : "serverCount" in source
                  ? source.state === "parsed" && source.serverCount > 0
                  : source.state === "present"),
            ).map((source) => source.path)).size;
            const pending = new Set(productSources.filter((source) =>
              source.state === "unverified" && source.path,
            ).map((source) => source.path)).size;
            const status = [
              found ? `发现 ${found} 处配置来源` : "",
              pending ? `${pending} 处待核实` : "",
            ].filter(Boolean).join(" · ") || "未发现已核实配置";
            return (
              <div className="agent-source-row" key={id}>
                <strong>{productNames[id] ?? id}</strong>
                <span>{status}</span>
              </div>
            );
          })}
        </div>
      </section>
      <section className="panel matrix-panel">
        <div className="panel-header">
          <div>
            <p className="eyebrow">Capability Matrix</p>
            <h2>{snapshot.agents.length} 个已配置适配器</h2>
          </div>
          <button className="button button-quiet" type="button" onClick={() => void refresh()} disabled={refreshing}>
            <RefreshCw size={16} aria-hidden="true" /> {refreshing ? "正在检测" : "重新检测"}
          </button>
        </div>
        {refreshError && <p className="agent-refresh-error" role="alert">重新检测失败：{refreshError}</p>}
        {snapshot.agents.length === 0 && <p className="agent-matrix-empty">未配置适配器；上方显示的是本机已核对的产品配置来源。</p>}
        <div className="matrix-wrap">
          <table className="capability-table">
            <thead>
              <tr>
                <th>Agent</th>
                <th>状态</th>
                {components.map((component) => (
                  <th key={component.id}>{component.label}</th>
                ))}
                <th>最近应用</th>
              </tr>
            </thead>
            <tbody>
              {snapshot.agents.map((agent) => (
                <AgentRow agent={agent} key={agent.id} />
              ))}
            </tbody>
          </table>
        </div>
        <div className="matrix-legend">
          <span>
            <i className="legend-dot dot-full" />
            完整
          </span>
          <span>
            <i className="legend-dot dot-partial" />
            部分支持
          </span>
          <span>
            <i className="legend-dot dot-unsupported" />
            不支持
          </span>
          <span>
            <i className="legend-dot dot-unknown" />
            待核实
          </span>
        </div>
      </section>
    </div>
  );
}

function AgentRow({ agent }: { agent: AgentSummary }) {
  return (
    <tr>
      <td>
        <div className="table-agent">
          <span className="agent-avatar" aria-hidden="true">
            {agent.name.slice(0, 1)}
          </span>
          <div>
            <strong>{agent.name}</strong>
            <small>
              {agent.vendor} · {agent.version}
            </small>
          </div>
        </div>
      </td>
      <td>
        <StatusBadge value={agent.syncState} />
      </td>
      {components.map((component) => (
        <td key={component.id}>
          <StatusBadge value={agent.capabilities.components[component.id]} />
        </td>
      ))}
      <td className="last-applied">{agent.lastApplied}</td>
    </tr>
  );
}
import { useState } from "react";
import { RefreshCw } from "lucide-react";
