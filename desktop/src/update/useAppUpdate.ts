import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useReducer, useRef, useState } from "react";
import { runUpdateCheck, runUpdateInstall } from "./coordinator";
import type { AppUpdateClient } from "./model";
import {
  createInitialUpdateState,
  reduceUpdateState,
  type UpdateState,
} from "./state";

const CHECK_FOR_UPDATES_EVENT = "check-for-updates";

export interface AppUpdateController {
  state: UpdateState;
  visible: boolean;
  checkNow(): Promise<void>;
  installAndRelaunch(): Promise<void>;
  dismiss(): void;
}

export function useAppUpdate({
  client,
  enabled,
  initialVersion,
  hasUnsavedChanges,
}: {
  client: AppUpdateClient | null;
  enabled: boolean;
  initialVersion: string;
  hasUnsavedChanges: boolean;
}): AppUpdateController {
  const [state, dispatch] = useReducer(
    reduceUpdateState,
    initialVersion,
    createInitialUpdateState,
  );
  const [visible, setVisible] = useState(false);
  const startedRef = useRef(false);
  const checkingRef = useRef(false);

  const check = useCallback(
    async (manual: boolean): Promise<void> => {
      if (!enabled || !client || checkingRef.current) return;
      checkingRef.current = true;
      if (manual) setVisible(true);
      const result = await runUpdateCheck(client, dispatch);
      checkingRef.current = false;
      if (result.candidate) setVisible(true);
      if (manual && result.error) setVisible(true);
    },
    [client, enabled],
  );

  useEffect(() => {
    if (!enabled || !client || startedRef.current) return;
    startedRef.current = true;
    void client
      .getCurrentVersion()
      .then((version) => dispatch({ type: "version-loaded", version }))
      .catch(() => undefined);
    void check(false);
  }, [check, client, enabled]);

  useEffect(() => {
    if (!enabled) return;
    let unlisten: (() => void) | undefined;
    void listen(CHECK_FOR_UPDATES_EVENT, () => void check(true))
      .then((dispose) => {
        unlisten = dispose;
      })
      .catch(() => undefined);
    return () => unlisten?.();
  }, [check, enabled]);

  const installAndRelaunch = useCallback(async (): Promise<void> => {
    if (!client || !state.candidate) return;
    setVisible(true);
    await runUpdateInstall({
      client,
      candidate: state.candidate,
      hasUnsavedChanges,
      dispatch,
    });
  }, [client, hasUnsavedChanges, state.candidate]);

  const dismiss = useCallback(() => {
    dispatch({ type: "dismissed" });
    setVisible(false);
  }, []);

  return {
    state,
    visible,
    checkNow: () => check(true),
    installAndRelaunch,
    dismiss,
  };
}
