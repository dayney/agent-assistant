import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { RefreshCw } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { RuleConflictSheet } from "../components/RuleConflictSheet";
import type {
  RuleRequest,
  RuleScope,
  RuleTarget,
  RuleWorkspace,
} from "../core/model";
import {
  blockedRuleTargets,
  createRulePreviewKey,
  deriveRuleCommandState,
  toggleAgentSelection,
} from "../core/rule-state";
import type { GlobalViewProps } from "./GlobalView";

type PendingScopeChange = { scope: RuleScope; projectPath: string };
type PendingTransition =
  | { kind: "scope"; next: PendingScopeChange }
  | { kind: "projects" };

export function RulesView({
  snapshot,
  client,
  onSnapshotRefresh,
  onRuleDirtyChange,
  onNavigateToProjects,
}: Omit<GlobalViewProps, "view">) {
  const [scope, setScope] = useState<RuleScope>("global");
  const [projectPath, setProjectPath] = useState(
    snapshot.projects[0]?.path ?? "",
  );
  const [workspace, setWorkspace] = useState<RuleWorkspace | null>(null);
  const [body, setBody] = useState("");
  const [selectedAgents, setSelectedAgents] = useState<string[]>([]);
  const [previewKey, setPreviewKey] = useState<string | null>(null);
  const [conflict, setConflict] = useState<RuleTarget | null>(null);
  const [backupConfirmation, setBackupConfirmation] = useState(false);
  const [pendingTransition, setPendingTransition] =
    useState<PendingTransition | null>(null);
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [inspectorVisible, setInspectorVisible] = useState(true);
  const [nativePreview, setNativePreview] = useState<{
    agent: string;
    body: string;
  } | null>(null);
  const scopeDialogRef = useRef<HTMLDialogElement>(null);
  const scopeInvokerRef = useRef<HTMLElement | null>(null);
  const conflictInvokerRef = useRef<HTMLButtonElement | null>(null);
  const commandHandlersRef = useRef<Record<string, () => void>>({});
  const request = useMemo<RuleRequest>(
    () => ({
      scope,
      projectPath: scope === "project" ? projectPath : undefined,
    }),
    [scope, projectPath],
  );
  const commandState = deriveRuleCommandState({
    scope,
    projectPath: scope === "project" ? projectPath : undefined,
    body,
    savedBody: workspace?.document.body ?? "",
    selectedAgents,
    previewKey,
    targets: workspace?.targets ?? [],
    busy,
  });
  const blockedTargets = blockedRuleTargets(workspace?.targets ?? []);

  useEffect(() => {
    onRuleDirtyChange?.(commandState.dirty);
  }, [commandState.dirty, onRuleDirtyChange]);

  useEffect(
    () => () => {
      onRuleDirtyChange?.(false);
    },
    [onRuleDirtyChange],
  );

  useEffect(() => {
    let cancelled = false;
    setBusy(true);
    setError("");
    setPreviewKey(null);
    client
      .getRules(request)
      .then((next) => {
        if (cancelled) return;
        setWorkspace(next);
        setBody(next.document.body);
        setSelectedAgents(next.document.agents);
      })
      .catch((cause: unknown) => {
        if (!cancelled) setError(errorText(cause));
      })
      .finally(() => {
        if (!cancelled) setBusy(false);
      });
    return () => {
      cancelled = true;
    };
  }, [client, request]);

  useEffect(() => {
    const dialog = scopeDialogRef.current;
    if (pendingTransition && !dialog?.open) dialog?.showModal();
    if (!pendingTransition && dialog?.open) dialog.close();
  }, [pendingTransition]);

  const run = useCallback(
    async (action: () => Promise<void>): Promise<void> => {
      setBusy(true);
      setError("");
      setMessage("");
      try {
        await action();
      } catch (cause) {
        setError(errorText(cause));
      } finally {
        setBusy(false);
      }
    },
    [],
  );

  const saveMother = useCallback(async (): Promise<void> => {
    if (!body.trim()) throw new Error("母版 Rule 不能为空。");
    if (selectedAgents.length === 0)
      throw new Error("至少选择一个目标 Agent。");
    const document = await client.saveRule({
      ...request,
      agents: selectedAgents,
      body,
    });
    setWorkspace((current) => (current ? { ...current, document } : current));
    setBody(document.body);
    setSelectedAgents(document.agents);
    setPreviewKey(null);
    await onSnapshotRefresh();
  }, [body, client, onSnapshotRefresh, request, selectedAgents]);

  const handleSave = useCallback(() => {
    if (!commandState.canSave) return;
    void run(async () => {
      await saveMother();
      setMessage("母版 Rule 已保存。");
    });
  }, [commandState.canSave, run, saveMother]);

  const handleNativePreview = useCallback(
    (agent: string, path: string) => {
      void run(async () => {
        const nativeBody = await client.previewNativeRule(agent, path);
        setNativePreview({ agent, body: nativeBody });
      });
    },
    [client, run],
  );

  const handleNativeRescan = useCallback(() => {
    void run(async () => {
      await onSnapshotRefresh();
      setNativePreview(null);
      setMessage("已重新扫描本机规则。");
    });
  }, [onSnapshotRefresh, run]);

  const handlePreview = useCallback(() => {
    if (!commandState.canPreview) return;
    void run(async () => {
      const next = await client.getRules({
        ...request,
        agents: selectedAgents,
      });
      setWorkspace(next);
      setPreviewKey(createRulePreviewKey({ ...request, body, selectedAgents }));
      setMessage(
        next.blocked
          ? "发现原生修改。请从目标列表审查差异。"
          : "预览完成，可以安全同步。",
      );
    });
  }, [body, client, commandState.canPreview, request, run, selectedAgents]);

  const closeConflict = useCallback(() => {
    setConflict(null);
    setBackupConfirmation(false);
    queueMicrotask(() => conflictInvokerRef.current?.focus());
  }, []);

  const openConflict = useCallback(
    (target: RuleTarget, invoker?: HTMLButtonElement) => {
      conflictInvokerRef.current = invoker ?? null;
      setConflict(target);
      setBackupConfirmation(false);
    },
    [],
  );

  const handleSync = useCallback(() => {
    if (!commandState.canSync) return;
    void run(async () => {
      const result = await client.syncRules({
        ...request,
        agents: selectedAgents,
      });
      setWorkspace(result.preview);
      setPreviewKey(null);
      if (!result.applied) {
        const nextConflict =
          blockedRuleTargets(result.preview.targets)[0] ?? null;
        if (nextConflict) openConflict(nextConflict);
        setMessage("同步已停止，请先处理原生文件差异。");
        return;
      }
      setMessage("Rule 已同步到所选 Agent。");
    });
  }, [
    client,
    commandState.canSync,
    openConflict,
    request,
    run,
    selectedAgents,
  ]);

  const handleImportNative = useCallback(() => {
    if (!conflict?.supported) return;
    void run(async () => {
      const document = await client.importNativeRule({
        ...request,
        agents: selectedAgents,
        agent: conflict.agent,
      });
      setWorkspace((current) => (current ? { ...current, document } : current));
      setBody(document.body);
      setSelectedAgents(document.agents);
      setPreviewKey(null);
      closeConflict();
      setMessage(
        `已将 ${conflict.agent} 的原生 Rule 导入母版，请审核后再预览。`,
      );
      await onSnapshotRefresh();
    });
  }, [
    client,
    closeConflict,
    conflict,
    onSnapshotRefresh,
    request,
    run,
    selectedAgents,
  ]);

  const handleBackupOverwrite = useCallback(() => {
    if (!conflict) return;
    void run(async () => {
      const result = await client.syncRules({
        ...request,
        agents: selectedAgents,
        resolution: "backup-overwrite",
      });
      setWorkspace(result.preview);
      setPreviewKey(null);
      closeConflict();
      setMessage(`已备份并覆盖 ${result.backups.length} 个原生文件。`);
    });
  }, [client, closeConflict, conflict, request, run, selectedAgents]);

  const applyScopeChange = useCallback((next: PendingScopeChange) => {
    setScope(next.scope);
    setProjectPath(next.projectPath);
    setPendingTransition(null);
    queueMicrotask(() => scopeInvokerRef.current?.focus());
  }, []);

  const requestScopeChange = useCallback(
    (next: PendingScopeChange, invoker: HTMLElement) => {
      if (next.scope === scope && next.projectPath === projectPath) return;
      scopeInvokerRef.current = invoker;
      if (commandState.dirty) {
        setPendingTransition({ kind: "scope", next });
        return;
      }
      applyScopeChange(next);
    },
    [applyScopeChange, commandState.dirty, projectPath, scope],
  );

  const closeScopeChange = useCallback(() => {
    setPendingTransition(null);
    queueMicrotask(() => scopeInvokerRef.current?.focus());
  }, []);

  const handleScopeSave = useCallback(() => {
    if (!pendingTransition) return;
    void run(async () => {
      await saveMother();
      if (pendingTransition.kind === "projects") {
        setPendingTransition(null);
        onNavigateToProjects();
        return;
      }
      applyScopeChange(pendingTransition.next);
      setMessage("母版 Rule 已保存，并切换到新的范围。");
    });
  }, [
    applyScopeChange,
    onNavigateToProjects,
    pendingTransition,
    run,
    saveMother,
  ]);

  const handleScopeDiscard = useCallback(() => {
    setBody(workspace?.document.body ?? "");
    setSelectedAgents(workspace?.document.agents ?? []);
    setPreviewKey(null);
    if (!pendingTransition) return;
    if (pendingTransition.kind === "projects") {
      setPendingTransition(null);
      onNavigateToProjects();
      return;
    }
    applyScopeChange(pendingTransition.next);
  }, [applyScopeChange, onNavigateToProjects, pendingTransition, workspace]);

  const handleProjectScope = useCallback(
    (invoker: HTMLButtonElement) => {
      if (snapshot.projects.length > 0) {
        requestScopeChange({ scope: "project", projectPath }, invoker);
        return;
      }
      scopeInvokerRef.current = invoker;
      if (commandState.dirty) {
        setPendingTransition({ kind: "projects" });
        return;
      }
      onNavigateToProjects();
    },
    [
      commandState.dirty,
      onNavigateToProjects,
      projectPath,
      requestScopeChange,
      snapshot.projects.length,
    ],
  );

  useEffect(() => {
    commandHandlersRef.current = {
      "rule.save": handleSave,
      "rule.preview": handlePreview,
      "rule.sync": handleSync,
      "rule.import-native": handleImportNative,
      "view.toggle-inspector": () => setInspectorVisible((visible) => !visible),
    };
  }, [handleImportNative, handlePreview, handleSave, handleSync]);

  useEffect(() => {
    if (!isTauri()) return;
    let unlisten: (() => void) | undefined;
    void listen<string>("app-menu-command", (event) =>
      commandHandlersRef.current[event.payload]?.(),
    )
      .then((dispose) => {
        unlisten = dispose;
      })
      .catch(() => undefined);
    return () => {
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    if (!isTauri()) return;
    void invoke("set_rule_menu_state", {
      active: true,
      canSave: commandState.canSave,
      canPreview: commandState.canPreview,
      canSync: commandState.canSync,
      canImportNative: Boolean(conflict?.supported),
      inspectorVisible,
    }).catch(() => undefined);
  }, [
    commandState.canPreview,
    commandState.canSave,
    commandState.canSync,
    conflict?.supported,
    inspectorVisible,
  ]);

  useEffect(() => {
    if (!isTauri()) return;
    return () => {
      void invoke("set_rule_menu_state", { active: false }).catch(
        () => undefined,
      );
    };
  }, []);

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "s") {
        event.preventDefault();
        commandHandlersRef.current["rule.save"]?.();
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  const availableAgents = workspace?.availableAgents ?? [];
  return (
    <div className="rule-workbench">
      <div className="rule-toolbar">
        <div className="segmented" aria-label="Rule 范围">
          <button
            type="button"
            className={scope === "global" ? "is-active" : ""}
            onClick={(event) =>
              requestScopeChange(
                { scope: "global", projectPath },
                event.currentTarget,
              )
            }
          >
            全局母版
          </button>
          <button
            type="button"
            className={scope === "project" ? "is-active" : ""}
            title={
              snapshot.projects.length === 0 ? "先导入项目" : undefined
            }
            onClick={(event) => handleProjectScope(event.currentTarget)}
          >
            项目母版
          </button>
        </div>
        {scope === "project" && (
          <label className="field-inline">
            <span>项目</span>
            <select
              value={projectPath}
              disabled={busy}
              onChange={(event) =>
                requestScopeChange(
                  { scope: "project", projectPath: event.target.value },
                  event.currentTarget,
                )
              }
            >
              {snapshot.projects.map((project) => (
                <option value={project.path} key={project.path}>
                  {project.name}
                </option>
              ))}
            </select>
          </label>
        )}
        <code className="canonical-path">
          {workspace?.document.canonicalPath ?? "读取中…"}
        </code>
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
      {scope === "global" && (
        <section className="native-rule-inventory" aria-labelledby="native-rule-title">
          <div className="section-heading">
            <div>
              <p className="eyebrow">Native Sources</p>
              <h2 id="native-rule-title">原生全局规则</h2>
            </div>
            <div className="native-rule-actions">
              <span className="target-count">
                已发现 {snapshot.global.nativeRules.filter((rule) => rule.state === "present").length}
                /{snapshot.global.nativeRules.length}
              </span>
              {snapshot.mode === "real" && (
                <button type="button" className="button button-quiet" disabled={busy} onClick={handleNativeRescan}>
                  <RefreshCw size={16} aria-hidden="true" /> 重新扫描
                </button>
              )}
            </div>
          </div>
          <p className="native-source-note">
            {snapshot.mode === "demo" ? "以下为演示示例，不是本机扫描结果。仅 Desktop 应用可查看原文。" : "本机发现不等于已导入母版。"}
            Antigravity、Antigravity IDE、Antigravity CLI 与 Gemini CLI 共用同一个全局 Rule 文件。
          </p>
          <div className="native-rule-list">
            {snapshot.global.nativeRules.slice().sort((left, right) =>
              Number(right.state === "present") - Number(left.state === "present"),
            ).map((rule, index) => (
              <div className="native-rule-row" key={`${rule.agent}:${rule.path}:${index}`}>
                <strong>{nativeRuleLabel(rule.agent)}{rule.kind === "auxiliary" ? " · 附加" : ""}</strong>
                <code title={rule.path || rule.reason}>{rule.path || rule.reason}</code>
                <span title={rule.reason}>
                  {rule.state === "present"
                    ? "已发现 · 未导入"
                    : rule.state === "missing"
                      ? "未找到"
                      : rule.state === "directory"
                        ? "目录已发现"
                      : rule.state === "unavailable"
                        ? "本机不可读取"
                        : rule.state === "empty"
                          ? "空文件"
                          : "无法安全读取"}
                </span>
                {rule.state === "present" && snapshot.mode === "real" && (
                  <button
                    type="button"
                    className="button button-secondary"
                    disabled={busy}
                    onClick={() => handleNativePreview(rule.agent, rule.path)}
                  >
                    查看原文
                  </button>
                )}
              </div>
            ))}
          </div>
          {nativePreview && (
            <div className="native-rule-preview">
              <div className="section-heading">
                <h3>{nativeRuleLabel(nativePreview.agent)} · 原文</h3>
                <button type="button" className="button button-secondary" onClick={() => setNativePreview(null)}>关闭</button>
              </div>
              <p>只读预览；尚未导入全局母版。</p>
              <textarea readOnly aria-label={`${nativeRuleLabel(nativePreview.agent)} 原生规则原文`} value={nativePreview.body} />
            </div>
          )}
        </section>
      )}
      <div className="rule-editor-grid">
        <section className="rule-editor-pane">
          <div className="section-heading">
            <div>
              <p className="eyebrow">Mother Rule</p>
              <h2>母版 Rule</h2>
            </div>
            <span
              className={
                commandState.dirty ? "edit-state is-dirty" : "edit-state"
              }
            >
              {commandState.dirty ? "未保存" : "已保存"}
            </span>
          </div>
          <textarea
            className="rule-textarea"
            value={body}
            onChange={(event) => {
              setBody(event.target.value);
              setPreviewKey(null);
            }}
            spellCheck={false}
            aria-label="母版 Rule Markdown"
            disabled={busy}
          />
          {workspace?.document.fragments.length ? (
            <p className="fragment-note">
              保留 fragments：{workspace.document.fragments.join("、")}
            </p>
          ) : null}
          <div className="rule-actions">
            <button
              className={
                commandState.primaryAction === "save"
                  ? "button button-primary"
                  : "button button-secondary"
              }
              type="button"
              disabled={!commandState.canSave}
              onClick={handleSave}
            >
              保存母版
            </button>
            <button
              className={
                commandState.primaryAction === "preview"
                  ? "button button-primary"
                  : "button button-secondary"
              }
              type="button"
              disabled={!commandState.canPreview}
              onClick={handlePreview}
            >
              预览同步
            </button>
            <button
              className={
                commandState.primaryAction === "sync"
                  ? "button button-primary"
                  : "button button-secondary"
              }
              type="button"
              disabled={!commandState.canSync}
              onClick={handleSync}
            >
              同步到 Agent
            </button>
          </div>
        </section>
        {inspectorVisible && (
          <aside className="rule-target-pane">
            <div className="section-heading">
              <div>
                <p className="eyebrow">Targets</p>
                <h2>目标 Agent</h2>
              </div>
              <span className="target-count">{selectedAgents.length}</span>
            </div>
            <div className="agent-selector">
              {availableAgents.map((agent) => (
                <label key={agent}>
                  <input
                    type="checkbox"
                    checked={selectedAgents.includes(agent)}
                    disabled={busy}
                    onChange={(event) => {
                      setSelectedAgents((current) =>
                        toggleAgentSelection(
                          current,
                          agent,
                          event.target.checked,
                        ),
                      );
                      setPreviewKey(null);
                    }}
                  />
                  <span>{agent}</span>
                </label>
              ))}
            </div>
            <div className="target-preview">
              {workspace?.targets.map((target) => (
                <button
                  type="button"
                  className={
                    target.blocked ? "target-row is-blocked" : "target-row"
                  }
                  key={target.agent}
                  onClick={(event) =>
                    target.blocked && openConflict(target, event.currentTarget)
                  }
                >
                  <span className="target-agent">{target.agent}</span>
                  <span className={`target-status status-${target.status}`}>
                    {statusLabel(target.status)}
                  </span>
                  <code>{target.path || target.reason || "无目标路径"}</code>
                </button>
              ))}
            </div>
          </aside>
        )}
      </div>
      <dialog
        ref={scopeDialogRef}
        className="conflict-dialog"
        aria-labelledby="scope-change-title"
        onClose={closeScopeChange}
      >
        <div className="dialog-heading">
          <div>
            <p className="eyebrow">Unsaved Rule</p>
            <h2 id="scope-change-title">先处理未保存的修改</h2>
          </div>
        </div>
        <p className="dialog-copy">
          保存会写入当前范围，然后
          {pendingTransition?.kind === "projects"
            ? "前往项目导入页"
            : "切换到所选范围"}
          。丢弃会放弃当前编辑内容。
        </p>
        <div className="dialog-actions">
          <button
            className="button button-secondary"
            type="button"
            disabled={busy}
            onClick={closeScopeChange}
          >
            取消
          </button>
          <button
            className="button button-secondary"
            type="button"
            disabled={busy}
            onClick={handleScopeDiscard}
          >
            丢弃
          </button>
          <button
            className="button button-primary"
            type="button"
            disabled={busy || !commandState.canSave}
            onClick={handleScopeSave}
          >
            保存
          </button>
        </div>
      </dialog>
      <RuleConflictSheet
        conflict={conflict}
        blockedTargets={blockedTargets}
        backupConfirmation={backupConfirmation}
        busy={busy}
        onClose={closeConflict}
        onImportNative={handleImportNative}
        onContinueToBackup={() => setBackupConfirmation(true)}
        onBackupOverwrite={handleBackupOverwrite}
      />
    </div>
  );
}

function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

function nativeRuleLabel(agent: string): string {
  const labels: Record<string, string> = {
    antigravity: "Antigravity",
    "antigravity-ide": "Antigravity IDE",
    "antigravity-cli": "Antigravity CLI",
    "cursor-account": "Cursor 账户规则",
    "cursor-local": "Cursor 本机规则",
    gemini: "Gemini CLI",
  };
  return labels[agent] ?? agent;
}

function statusLabel(status: string): string {
  const labels: Record<string, string> = {
    clean: "已同步",
    pending: "待更新",
    new: "新建",
    converged: "已收敛",
    drift: "原生修改",
    conflict: "双向冲突",
    "foreign-collision": "未托管文件",
    "native-only": "仅原生存在",
    unsupported: "不支持",
    empty: "无母版",
  };
  return labels[status] ?? status;
}

function errorText(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}
