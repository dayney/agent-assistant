import type {
  AppUpdateClient,
  UpdateCandidate,
  UpdateProgress,
} from "./model";

export type TauriDownloadEvent =
  | { event: "Started"; data: { contentLength?: number | null } }
  | { event: "Progress"; data: { chunkLength: number } }
  | { event: "Finished" };

export interface TauriUpdate {
  version: string;
  body?: string | null;
  date?: string | null;
  downloadAndInstall(
    onEvent: (event: TauriDownloadEvent) => void,
  ): Promise<void>;
}

export interface TauriUpdateBridge {
  getCurrentVersion(): Promise<string>;
  check(): Promise<TauriUpdate | null>;
  relaunch?(): Promise<void>;
}

export function createTauriUpdateClient(
  bridge: TauriUpdateBridge,
): AppUpdateClient {
  let checkedUpdate: TauriUpdate | null = null;

  return {
    getCurrentVersion(): Promise<string> {
      return bridge.getCurrentVersion();
    },

    async check(): Promise<UpdateCandidate | null> {
      checkedUpdate = await bridge.check();
      if (!checkedUpdate) return null;

      return {
        version: checkedUpdate.version,
        notes: checkedUpdate.body ?? "",
        publishedAt: checkedUpdate.date ?? null,
      };
    },

    async install(
      candidate: UpdateCandidate,
      onProgress: (progress: UpdateProgress) => void,
    ): Promise<void> {
      if (!checkedUpdate || checkedUpdate.version !== candidate.version) {
        throw new Error("没有可安装的已校验更新，请重新检查更新");
      }

      let downloadedBytes = 0;
      let totalBytes: number | null = null;
      await checkedUpdate.downloadAndInstall((event) => {
        if (event.event === "Started") {
          totalBytes = event.data.contentLength ?? null;
          onProgress({ downloadedBytes, totalBytes });
          return;
        }
        if (event.event === "Progress") {
          downloadedBytes += event.data.chunkLength;
          onProgress({ downloadedBytes, totalBytes });
        }
      });
    },

    async relaunch(): Promise<void> {
      if (!bridge.relaunch) {
        throw new Error("当前运行环境不支持重新启动应用");
      }
      await bridge.relaunch();
    },
  };
}
