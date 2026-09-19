import { describe, expect, it } from "vitest";
import {
  createInitialUpdateState,
  reduceUpdateState,
  type UpdateCandidate,
} from "./state";

const candidate: UpdateCandidate = {
  version: "0.2.0",
  notes: "修复规则同步问题",
  publishedAt: "2026-09-16T00:00:00Z",
};

describe("update state", () => {
  it("uses the installed version reported by Tauri", () => {
    const state = reduceUpdateState(createInitialUpdateState("0.1.0"), {
      type: "version-loaded",
      version: "0.1.1",
    });

    expect(state.currentVersion).toBe("0.1.1");
  });

  it("moves a background check into an available state", () => {
    const checking = reduceUpdateState(createInitialUpdateState("0.1.0"), {
      type: "check-started",
    });

    expect(checking.status).toBe("checking");
    expect(
      reduceUpdateState(checking, { type: "check-succeeded", candidate }),
    ).toEqual({
      status: "available",
      currentVersion: "0.1.0",
      candidate,
      progress: null,
      error: null,
    });
  });

  it("records an explicit no-update result", () => {
    const state = reduceUpdateState(
      reduceUpdateState(createInitialUpdateState("0.1.0"), {
        type: "check-started",
      }),
      { type: "check-succeeded", candidate: null },
    );

    expect(state.status).toBe("up-to-date");
    expect(state.candidate).toBeNull();
  });

  it("tracks download progress and preserves the candidate", () => {
    let state = reduceUpdateState(createInitialUpdateState("0.1.0"), {
      type: "check-succeeded",
      candidate,
    });
    state = reduceUpdateState(state, { type: "download-started" });
    state = reduceUpdateState(state, {
      type: "download-progress",
      progress: { downloadedBytes: 512, totalBytes: 1024 },
    });

    expect(state.status).toBe("downloading");
    expect(state.progress).toEqual({ downloadedBytes: 512, totalBytes: 1024 });
    expect(state.candidate).toEqual(candidate);
  });

  it("marks a downloaded update ready to relaunch", () => {
    let state = reduceUpdateState(createInitialUpdateState("0.1.0"), {
      type: "check-succeeded",
      candidate,
    });
    state = reduceUpdateState(state, { type: "download-started" });

    expect(reduceUpdateState(state, { type: "download-succeeded" }).status).toBe(
      "ready",
    );
  });

  it("keeps the current app and candidate visible after an error", () => {
    const state = reduceUpdateState(
      reduceUpdateState(createInitialUpdateState("0.1.0"), {
        type: "check-succeeded",
        candidate,
      }),
      { type: "failed", error: "无法连接更新服务器" },
    );

    expect(state).toMatchObject({
      status: "error",
      currentVersion: "0.1.0",
      candidate,
      error: "无法连接更新服务器",
    });
  });

  it("dismisses an available update without losing its metadata", () => {
    const state = reduceUpdateState(
      reduceUpdateState(createInitialUpdateState("0.1.0"), {
        type: "check-succeeded",
        candidate,
      }),
      { type: "dismissed" },
    );

    expect(state.status).toBe("dismissed");
    expect(state.candidate).toEqual(candidate);
  });
});
