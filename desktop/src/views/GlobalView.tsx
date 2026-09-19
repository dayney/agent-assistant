import type { CoreClient } from "../core/client";
import type { WorkspaceSnapshot } from "../core/model";
import { ProvenanceChain } from "../components/ProvenanceChain";
import { McpView } from "./McpView";
import { RulesView } from "./RulesView";

type GlobalTab =
  "rules" | "mcp" | "skills" | "workflows" | "hooks" | "subagents";

const tabs: Array<{ id: GlobalTab; label: string }> = [
  { id: "rules", label: "规则" },
  { id: "mcp", label: "MCP" },
  { id: "skills", label: "Skills" },
  { id: "workflows", label: "Workflows" },
  { id: "hooks", label: "Hooks" },
  { id: "subagents", label: "Subagents" },
];

export interface GlobalViewProps {
  view: GlobalTab;
  snapshot: WorkspaceSnapshot;
  client: CoreClient;
  onSnapshotRefresh: () => Promise<void>;
  onRuleDirtyChange?: (dirty: boolean) => void;
  onNavigateToProjects: () => void;
}

export function GlobalView({
  view,
  snapshot,
  client,
  onSnapshotRefresh,
  onRuleDirtyChange,
  onNavigateToProjects,
}: GlobalViewProps) {
  const tab = view;
  const count =
    tab === "rules"
      ? snapshot.global.rules.length
      : tab === "mcp"
        ? snapshot.global.mcp.length
        : snapshot.global[tab];

  return (
    <div className="view-stack">
      <section className="page-heading">
        <p className="eyebrow">Global Scope</p>
        <h1>{tabs.find((item) => item.id === tab)?.label ?? "全局配置"}</h1>
        <p className="lede">
          母版是唯一编辑源，各 Agent 文件由确定性适配器生成。
        </p>
      </section>
      <div
        className={
          tab === "rules"
            ? "content-grid rule-layout"
            : tab === "mcp"
              ? "mcp-page-full"
              : "content-grid global-grid"
        }
      >
        {tab === "mcp" ? (
          <McpView
            snapshot={snapshot}
            client={client}
            onSnapshotRefresh={onSnapshotRefresh}
          />
        ) : (
          <>
            <section className="panel panel-main">
              <div className="tab-content">
                {tab === "rules" && (
                  <RulesView
                    snapshot={snapshot}
                    client={client}
                    onSnapshotRefresh={onSnapshotRefresh}
                    onRuleDirtyChange={onRuleDirtyChange}
                    onNavigateToProjects={onNavigateToProjects}
                  />
                )}
                {tab === "skills" && (
                  <>
                    <div className="panel-header">
                      <div>
                        <p className="eyebrow">母版 {count} 个</p>
                        <h2>Skills</h2>
                      </div>
                    </div>
                    <section className="native-skill-inventory" aria-labelledby="native-skill-title">
                      <div className="section-heading">
                        <div>
                          <p className="eyebrow">Native Sources</p>
                          <h2 id="native-skill-title">本机技能目录</h2>
                        </div>
                        <span className="target-count">
                          {snapshot.global.nativeSkills.reduce(
                            (total, source) => total + source.items.filter((item) => item.state === "present").length,
                            0,
                          )} 个来源条目
                        </span>
                      </div>
                      <p className="native-source-note">
                        {snapshot.mode === "demo" ? "以下为演示示例，不是本机扫描结果。" : "仅检查技能目录与 SKILL.md，不读取正文或导入文件。"}
                        Antigravity、Antigravity IDE 与 CLI 分别列出；共享目录可能在多个来源中出现。
                      </p>
                      <div className="native-skill-list">
                        {snapshot.global.nativeSkills.slice().sort((left, right) =>
                          Number(right.state === "present" || right.state === "partial") -
                          Number(left.state === "present" || left.state === "partial"),
                        ).map((source, index) => (
                          <details className="native-skill-source" key={`${source.agent}:${source.path}:${index}`}>
                            <summary>
                              <strong>{nativeSkillLabel(source.agent)}</strong>
                              <code title={source.path || source.reason}>{source.path || source.reason}</code>
                              <span title={source.reason}>
                                {source.state === "present" || source.state === "partial"
                                  ? `${source.items.filter((item) => item.state === "present").length} 个已发现${source.state === "partial" ? " · 有异常" : ""}`
                                  : source.state === "missing" ? "未找到"
                                  : source.state === "unavailable" ? "无专属目录"
                                  : source.state === "empty" ? "空目录"
                                  : "无法安全读取"}
                              </span>
                            </summary>
                            {source.items.length > 0 && (
                              <div className="native-skill-items">
                                {source.items.map((item, itemIndex) => (
                                  <div key={`${item.path}:${itemIndex}`}>
                                    <span title={item.path}>{item.name}</span>
                                    <small>{item.state === "present" ? "已发现" : item.state === "unsafe" ? "不安全路径" : "待核实"}</small>
                                  </div>
                                ))}
                              </div>
                            )}
                            {source.items.length === 0 && (
                              <p className="native-skill-empty">{source.reason || "没有可预览的技能条目"}</p>
                            )}
                          </details>
                        ))}
                      </div>
                    </section>
                  </>
                )}
                {tab === "workflows" && (
                  <>
                    <div className="panel-header">
                      <div>
                        <p className="eyebrow">母版 {count} 个</p>
                        <h2>Workflows</h2>
                      </div>
                    </div>
                    <section className="native-skill-inventory" aria-labelledby="native-workflow-title">
                      <div className="section-heading">
                        <div>
                          <p className="eyebrow">Native Sources</p>
                          <h2 id="native-workflow-title">本机 Workflow 来源</h2>
                        </div>
                        <span className="target-count">
                          {snapshot.global.nativeWorkflows.reduce((total, source) => total + source.items.filter((item) => item.state === "present").length, 0)} 个文件
                        </span>
                      </div>
                      <p className="native-source-note">
                        {snapshot.mode === "demo" ? "以下为演示示例。" : "只读盘点，尚未导入。"}
                        Antigravity IDE 使用独立的全局 Workflow 目录；其他端没有已核实的独立目录。
                      </p>
                      <div className="native-skill-list">
                        {snapshot.global.nativeWorkflows.map((source) => (
                          <details className="native-skill-source" key={source.agent}>
                            <summary>
                              <strong>{nativeSkillLabel(source.agent)}</strong>
                              <code title={source.path || source.reason}>{source.path || source.reason}</code>
                              <span>{source.state === "present" || source.state === "partial"
                                ? `${source.items.filter((item) => item.state === "present").length} 个已发现${source.state === "partial" ? " · 有异常" : ""}`
                                : source.state === "unavailable" ? "无独立目录"
                                : source.state === "missing" ? "未找到"
                                : source.state === "empty" ? "没有 Workflow 文件"
                                : "无法读取"}</span>
                            </summary>
                            {source.items.length ? (
                              <div className="native-skill-items">
                                {source.items.map((item) => (
                                  <div key={item.path}>
                                    <span title={item.path}>{item.name}</span>
                                    <small>{item.state === "present" ? "已发现" : item.state === "empty" ? "空文件" : item.state === "unsafe" ? "不安全路径" : "无法读取"}</small>
                                  </div>
                                ))}
                              </div>
                            ) : <p className="native-skill-empty">{source.reason}</p>}
                          </details>
                        ))}
                      </div>
                    </section>
                  </>
                )}
                {tab !== "rules" && tab !== "skills" && tab !== "workflows" && tab !== "hooks" && tab !== "subagents" && (
                  <div className="empty-state">
                    <span className="empty-icon" aria-hidden="true">◌</span>
                    <strong>{tabs.find((item) => item.id === tab)?.label} 尚未进入当前迭代</strong>
                    <p>该配置尚未纳入当前只读盘点。</p>
                  </div>
                )}
              </div>
            </section>
            {tab !== "rules" && (
              <aside className="panel panel-side source-panel">
                <p className="eyebrow">基线来源</p>
                <h2>公共配置</h2>
                <ProvenanceChain active="global" />
                <p className="panel-note">
                  全局配置先进入每个 Agent，再叠加项目 Profile。任何不支持的能力都会在应用前显示。
                </p>
                <div className="source-file">
                  <span>Canonical source</span>
                  <code>{snapshot.global.canonicalPath}</code>
                </div>
              </aside>
            )}
          </>
        )}
      </div>
    </div>
  );
}

function nativeSkillLabel(agent: string): string {
  if (agent === "antigravity-ide") return "Antigravity IDE";
  if (agent === "antigravity-cli") return "Antigravity CLI";
  if (agent === "antigravity") return "Antigravity";
  if (agent === "shared") return "通用";
  return agent;
}
