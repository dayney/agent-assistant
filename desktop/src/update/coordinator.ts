import type { AppUpdateClient, UpdateCandidate } from "./model";
import type { UpdateEvent } from "./state";

type Dispatch = (event: UpdateEvent) => void;

export async function runUpdateCheck(
  client: AppUpdateClient,
  dispatch: Dispatch,
): Promise<{ candidate: UpdateCandidate | null; error: string | null }> {
  dispatch({ type: "check-started" });
  try {
    const candidate = await client.check();
    dispatch({ type: "check-succeeded", candidate });
    return { candidate, error: null };
  } catch (cause) {
    const error = errorText(cause);
    dispatch({ type: "failed", error });
    return { candidate: null, error };
  }
}

export async function runUpdateInstall({
  client,
  candidate,
  hasUnsavedChanges,
  dispatch,
}: {
  client: AppUpdateClient;
  candidate: UpdateCandidate;
  hasUnsavedChanges: boolean;
  dispatch: Dispatch;
}): Promise<{ installed: boolean; error: string | null }> {
  if (hasUnsavedChanges) {
    const error = "请先保存或放弃未保存的 Rule，再安装更新。";
    dispatch({ type: "failed", error });
    return { installed: false, error };
  }

  dispatch({ type: "download-started" });
  try {
    await client.install(candidate, (progress) => {
      dispatch({ type: "download-progress", progress });
    });
    dispatch({ type: "download-succeeded" });
    await client.relaunch();
    return { installed: true, error: null };
  } catch (cause) {
    const error = errorText(cause);
    dispatch({ type: "failed", error });
    return { installed: false, error };
  }
}

function errorText(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}
