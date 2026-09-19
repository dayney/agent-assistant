import { useState, useCallback, useEffect } from "react";
import { Eye, EyeOff } from "lucide-react";
import type { CoreClient } from "../core/client";
import type {
  GlobalMcpItem,
  McpAdapter,
  McpCredential,
  ProjectMcpItem,
  ProjectSummary,
  WorkspaceSnapshot,
} from "../core/model";

type McpFilter = "all" | "global" | "agent" | "project";

interface McpCatalogRow {
  key: string;
  id: string;
  name: string;
  description: string;
  endpoint: string;
  scope: "global" | "agent" | "project";
  scopeLabel: string;
  sourceLabel: string;
  projectPath?: string;
  projectName?: string;
  recipe: string;
  recipeStatus: string;
  credentialState: string;
  credentials: McpCredential[];
  adapters: McpAdapter[];
  canPromote: boolean;
  serverId: string;
  nativeAgent?: string;
}

interface McpViewProps {
  snapshot: WorkspaceSnapshot;
  client: CoreClient;
  onSnapshotRefresh: () => Promise<void>;
}

export function McpView({ snapshot, client, onSnapshotRefresh }: McpViewProps) {
  const [filter, setFilter] = useState<McpFilter>("all");
  const [query, setQuery] = useState("");
  const [expandedKey, setExpandedKey] = useState<string | null>(null);
  const [promoting, setPromoting] = useState<string | null>(null);
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");

  const rows = buildMcpCatalog(snapshot);
  const visibleRows = rows.filter((row) => matchesFilter(row, filter, query));

  async function promote(row: McpCatalogRow) {
    const operationKey = row.scope === "project"
      ? `project:${row.projectPath}:${row.serverId}`
      : `${row.nativeAgent}:${row.serverId}`;
    setPromoting(operationKey);
    setMessage("");
    setError("");
    try {
      const result = row.scope === "project" && row.projectPath
        ? await client.promoteProjectMcp(row.projectPath, row.serverId)
        : await client.promoteNativeMcp(row.nativeAgent ?? "", row.serverId);
      setMessage(
        result.status === "already-global"
          ? `${row.id} 已经在全局 MCP 中。`
          : `${row.id} 已转为全局 MCP。`,
      );
      await onSnapshotRefresh();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setPromoting(null);
    }
  }

  const counts = {
    all: rows.length,
    global: rows.filter((row) => row.scope === "global").length,
    agent: rows.filter((row) => row.scope === "agent").length,
    project: rows.filter((row) => row.scope === "project").length,
  };

  const scopeTabs = [
    ["all", "全部", counts.all],
    ["global", "全局", counts.global],
    ["agent", "Agent 原生", counts.agent],
    ["project", "项目", counts.project],
  ] as const;

  return (
    <div className="mcp-workspace">
      {/* 模块一：目录列表卡 */}
      <div className="mcp-module mcp-list-module">
        <div className="mcp-module-header">
          <div className="mcp-module-title">
            <span className="eyebrow">MCP 目录</span>
            <span className="mcp-total-badge">{counts.all}</span>
          </div>
          <label className="mcp-search">
            <span className="sr-only">搜索 MCP</span>
            <input
              type="search"
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder="搜索名称、ID 或 Agent…"
            />
          </label>
        </div>

        <div className="mcp-tab-bar">
          <div className="mcp-tab-list" role="tablist" aria-label="MCP 作用域">
            {scopeTabs.map(([scope, label, count]) => (
              <button
                key={scope}
                className={filter === scope ? "mcp-tab is-active" : "mcp-tab"}
                type="button"
                role="tab"
                aria-selected={filter === scope}
                onClick={() => setFilter(scope)}
              >
                {label}
                <span className="mcp-tab-badge">{count}</span>
              </button>
            ))}
          </div>
        </div>

        {error && <p className="mcp-inline-alert is-error" role="alert">{error}</p>}
        {message && <p className="mcp-inline-alert is-success" role="status">{message}</p>}

        <div className="mcp-catalog" role="table" aria-label="MCP 配置目录">
          <div className="mcp-catalog-head" role="row">
            <span>MCP</span>
            <span>作用域</span>
            <span>适配 Agent</span>
            <span>端点</span>
            <span></span>
          </div>
          {visibleRows.length === 0 ? (
            <div className="empty-state" role="row">
              <strong>没有匹配的 MCP</strong>
              <p>尝试清除搜索或切换作用域。</p>
            </div>
          ) : visibleRows.flatMap((row) => {
            const operationKey = row.scope === "project"
              ? `project:${row.projectPath}:${row.serverId}`
              : `${row.nativeAgent}:${row.serverId}`;
            const isExpanded = row.key === expandedKey;

            const catalogRow = (
              <div
                className={`mcp-catalog-row${isExpanded ? " is-selected" : ""}`}
                key={row.key}
                role="row"
                aria-selected={isExpanded}
                aria-expanded={isExpanded}
                onClick={() => setExpandedKey(isExpanded ? null : row.key)}
                tabIndex={0}
                onKeyDown={(e) => e.key === "Enter" && setExpandedKey(isExpanded ? null : row.key)}
              >
                <div className="mcp-primary-cell">
                  <div className="mcp-primary-title">
                    <strong>{row.name}</strong>
                    {row.credentialState === "plaintext-blocked" && (
                      <span className="mcp-cred-badge is-blocked" title="存在明文密钥">⚠ 明文</span>
                    )}
                    {row.credentialState === "configured" && (
                      <span className="mcp-cred-badge is-ok" title="密钥已配置">🔑</span>
                    )}
                  </div>
                  <code>{row.id}</code>
                  {row.description && (
                    <span className="mcp-row-desc">{row.description}</span>
                  )}
                </div>

                <div className="mcp-scope-cell">
                  <span className={`scope-chip scope-${row.scope}`}>{row.scopeLabel}</span>
                  <small>{row.projectName ?? ""}</small>
                </div>

                <div className="mcp-adapter-cell">
                  {renderAdapterChips(row.adapters)}
                </div>

                <div className="mcp-runtime-cell">
                  <strong>{row.recipe ? row.recipe : "—"}</strong>
                  <small title={row.endpoint}>{row.endpoint}</small>
                </div>

                <div className="mcp-action-cell">
                  <span className={`mcp-expand-chevron${isExpanded ? " is-open" : ""}`} aria-hidden="true">›</span>
                </div>
              </div>
            );

            if (!isExpanded) return [catalogRow];

            const drawerEl = (
              <McpRowDrawer
                key={`drawer-${row.key}`}
                row={row}
                promoting={promoting}
                operationKey={operationKey}
                client={client}
                mode={snapshot.mode}
                onPromote={promote}
                onSaved={onSnapshotRefresh}
                onClose={() => setExpandedKey(null)}
              />
            );
            return [catalogRow, drawerEl];
          })}
        </div>
      </div>

      {/* 模块二：Native Sources 卡 */}
      <div className="mcp-module mcp-native-module" aria-labelledby="native-mcp-title">
        <div className="mcp-module-header">
          <div className="mcp-module-title">
            <span className="eyebrow">Native Sources</span>
            <span className="mcp-native-status">
              已解析 {snapshot.global.nativeMcp.filter((s) => s.state === "parsed").length}
              /{snapshot.global.nativeMcp.length}
            </span>
          </div>
          <h2 id="native-mcp-title" className="mcp-module-h2">本机 MCP 来源</h2>
        </div>
        <p className="native-source-note">
          {snapshot.mode === "demo" ? "以下为演示示例，不是本机扫描结果。" : "本机来源尚未导入母版。"}
          Antigravity IDE 与 Antigravity CLI 共享同一份配置；
          {snapshot.global.nativeMcp.find((source) => source.agent === "antigravity")?.state === "parsed"
            ? "Antigravity 桌面应用也链接到该文件。"
            : "Antigravity 桌面应用的共享链接未核实。"}
        </p>
        <div className="native-mcp-list">
          {snapshot.global.nativeMcp.slice().sort((left, right) =>
            Number(right.state === "parsed") - Number(left.state === "parsed"),
          ).map((source, index) => (
            <div className="native-mcp-row" key={`${source.agent}:${source.path}:${index}`}>
              <strong>{nativeMcpLabel(source.agent)}</strong>
              <code title={source.path || source.reason}>{source.path || source.reason}</code>
              <span title={source.reason}>{nativeSourceStatus(source)}</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

// ── 行内抽屉组件 ────────────────────────────────────────────────────────────────
function McpRowDrawer({
  row, promoting, operationKey, client, mode, onPromote, onSaved, onClose,
}: {
  row: McpCatalogRow;
  promoting: string | null;
  operationKey: string;
  client: CoreClient;
  mode: string;
  onPromote: (row: McpCatalogRow) => Promise<void>;
  onSaved: () => Promise<void>;
  onClose: () => void;
}) {
  const [editing, setEditing] = useState(false);
  const [confirmPromote, setConfirmPromote] = useState(false);
  const [importingFrom, setImportingFrom] = useState<string | null>(null);
  const [importResult, setImportResult] = useState<string | null>(null);
  const canEdit = row.scope === "global" && mode !== "demo";

  const handleImportNative = async (agent: string) => {
    setImportingFrom(agent);
    setImportResult(null);
    try {
      const result = await client.importNativeMcpForce(agent, row.serverId);
      setImportResult(result.status === "force-imported" ? "✓ 已从原生导入并覆盖母版" : "✓ 已同步（内容一致）");
      await onSaved();
    } catch (err) {
      setImportResult(`✗ 导入失败：${String(err)}`);
    } finally {
      setImportingFrom(null);
    }
  };

  return (
    <div className="mcp-row-drawer" role="region" aria-label={`${row.name} 详情`}>
      <div className="mcp-drawer-header">
        <div className="mcp-drawer-title-group">
          <span className={`scope-chip scope-${row.scope}`}>{row.scopeLabel}</span>
          <h3 className="mcp-drawer-title">{row.name}</h3>
          {row.description && <p className="mcp-drawer-desc">{row.description}</p>}
        </div>
        <div className="mcp-drawer-actions">
          {canEdit && !editing && (
            <button type="button" className="button button-quiet button-small" onClick={() => setEditing(true)}>
              编辑
            </button>
          )}
          <button type="button" className="button button-quiet button-small" onClick={onClose} aria-label="关闭详情">
            ✕
          </button>
        </div>
      </div>

      {editing ? (
        <McpEditForm
          row={row}
          client={client}
          onSaved={async () => { setEditing(false); await onSaved(); }}
          onCancel={() => setEditing(false)}
        />
      ) : (
        <>
          <dl className="mcp-detail-list">
            <div><dt>MCP ID</dt><dd><code>{row.id}</code></dd></div>
            <div><dt>Recipe</dt><dd>{row.recipe || "未声明 Recipe"}</dd></div>
            <div><dt>运行端点</dt><dd><code>{row.endpoint}</code></dd></div>
            <div><dt>来源</dt><dd><code>{row.sourceLabel}</code></dd></div>
          </dl>

          {/* 从原生导入到母版：仅当该 MCP 存在于原生配置且不在 demo 模式时显示 */}
          {row.nativeAgent && mode !== "demo" && (
            <section className="mcp-detail-section">
              <p className="eyebrow">从原生导入到母版</p>
              <p className="panel-note">
                将 <strong>{agentDisplayName(row.nativeAgent)}</strong> 原生配置文件中的真实参数（command/args/env）同步覆盖到母版；明文凭据会自动转换为环境变量引用，不会写入母版。
              </p>
              <div className="mcp-detail-row" style={{ gap: "8px", flexWrap: "wrap" }}>
                {([row.nativeAgent, ...(row.nativeAgent === "antigravity" ? ["antigravity-ide"] : [])]
                  .filter(Boolean) as string[]).map((agent) => (
                  <button
                    key={agent}
                    type="button"
                    className="button button-secondary button-small"
                    disabled={importingFrom !== null}
                    onClick={() => handleImportNative(agent)}
                  >
                    {importingFrom === agent ? "导入中…" : `从 ${agentDisplayName(agent)} 导入`}
                  </button>
                ))}
              </div>
              {importResult && (
                <p className={`panel-note ${importResult.startsWith("✓") ? "mcp-import-ok" : "mcp-import-err"}`}>
                  {importResult}
                </p>
              )}
            </section>
          )}

          <section className="mcp-detail-section">
            <p className="eyebrow">凭据</p>
            {row.credentials.length ? row.credentials.map((cred) => (
              <div className="mcp-detail-row" key={cred.key}>
                <strong>{cred.key}</strong>
                <span>{credentialLabel(cred.status)} · {credentialSourceLabel(cred.source)}</span>
                {cred.reference && <CopyableReference value={cred.reference} />}
              </div>
            )) : <p className="panel-note">该 MCP 不需要密钥。</p>}
          </section>

          <section className="mcp-detail-section">
            <p className="eyebrow">Agent 适配</p>
            <div className="mcp-detail-adapters">
              {row.adapters.length ? row.adapters.map((adapter) => (
                <AgentAdapterRow
                  key={adapter.agent}
                  mcpId={row.serverId}
                  adapter={adapter}
                  client={client}
                  mode={mode}
                  onSnapshotRefresh={onSaved}
                />
              )) : <p className="panel-note">暂未核实 Agent 适配状态。</p>}
            </div>
          </section>

          {row.canPromote && !confirmPromote && (
            <button
              className="button button-primary button-wide"
              type="button"
              disabled={promoting !== null}
              onClick={() => setConfirmPromote(true)}
            >
              转换至全局
            </button>
          )}
          {row.canPromote && confirmPromote && (
            <PromoteConfirmStep
              row={row}
              promoting={promoting === operationKey}
              onConfirm={async () => { setConfirmPromote(false); await onPromote(row); }}
              onCancel={() => setConfirmPromote(false)}
            />
          )}
        </>
      )}
    </div>
  );
}

// ── 编辑表单组件 ────────────────────────────────────────────────────────────────
function McpEditForm({
  row, client, onSaved, onCancel,
}: {
  row: McpCatalogRow;
  client: CoreClient;
  onSaved: () => Promise<void>;
  onCancel: () => void;
}) {
  const [draftName, setDraftName] = useState(row.name);
  const [draftDesc, setDraftDesc] = useState(row.description);
  const [draftRecipe, setDraftRecipe] = useState(row.recipe);
  const [draftEndpoint, setDraftEndpoint] = useState(row.endpoint);
  const [draftEnv, setDraftEnv] = useState<Record<string, string>>(() => {
    const map: Record<string, string> = {};
    for (const cred of row.credentials) { map[cred.key] = cred.reference ?? ""; }
    return map;
  });
  const [secretVisible, setSecretVisible] = useState<Record<string, boolean>>({});
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState("");

  const hasUnsafeRef = Object.values(draftEnv).some(
    (v) => v && !v.startsWith("${secret:") && !v.startsWith("${env:"),
  );

  const handleSave = useCallback(async () => {
    setSaving(true);
    setSaveError("");
    try {
      const isHttp = draftEndpoint.startsWith("http://") || draftEndpoint.startsWith("https://");
      await client.saveGlobalMcp({
        mcpId: row.serverId,
        name: draftName,
        description: draftDesc,
        recipe: draftRecipe,
        url: isHttp ? draftEndpoint : "",
        command: isHttp ? "" : draftEndpoint,
        env: draftEnv,
      });
      await onSaved();
    } catch (error) {
      setSaveError(error instanceof Error ? error.message : String(error));
    } finally {
      setSaving(false);
    }
  }, [row, client, draftName, draftDesc, draftRecipe, draftEndpoint, draftEnv, onSaved]);

  return (
    <div className="mcp-edit-form">
      <label className="mcp-edit-field">
        <span>名称</span>
        <input type="text" value={draftName} onChange={(e) => setDraftName(e.target.value)} placeholder="MCP 显示名称" />
      </label>
      <label className="mcp-edit-field">
        <span>功能简介</span>
        <input type="text" value={draftDesc} onChange={(e) => setDraftDesc(e.target.value)} placeholder="一句话描述这个 MCP 的用途" />
      </label>
      <label className="mcp-edit-field">
        <span>Recipe</span>
        <input type="text" value={draftRecipe} onChange={(e) => setDraftRecipe(e.target.value)} placeholder="如 github、chrome-devtools" />
      </label>
      <label className="mcp-edit-field">
        <span>运行端点</span>
        <input type="text" value={draftEndpoint} onChange={(e) => setDraftEndpoint(e.target.value)} placeholder="命令路径或 https:// URL" />
      </label>

      {row.credentials.length > 0 && (
        <div className="mcp-edit-credentials">
          <p className="eyebrow">凭据</p>
          {row.credentials.map((cred) => {
            const val = draftEnv[cred.key] ?? "";
            const isVisible = secretVisible[cred.key] ?? false;
            const unsafe = val && !val.startsWith("${secret:") && !val.startsWith("${env:");
            return (
              <label key={cred.key} className="mcp-edit-field">
                <span>
                  {cred.key}
                  {unsafe && <span className="mcp-edit-warn" title="建议使用 ${secret:X} 格式">⚠</span>}
                </span>
                <div className="mcp-secret-input-wrap">
                  <input
                    type={isVisible ? "text" : "password"}
                    value={val}
                    onChange={(e) => setDraftEnv({ ...draftEnv, [cred.key]: e.target.value })}
                    placeholder={cred.reference ?? `\${secret:${cred.key}}`}
                    autoComplete="off"
                  />
                  <button
                    type="button"
                    className="mcp-secret-eye"
                    aria-label={isVisible ? "隐藏密钥" : "显示密钥"}
                    onClick={() => setSecretVisible((prev) => ({ ...prev, [cred.key]: !isVisible }))}
                  >
                    {isVisible ? <EyeOff size={16} /> : <Eye size={16} />}
                  </button>
                </div>
              </label>
            );
          })}
        </div>
      )}

      {hasUnsafeRef && (
        <p className="mcp-edit-warn-banner">
          ⚠ 凭据值不是 <code>{"${secret:X}"}</code> 或 <code>{"${env:X}"}</code> 格式，保存后全局推广可能被阻断。
        </p>
      )}
      {saveError && <p className="inline-alert is-error" role="alert">{saveError}</p>}

      <div className="mcp-edit-actions">
        <button type="button" className="button button-primary" disabled={saving || !draftName.trim()} onClick={() => void handleSave()}>
          {saving ? "保存中…" : "保存"}
        </button>
        <button type="button" className="button button-quiet" disabled={saving} onClick={onCancel}>
          取消
        </button>
      </div>
    </div>
  );
}

// ── 两步确认转全局 ────────────────────────────────────────────────────────────────
function PromoteConfirmStep({
  row, promoting, onConfirm, onCancel,
}: {
  row: McpCatalogRow;
  promoting: boolean;
  onConfirm: () => Promise<void>;
  onCancel: () => void;
}) {
  const hasPlaintext = row.credentials.some((c) => c.status === "plaintext-blocked");
  return (
    <div className="mcp-promote-confirm">
      <p className="mcp-promote-confirm-title">
        将 <strong>{row.id}</strong> 转换至全局 MCP？
        {row.scope === "project" && row.projectName && (
          <span className="mcp-promote-source"> · 来自项目 {row.projectName}</span>
        )}
      </p>
      {hasPlaintext && (
        <p className="mcp-promote-warn">
          ⚠ 该 MCP 包含明文密钥，转换后建议改为 <code>{"${secret:X}"}</code> 格式以防止泄露。
        </p>
      )}
      <div className="mcp-promote-confirm-actions">
        <button type="button" className="button button-primary button-small" disabled={promoting} onClick={() => void onConfirm()}>
          {promoting ? "处理中…" : "确认转换"}
        </button>
        <button type="button" className="button button-quiet button-small" disabled={promoting} onClick={onCancel}>
          取消
        </button>
      </div>
    </div>
  );
}

function buildMcpCatalog(snapshot: WorkspaceSnapshot): McpCatalogRow[] {
  const globalRows = snapshot.global.mcp.map((item) => globalRow(item, snapshot.global.canonicalPath));
  const nativeRows = snapshot.global.nativeMcpItems.map((item) => nativeRow(item));
  const projectRows = snapshot.projects.flatMap((project) =>
    project.mcp.map((item) => projectRow(project, item)),
  );
  const rows = [...globalRows];
  const globalByServerId = new Map(globalRows.map((row) => [row.serverId, row]));
  const nativeByIdentity = new Map<string, McpCatalogRow>();

  for (const native of nativeRows) {
    const global = globalByServerId.get(native.serverId);
    if (global && sameMcpDefinition(global, native)) {
      mergeMcpMetadata(global, native);
      continue;
    }

    const identity = mcpIdentity(native);
    const existing = nativeByIdentity.get(identity);
    if (existing) {
      mergeMcpMetadata(existing, native);
      continue;
    }
    nativeByIdentity.set(identity, native);
    rows.push(native);
  }

  return [...rows, ...projectRows];
}

function mcpIdentity(row: McpCatalogRow): string {
  // Secret references may differ between canonical and native files; they do not
  // make the same server a second catalog entry.
  return [row.serverId, row.endpoint, row.recipe].join("|");
}

function sameMcpDefinition(left: McpCatalogRow, right: McpCatalogRow): boolean {
  return mcpIdentity(left) === mcpIdentity(right);
}

function mergeMcpMetadata(target: McpCatalogRow, incoming: McpCatalogRow): void {
  const adapters = new Map(target.adapters.map((adapter) => [adapter.agent, adapter]));
  for (const adapter of incoming.adapters) {
    const existing = adapters.get(adapter.agent);
    if (!existing || existing.status === "unverified") {
      adapters.set(adapter.agent, adapter);
    }
  }
  target.adapters = [...adapters.values()].sort((left, right) => left.agent.localeCompare(right.agent));

  const credentials = new Map(target.credentials.map((credential) => [credential.key, credential]));
  for (const credential of incoming.credentials) {
    const existing = credentials.get(credential.key);
    if (!existing || existing.status === "missing") {
      credentials.set(credential.key, credential);
    }
  }
  target.credentials = [...credentials.values()].sort((left, right) => left.key.localeCompare(right.key));
}

function globalRow(item: GlobalMcpItem, canonicalPath: string): McpCatalogRow {
  return {
    key: `global:${item.serverId}`,
    id: item.serverId,
    name: item.name,
    description: item.description,
    endpoint: item.endpoint,
    scope: "global",
    scopeLabel: "全局",
    sourceLabel: `${canonicalPath}/mcp/${item.serverId}.toml`,
    recipe: item.recipe,
    recipeStatus: item.recipeStatus,
    credentialState: item.credentialState,
    credentials: item.credentials,
    adapters: item.adapters,
    canPromote: false,
    serverId: item.serverId,
  };
}

function nativeRow(item: GlobalMcpItem): McpCatalogRow {
  return {
    key: `agent:${item.serverId}:${item.agents.join(",")}`,
    id: item.serverId,
    name: item.name,
    description: item.description,
    endpoint: item.endpoint,
    scope: "agent",
    scopeLabel: "Agent 原生",
    sourceLabel: "本机 Agent 原生配置",
    recipe: item.recipe,
    recipeStatus: item.recipeStatus,
    credentialState: item.credentialState,
    credentials: item.credentials,
    adapters: item.adapters,
    canPromote: item.canPromote,
    serverId: item.serverId,
    nativeAgent: item.agents[0] ?? item.adapters[0]?.agent,
  };
}

function projectRow(project: ProjectSummary, item: ProjectMcpItem): McpCatalogRow {
  return {
    key: `project:${project.path}:${item.id}`,
    id: item.id,
    name: item.name,
    description: "项目范围 MCP，仅在当前项目生效。",
    endpoint: item.endpoint,
    scope: "project",
    scopeLabel: "项目",
    sourceLabel: `${project.path}/.agentsync/mcp/${item.id}.toml`,
    projectPath: project.path,
    projectName: project.name,
    recipe: item.recipe,
    recipeStatus: item.recipeStatus,
    credentialState: item.credentialState,
    credentials: item.credentials,
    adapters: item.adapters,
    canPromote: true,
    serverId: item.id,
  };
}

function matchesFilter(row: McpCatalogRow, filter: McpFilter, query: string): boolean {
  if (filter !== "all" && row.scope !== filter) return false;
  const normalized = query.trim().toLowerCase();
  if (!normalized) return true;
  return [
    row.id, row.name, row.description, row.projectName ?? "", row.recipe,
    ...row.adapters.map((adapter) => agentDisplayName(adapter.agent)),
  ].some((value) => value.toLowerCase().includes(normalized));
}

function renderAdapterChips(adapters: McpAdapter[]) {
  if (!adapters.length) return <span className="mcp-muted">未核实</span>;
  const visible = adapters.slice(0, 2);
  const rest = adapters.length - visible.length;
  return (
    <div className="mcp-adapter-list">
      {visible.map((adapter) => (
        <span className={`mcp-adapter-chip is-${adapter.status}`} key={adapter.agent}>
          {agentDisplayName(adapter.agent)}
        </span>
      ))}
      {rest > 0 && <small>+{rest}</small>}
    </div>
  );
}

function credentialLabel(status: string): string {
  if (status === "configured") return "已引用";
  if (status === "missing") return "缺失";
  if (status === "plaintext-blocked") return "明文阻断";
  return "无需密钥";
}

function credentialSourceLabel(source: string): string {
  if (source === "secret-vault") return "Secret Vault";
  if (source === "environment") return "环境变量";
  if (source === "literal") return "明文配置";
  return "未配置";
}

function adapterLabel(status: string): string {
  if (status === "supported") return "可适配";
  if (status === "partial") return "部分支持";
  if (status === "unsupported") return "不支持";
  if (status === "native") return "原生发现";
  if (status === "project") return "项目目标";
  return "待核实";
}

// ── 单个 Agent 适配行（含「适配」和「测试」按钮） ─────────────────────────────
function AgentAdapterRow({
  mcpId,
  adapter,
  client,
  mode,
  onSnapshotRefresh,
}: {
  mcpId: string;
  adapter: McpAdapter;
  client: CoreClient;
  mode: string;
  onSnapshotRefresh: () => Promise<void>;
}) {
  const [pushing, setPushing] = useState(false);
  const [updating, setUpdating] = useState(false);
  const [verifying, setVerifying] = useState(false);
  const [checking, setChecking] = useState(mode !== "demo");
  const [verifyStatus, setVerifyStatus] = useState<string | null>(null);
  const [pushResult, setPushResult] = useState<string | null>(null);
  const [updateResult, setUpdateResult] = useState<string | null>(null);
  const [opError, setOpError] = useState("");

  const isDemoOrUnsupported = mode === "demo";

  useEffect(() => {
    if (mode === "demo") {
      setChecking(false);
      return;
    }
    let cancelled = false;
    setChecking(true);
    void client
      .verifyMcpInAgent({ mcpId, agent: adapter.agent, runtimeCheck: false })
      .then((result) => {
        if (!cancelled) setVerifyStatus(result.status);
      })
      .catch(() => {
        // An unsupported or unverified adapter is still allowed to render its status.
      })
      .finally(() => {
        if (!cancelled) setChecking(false);
      });
    return () => {
      cancelled = true;
    };
  }, [mcpId, adapter.agent, client, mode]);

  const handlePush = useCallback(async () => {
    setPushing(true);
    setOpError("");
    setPushResult(null);
    try {
      const existing = await client.verifyMcpInAgent({
        mcpId,
        agent: adapter.agent,
        runtimeCheck: false,
      });
      if (existing.status === "found") {
        setVerifyStatus("found");
        return;
      }
      const result = await client.pushMcpToAgent({ mcpId, agent: adapter.agent });
      setPushResult(result.status);
      const verification = await client.verifyMcpInAgent({
        mcpId,
        agent: adapter.agent,
        runtimeCheck: false,
      });
      setVerifyStatus(verification.status);
      await onSnapshotRefresh();
    } catch (err) {
      setOpError(err instanceof Error ? err.message : String(err));
    } finally {
      setPushing(false);
    }
  }, [mcpId, adapter.agent, client, onSnapshotRefresh]);

  const handleUpdate = useCallback(async () => {
    setUpdating(true);
    setOpError("");
    setUpdateResult(null);
    try {
      await client.pushMcpToAgent({ mcpId, agent: adapter.agent });
      const verification = await client.verifyMcpInAgent({
        mcpId,
        agent: adapter.agent,
        runtimeCheck: true,
      });
      if (verification.status !== "found") {
        throw new Error(`更新后配置校验失败：${verification.status}`);
      }
      if (verification.runtimeStatus === "blocked-secret-reference") {
        setUpdateResult("updated-secret-blocked");
      } else if (
        verification.runtimeStatus !== "initialized" &&
        verification.runtimeStatus !== "not-applicable"
      ) {
        throw new Error(`更新后运行测试失败：${verification.runtimeStatus}`);
      } else {
        setUpdateResult("updated");
      }
      await onSnapshotRefresh();
    } catch (err) {
      setOpError(err instanceof Error ? err.message : String(err));
    } finally {
      setUpdating(false);
    }
  }, [mcpId, adapter.agent, client, onSnapshotRefresh]);

  const handleVerify = useCallback(async () => {
    setVerifying(true);
    setOpError("");
    try {
      const result = await client.verifyMcpInAgent({ mcpId, agent: adapter.agent, runtimeCheck: true });
      const runtimeStatus = result.runtimeStatus;
      setVerifyStatus(
        runtimeStatus === "initialized"
          ? "runtime-passed"
          : runtimeStatus === "not-applicable" || runtimeStatus === "not-requested"
          ? result.status
          : runtimeStatus === "blocked-secret-reference"
          ? "runtime-blocked"
          : result.status === "found"
          ? "runtime-failed"
          : result.status,
      );
    } catch (err) {
      setOpError(err instanceof Error ? err.message : String(err));
    } finally {
      setVerifying(false);
    }
  }, [mcpId, adapter.agent, client]);

  const hasMatchingConfig = ["found", "runtime-passed", "runtime-blocked", "runtime-failed"].includes(
    verifyStatus ?? "",
  );

  const dot = verifyStatus === "found" || verifyStatus === "runtime-passed"
    ? "🟢"
    : verifyStatus === "missing" || verifyStatus === "file-missing" || verifyStatus === "mismatch"
    ? "⚫"
    : verifyStatus === "parse-error"
    ? "🟡"
    : adapter.status === "native"
    ? "🟢"
    : adapter.status === "partial"
    ? "🟡"
    : "⚫";

  const verifyLabel = checking
    ? "检查适配状态…"
    : verifyStatus === "runtime-passed"
    ? "运行测试通过"
    : verifyStatus === "found"
    ? "文件一致"
    : verifyStatus === "missing"
    ? "未配置"
    : verifyStatus === "file-missing"
    ? "配置文件不存在"
    : verifyStatus === "parse-error"
    ? "配置文件格式异常"
    : verifyStatus === "mismatch"
    ? "配置内容不一致"
    : verifyStatus === "runtime-blocked"
    ? "密钥引用未解析"
    : verifyStatus === "runtime-failed"
    ? "运行测试失败"
    : verifyStatus === "unsupported"
    ? "不支持自动写入"
    : null;

  return (
    <div className="mcp-adapter-action-row">
      <div className="mcp-adapter-info">
        <span className="mcp-adapter-dot">{dot}</span>
        <strong>{agentDisplayName(adapter.agent)}</strong>
        {verifyLabel
          ? <span className="mcp-adapter-verify-label">{verifyLabel}</span>
          : <span className="mcp-adapter-status-label">{adapterLabel(adapter.status)}</span>
        }
      </div>
      {!isDemoOrUnsupported && (
        <div className="mcp-adapter-btns">
          {!checking && !hasMatchingConfig && (
            <button
              type="button"
              className="button button-primary button-small"
              disabled={pushing || updating || verifying}
              onClick={() => void handlePush()}
            >
              {pushing ? "适配中…" : pushResult === "written" ? "✓ 已写入" : pushResult === "updated" ? "✓ 已更新" : "适配"}
            </button>
          )}
          {!checking && hasMatchingConfig && (
            <button
              type="button"
              className="button button-quiet button-small"
              title="重新写入 canonical 配置并运行一次真实 MCP 校验"
              disabled={pushing || updating || verifying}
              onClick={() => void handleUpdate()}
            >
              {updating ? "更新中…" : updateResult === "updated" ? "✓ 已更新" : "更新"}
            </button>
          )}
          <button
            type="button"
            className="button button-quiet button-small"
            disabled={pushing || updating || verifying}
            onClick={() => void handleVerify()}
          >
            {verifying ? "测试中…" : "测试"}
          </button>
        </div>
      )}
      {opError && <p className="mcp-adapter-error">{opError}</p>}
      {(pushResult === "written" || pushResult === "updated" || updateResult === "updated-secret-blocked") && (
        <div className="mcp-adapter-push-notice" role="note">
          <span className="mcp-adapter-push-notice-icon">ℹ️</span>
          <span>
            配置已写入，但母版中的 <code>{"${secret:X}"}</code> / <code>{"${env:X}"}</code> 引用<strong>不会自动解析</strong>。
            请在 {agentDisplayName(adapter.agent)} 配置中确认密钥已填写为真实值，否则 MCP 无法正常工作。
          </span>
        </div>
      )}
      {updateResult === "updated-secret-blocked" && (
        <p className="mcp-adapter-error" role="status">
          配置已更新，但密钥引用未解析，运行测试已跳过。
        </p>
      )}
    </div>
  );
}

function agentDisplayName(agent: string): string {
  if (agent === "codex") return "Codex";
  if (agent === "cursor") return "Cursor";
  if (agent === "gemini") return "Gemini CLI";
  if (agent === "antigravity-ide") return "Antigravity IDE";
  if (agent === "antigravity-cli") return "Antigravity CLI";
  if (agent === "antigravity") return "Antigravity";
  return agent;
}

function nativeMcpLabel(agent: string): string {
  return agentDisplayName(agent);
}

function nativeSourceStatus(source: WorkspaceSnapshot["global"]["nativeMcp"][number]): string {
  if (source.state === "parsed") return source.serverCount ? `${source.serverCount} 个已解析` : "无 MCP 定义";
  if (source.state === "unverified") return "已发现 · 待核实";
  if (source.state === "missing") return "未找到";
  if (source.state === "unavailable") return "无已核实路径";
  if (source.state === "empty") return "空文件";
  if (source.state === "invalid") return "格式无效";
  if (source.state === "oversized") return "超过扫描上限";
  return "无法安全读取";
}

function CopyableReference({ value }: { value: string }) {
  const [copied, setCopied] = useState(false);
  const handleCopy = useCallback(() => {
    void navigator.clipboard.writeText(value).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    });
  }, [value]);
  return (
    <button
      type="button"
      className={`credential-reference${copied ? " credential-reference--copied" : ""}`}
      onClick={handleCopy}
      title="点击复制"
      aria-label={`复制凭据引用: ${value}`}
    >
      <code>{value}</code>
      <span className="credential-reference-hint">{copied ? "✓ 已复制" : "点击复制"}</span>
    </button>
  );
}
