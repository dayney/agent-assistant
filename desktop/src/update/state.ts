import type {
  UpdateCandidate,
  UpdateProgress,
  UpdateState,
} from "./model";

export type { UpdateCandidate, UpdateProgress, UpdateState } from "./model";

export type UpdateEvent =
  | { type: "version-loaded"; version: string }
  | { type: "check-started" }
  | { type: "check-succeeded"; candidate: UpdateCandidate | null }
  | { type: "download-started" }
  | { type: "download-progress"; progress: UpdateProgress }
  | { type: "download-succeeded" }
  | { type: "dismissed" }
  | { type: "failed"; error: string };

export function createInitialUpdateState(currentVersion: string): UpdateState {
  return {
    status: "idle",
    currentVersion,
    candidate: null,
    progress: null,
    error: null,
  };
}

export function reduceUpdateState(
  state: UpdateState,
  event: UpdateEvent,
): UpdateState {
  switch (event.type) {
    case "version-loaded":
      return { ...state, currentVersion: event.version };
    case "check-started":
      return { ...state, status: "checking", error: null, progress: null };
    case "check-succeeded":
      return {
        ...state,
        status: event.candidate ? "available" : "up-to-date",
        candidate: event.candidate,
        progress: null,
        error: null,
      };
    case "download-started":
      return { ...state, status: "downloading", progress: null, error: null };
    case "download-progress":
      return { ...state, status: "downloading", progress: event.progress };
    case "download-succeeded":
      return { ...state, status: "ready", progress: null, error: null };
    case "dismissed":
      return { ...state, status: "dismissed", error: null };
    case "failed":
      return { ...state, status: "error", error: event.error };
  }
}
