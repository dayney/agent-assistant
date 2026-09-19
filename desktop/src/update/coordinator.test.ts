import { describe, expect, it, vi } from "vitest";
import type { AppUpdateClient, UpdateCandidate } from "./model";
import { runUpdateCheck, runUpdateInstall } from "./coordinator";
import type { UpdateEvent } from "./state";

const candidate: UpdateCandidate = {
  version: "0.2.0",
  notes: "修复规则同步问题",
  publishedAt: "2026-09-16T00:00:00Z",
};

function client(overrides: Partial<AppUpdateClient> = {}): AppUpdateClient {
  return {
    getCurrentVersion: vi.fn().mockResolvedValue("0.1.0"),
    check: vi.fn().mockResolvedValue(candidate),
    install: vi.fn().mockResolvedValue(undefined),
    relaunch: vi.fn().mockResolvedValue(undefined),
    ...overrides,
  };
}

describe("update coordinator", () => {
  it("reports an available update from a background check", async () => {
    const dispatch = vi.fn<(event: UpdateEvent) => void>();

    const result = await runUpdateCheck(client(), dispatch);

    expect(result).toEqual({ candidate, error: null });
    expect(dispatch.mock.calls.map(([event]) => event)).toEqual([
      { type: "check-started" },
      { type: "check-succeeded", candidate },
    ]);
  });

  it("returns an error without throwing when a check fails", async () => {
    const dispatch = vi.fn<(event: UpdateEvent) => void>();
    const error = new Error("network unavailable");

    const result = await runUpdateCheck(
      client({ check: vi.fn().mockRejectedValue(error) }),
      dispatch,
    );

    expect(result).toEqual({ candidate: null, error: "network unavailable" });
    expect(dispatch).toHaveBeenLastCalledWith({
      type: "failed",
      error: "network unavailable",
    });
  });

  it("blocks installation while a Rule has unsaved changes", async () => {
    const updateClient = client();
    const dispatch = vi.fn<(event: UpdateEvent) => void>();

    const result = await runUpdateInstall({
      client: updateClient,
      candidate,
      hasUnsavedChanges: true,
      dispatch,
    });

    expect(result).toEqual({
      installed: false,
      error: "请先保存或放弃未保存的 Rule，再安装更新。",
    });
    expect(updateClient.install).not.toHaveBeenCalled();
    expect(updateClient.relaunch).not.toHaveBeenCalled();
  });

  it("installs the verified update, reports progress, and relaunches", async () => {
    const install = vi.fn(
      async (
        _candidate: UpdateCandidate,
        onProgress: (progress: {
          downloadedBytes: number;
          totalBytes: number | null;
        }) => void,
      ) => {
        onProgress({ downloadedBytes: 50, totalBytes: 100 });
      },
    );
    const updateClient = client({ install });
    const dispatch = vi.fn<(event: UpdateEvent) => void>();

    const result = await runUpdateInstall({
      client: updateClient,
      candidate,
      hasUnsavedChanges: false,
      dispatch,
    });

    expect(result).toEqual({ installed: true, error: null });
    expect(dispatch.mock.calls.map(([event]) => event)).toEqual([
      { type: "download-started" },
      {
        type: "download-progress",
        progress: { downloadedBytes: 50, totalBytes: 100 },
      },
      { type: "download-succeeded" },
    ]);
    expect(updateClient.relaunch).toHaveBeenCalledOnce();
  });
});
