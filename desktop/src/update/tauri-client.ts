import { getVersion } from "@tauri-apps/api/app";
import { relaunch } from "@tauri-apps/plugin-process";
import { check } from "@tauri-apps/plugin-updater";
import { createTauriUpdateClient } from "./client";
import type { AppUpdateClient } from "./model";

export function createDesktopUpdateClient(): AppUpdateClient {
  return createTauriUpdateClient({
    getCurrentVersion: getVersion,
    check,
    relaunch,
  });
}
