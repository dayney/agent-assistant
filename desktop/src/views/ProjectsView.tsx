import { useState } from "react";
import type { CoreClient } from "../core/client";
import type { ProjectSummary, WorkspaceSnapshot } from "../core/model";
import { ProvenanceChain } from "../components/ProvenanceChain";
import { StatusBadge } from "../components/StatusBadge";
import { useProjectImport } from "../hooks/useProjectImport";

interface ProjectsViewProps {
  snapshot: WorkspaceSnapshot;
  client: CoreClient;
  onSnapshotRefresh: () => Promise<void>;
}

export function ProjectsView({
  snapshot,
  client,
  onSnapshotRefresh,
}: ProjectsViewProps) {
  const [selectedId, setSelectedId] = useState(snapshot.projects[0]?.id ?? "");
  const [promotingId, setPromotingId] = useState<string | null>(null);
  const [mcpMessage, setMcpMessage] = useState("");
  const [mcpError, setMcpError] = useState("");
  const selected = snapshot.projects.find((item) => item.id === selectedId);

  async function promoteProjectMcp(mcpId: string) {
    if (!selected) return;
    setPromotingId(mcpId);
    setMcpMessage("");
    setMcpError("");
    try {
      const result = await client.promoteProjectMcp(selected.path, mcpId);
      setMcpMessage(
        result.status === "already-global"
          ? `${mcpId} 已经在全局 MCP 中。`
          : `${mcpId} 已提升为全局 MCP。`,
      );
      await onSnapshotRefresh();
    } catch (cause) {
      setMcpError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setPromotingId(null);
    }
  }

  const {
    projectPath,
    setProjectPath,
    imported,
    proposal,
    proposalBody,
    setProposalBody,
    selectedAgents,
    toggleAgent,
    detectedAgents,
    busy,
    message,
    error,
    handleImport,
    handleAnalyze,
    handleSaveProposal,
  } = useProjectImport(client, snapshot, onSnapshotRefresh);

  return (
    <div className="view-stack">
      <section className="page-heading page-heading-row">
        <div>
          <p className="eyebrow">Project Profiles</p>
          <h1>项目</h1>
          <p className="lede">手动导入项目，审查原生 Rule，再建立项目母版。</p>
        </div>
      </section>
      <section className="panel project-import-panel">
        <div className="project-import-form">
          <label>
            <span>本机项目路径</span>
            <input
              value={projectPath}
              onChange={(event) => setProjectPath(event.target.value)}
              placeholder="/Users/name/git/work/project"
              disabled={busy}
            />
          </label>
          <button
            className="button button-primary"
            type="button"
            onClick={handleImport}
            disabled={busy || !projectPath.trim()}
          >
            导入项目
          </button>
        </div>
        {error && (
          <div className="inline-alert is-error" role="alert">
            {error}
          </div>
        )}
        {message && (
          <div className="inline-alert" role="status">
            {message}
          </div>
        )}
        {imported && (
          <div className="import-analysis">
            <div className="source-evidence">
              <div className="section-heading">
                <div>
                  <p className="eyebrow">Native Evidence</p>
                  <h2>原生 Rule 来源</h2>
                </div>
                <span className="target-count">{imported.sources.length}</span>
              </div>
              {imported.sources.length ? (
                imported.sources.map((source) => (
                  <div className="evidence-row" key={source.path}>
                    <code>{source.path}</code>
                    <span>{source.agents.join("、")}</span>
                  </div>
                ))
              ) : (
                <p className="panel-note">
                  未发现已知 Agent 的项目 Rule 文件。
                </p>
              )}
            </div>
            <div className="proposal-pane">
              <div className="section-heading">
                <div>
                  <p className="eyebrow">Reviewed Proposal</p>
                  <h2>项目母版候选</h2>
                </div>
                {imported.sources.length > 0 && (
                  <button
                    className="button button-secondary"
                    type="button"
                    onClick={handleAnalyze}
                    disabled={busy}
                  >
                    {imported.needsAnalysis
                      ? "使用本机 Codex 分析"
                      : "生成一致候选"}
                  </button>
                )}
              </div>
              {proposal ? (
                <>
                  <textarea
                    className="proposal-textarea"
                    value={proposalBody}
                    onChange={(event) => setProposalBody(event.target.value)}
                    spellCheck={false}
                    aria-label="项目母版候选"
                  />
                  <div className="proposal-meta">
                    <span>分析器：{proposal.analyzer}</span>
                    {proposal.notes.map((note) => (
                      <p key={note}>{note}</p>
                    ))}
                  </div>
                  <div className="agent-selector compact">
                    {detectedAgents.map((agent) => (
                      <label key={agent}>
                        <input
                          type="checkbox"
                          checked={selectedAgents.includes(agent)}
                          onChange={(event) =>
                            toggleAgent(agent, event.target.checked)
                          }
                        />
                        <span>{agent}</span>
                      </label>
                    ))}
                  </div>
                  <div className="rule-actions">
                    <button
                      className="button button-primary"
                      type="button"
                      onClick={handleSaveProposal}
                      disabled={busy}
                    >
                      保存为项目母版
                    </button>
                  </div>
                </>
              ) : (
                <div className="proposal-empty">
                  <strong>尚未生成候选</strong>
                  <p>分析不会写入项目；保存前可以完整修改候选内容。</p>
                </div>
              )}
            </div>
          </div>
        )}
      </section>
      <div className="content-grid project-grid">
        <section className="panel panel-main project-list-panel">
          <div className="panel-header">
            <div>
              <p className="eyebrow">已纳管</p>
              <h2>{snapshot.projects.length} 个项目</h2>
            </div>
          </div>
          <div className="project-list">
            {snapshot.projects.map((item) => (
              <ProjectRow
                key={item.path}
                project={item}
                active={item.id === selected?.id}
                onClick={() => setSelectedId(item.id)}
              />
            ))}
          </div>
        </section>
        {selected && (
          <aside className="panel panel-side project-detail">
            <div className="detail-heading">
              <div className="project-icon" aria-hidden="true">
                {selected.name.slice(0, 1)}
              </div>
              <div>
                <p className="eyebrow">当前项目</p>
                <h2>{selected.name}</h2>
                <code>{selected.path}</code>
              </div>
            </div>
            <StatusBadge value={selected.syncState} />

            <DetailSection label="Profile">
              <strong>{selected.profile}</strong>
            </DetailSection>

            <DetailSection label="技术栈">
              <div className="tag-list">
                {selected.stack.map((item) => (
                  <span className="tag" key={item}>
                    {item}
                  </span>
                ))}
              </div>
            </DetailSection>

            <DetailSection label="绑定 Agent">
              <div className="agent-chip-list">
                {selected.agents.map((agent) => (
                  <span className="agent-chip" key={agent}>
                    <span className="agent-avatar mini" aria-hidden="true">
                      {agent.slice(0, 1)}
                    </span>
                    {agent}
                  </span>
                ))}
              </div>
            </DetailSection>

            <DetailSection label="项目 MCP">
              {selected.mcp.length === 0 ? (
                <p className="panel-note">当前项目没有 MCP 候选。</p>
              ) : (
                <div className="project-mcp-list">
                  {selected.mcp.map((mcp) => (
                    <div className="project-mcp-row" key={mcp.id}>
                      <div>
                        <strong>{mcp.name}</strong>
                        <small>
                          {mcp.recipe
                            ? `Recipe：${mcp.recipe}`
                            : "未声明 Recipe"}
                          {mcp.credentialState === "configured"
                            ? " · 凭据已引用"
                            : mcp.credentialState === "not-required"
                              ? " · 无需 Token"
                              : " · 凭据需处理"}
                        </small>
                      </div>
                      <button
                        className="button button-secondary button-small"
                        type="button"
                        onClick={() => void promoteProjectMcp(mcp.id)}
                        disabled={promotingId !== null}
                      >
                        {promotingId === mcp.id ? "处理中" : "转为全局"}
                      </button>
                    </div>
                  ))}
                </div>
              )}
              {mcpError && <p className="inline-alert is-error" role="alert">{mcpError}</p>}
              {mcpMessage && <p className="panel-note" role="status">{mcpMessage}</p>}
              <p className="panel-note">项目 MCP 只在本项目生效；提升后才进入全局 canonical，并由各 Agent 适配器统一生成。</p>
            </DetailSection>

            <DetailSection label="生效路径" className="detail-source">
              <ProvenanceChain />
              <p className="panel-note">最后更新 {selected.updatedAt}</p>
            </DetailSection>
          </aside>
        )}
      </div>
    </div>
  );
}

function DetailSection({
  label,
  children,
  className = "",
}: {
  label: string;
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <div className={`detail-section ${className}`.trim()}>
      <span className="detail-label">{label}</span>
      {children}
    </div>
  );
}

function ProjectRow({
  project,
  active,
  onClick,
}: {
  project: ProjectSummary;
  active: boolean;
  onClick: () => void;
}) {
  return (
    <button
      className={active ? "project-row is-active" : "project-row"}
      type="button"
      onClick={onClick}
    >
      <span className="project-icon" aria-hidden="true">
        {project.name.slice(0, 1)}
      </span>
      <span className="project-row-copy">
        <strong>{project.name}</strong>
        <small>{project.path}</small>
      </span>
      <StatusBadge value={project.syncState} />
      <span className="row-chevron" aria-hidden="true">
        ›
      </span>
    </button>
  );
}
