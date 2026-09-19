import { useRef } from "react";
import type { UpdateState } from "../update/model";

export function UpdateNotice({
  state,
  hasUnsavedChanges,
  onCheck,
  onInstall,
  onDismiss,
}: {
  state: UpdateState;
  hasUnsavedChanges: boolean;
  onCheck: () => Promise<void>;
  onInstall: () => Promise<void>;
  onDismiss: () => void;
}) {
  const dialogRef = useRef<HTMLDialogElement>(null);
  const percentage = progressPercentage(state);
  const canDismiss = state.status !== "downloading" && state.status !== "ready";

  return (
    <>
      <div className={`update-banner is-${state.status}`} role="status">
        <span className="update-indicator" aria-hidden="true">
          {state.status === "error" ? "!" : "↑"}
        </span>
        <div className="update-copy">
          <strong>{updateTitle(state)}</strong>
          <span>{updateDetail(state, percentage)}</span>
          {state.status === "downloading" && (
            <progress
              aria-label="更新下载进度"
              value={percentage ?? undefined}
              max={100}
            />
          )}
        </div>
        {state.status === "available" && state.candidate && (
          <button
            className="button button-primary"
            type="button"
            onClick={() => dialogRef.current?.showModal()}
          >
            查看更新
          </button>
        )}
        {(state.status === "up-to-date" || state.status === "error") && (
          <button
            className="button button-quiet"
            type="button"
            onClick={() => void onCheck()}
          >
            重新检查
          </button>
        )}
        {canDismiss && (
          <button
            className="icon-button"
            type="button"
            aria-label="关闭更新提示"
            title="关闭"
            onClick={onDismiss}
          >
            ×
          </button>
        )}
      </div>

      {state.candidate && (
        <dialog
          ref={dialogRef}
          className="update-dialog"
          aria-labelledby="update-dialog-title"
        >
          <div className="dialog-heading">
            <div>
              <p className="eyebrow">Desktop Update</p>
              <h2 id="update-dialog-title">
                更新到 v{state.candidate.version}
              </h2>
            </div>
            <button
              className="icon-button"
              type="button"
              aria-label="关闭更新窗口"
              title="关闭"
              onClick={() => dialogRef.current?.close()}
            >
              ×
            </button>
          </div>
          <div className="update-release-notes">
            {state.candidate.notes || "此版本没有提供发布说明。"}
          </div>
          {hasUnsavedChanges && (
            <div className="inline-alert is-error" role="alert">
              请先保存或放弃未保存的 Rule，再安装更新。
            </div>
          )}
          <p className="dialog-copy">
            更新包会先完成签名校验，然后安装并重新启动应用。
          </p>
          <div className="dialog-actions">
            <button
              className="button button-secondary"
              type="button"
              onClick={() => dialogRef.current?.close()}
            >
              稍后
            </button>
            <button
              className="button button-primary"
              type="button"
              disabled={hasUnsavedChanges}
              onClick={() => {
                dialogRef.current?.close();
                void onInstall();
              }}
            >
              下载并重启安装
            </button>
          </div>
        </dialog>
      )}
    </>
  );
}

function progressPercentage(state: UpdateState): number | null {
  const progress = state.progress;
  if (!progress?.totalBytes || progress.totalBytes <= 0) return null;
  return Math.min(
    100,
    Math.round((progress.downloadedBytes / progress.totalBytes) * 100),
  );
}

function updateTitle(state: UpdateState): string {
  switch (state.status) {
    case "checking":
      return "正在检查更新";
    case "available":
      return `发现新版本 v${state.candidate?.version ?? ""}`;
    case "downloading":
      return "正在下载并校验更新";
    case "ready":
      return "更新已安装，正在重新启动";
    case "up-to-date":
      return "已是最新版本";
    case "error":
      return "更新检查失败";
    default:
      return "桌面应用更新";
  }
}

function updateDetail(state: UpdateState, percentage: number | null): string {
  switch (state.status) {
    case "checking":
      return `当前版本 v${state.currentVersion}`;
    case "available":
      return `当前版本 v${state.currentVersion}，安装前会再次确认。`;
    case "downloading":
      return percentage === null ? "正在接收更新包…" : `已完成 ${percentage}%`;
    case "ready":
      return "应用将自动恢复到新版本。";
    case "up-to-date":
      return `当前版本 v${state.currentVersion}`;
    case "error":
      return state.error ?? "请稍后重试。";
    default:
      return "";
  }
}
