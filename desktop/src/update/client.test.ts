import { describe, expect, it, vi } from "vitest";
import {
  createTauriUpdateClient,
  type TauriDownloadEvent,
  type TauriUpdateBridge,
} from "./client";

describe("Tauri update client", () => {
  it("reads the installed version from Tauri app metadata", async () => {
    const bridge: TauriUpdateBridge = {
      getCurrentVersion: vi.fn().mockResolvedValue("0.1.0"),
      check: vi.fn(),
    };

    await expect(
      createTauriUpdateClient(bridge).getCurrentVersion(),
    ).resolves.toBe("0.1.0");
  });

  it("maps an available update into the app update model", async () => {
    const bridge: TauriUpdateBridge = {
      getCurrentVersion: vi.fn(),
      check: vi.fn().mockResolvedValue({
        version: "0.2.0",
        body: "修复规则同步问题",
        date: "2026-09-16T00:00:00Z",
        downloadAndInstall: vi.fn(),
      }),
    };

    await expect(createTauriUpdateClient(bridge).check()).resolves.toEqual({
      version: "0.2.0",
      notes: "修复规则同步问题",
      publishedAt: "2026-09-16T00:00:00Z",
    });
  });

  it("returns null when the updater has no newer release", async () => {
    const bridge: TauriUpdateBridge = {
      getCurrentVersion: vi.fn(),
      check: vi.fn().mockResolvedValue(null),
    };

    await expect(createTauriUpdateClient(bridge).check()).resolves.toBeNull();
  });

  it("reports cumulative download progress and installs the checked update", async () => {
    const downloadAndInstall = vi.fn(
      async (onEvent: (event: TauriDownloadEvent) => void) => {
        onEvent({ event: "Started", data: { contentLength: 1024 } });
        onEvent({ event: "Progress", data: { chunkLength: 256 } });
        onEvent({ event: "Progress", data: { chunkLength: 128 } });
        onEvent({ event: "Finished" });
      },
    );
    const bridge: TauriUpdateBridge = {
      getCurrentVersion: vi.fn(),
      check: vi.fn().mockResolvedValue({
        version: "0.2.0",
        body: "",
        date: null,
        downloadAndInstall,
      }),
    };
    const client = createTauriUpdateClient(bridge);
    const progress: Array<{ downloadedBytes: number; totalBytes: number | null }> = [];

    await client.check();
    await client.install(
      {
        version: "0.2.0",
        notes: "",
        publishedAt: null,
      },
      (value) => progress.push(value),
    );

    expect(downloadAndInstall).toHaveBeenCalledOnce();
    expect(progress).toEqual([
      { downloadedBytes: 0, totalBytes: 1024 },
      { downloadedBytes: 256, totalBytes: 1024 },
      { downloadedBytes: 384, totalBytes: 1024 },
    ]);
  });

  it("preserves updater failures for the UI to render", async () => {
    const bridge: TauriUpdateBridge = {
      getCurrentVersion: vi.fn(),
      check: vi.fn().mockRejectedValue(new Error("network unavailable")),
    };

    await expect(createTauriUpdateClient(bridge).check()).rejects.toThrow(
      "network unavailable",
    );
  });
});
