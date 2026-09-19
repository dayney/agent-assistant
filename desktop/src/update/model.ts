export type UpdateStatus =
  | "idle"
  | "checking"
  | "available"
  | "downloading"
  | "ready"
  | "up-to-date"
  | "dismissed"
  | "error";

export interface UpdateCandidate {
  version: string;
  notes: string;
  publishedAt: string | null;
}

export interface UpdateProgress {
  downloadedBytes: number;
  totalBytes: number | null;
}

export interface UpdateState {
  status: UpdateStatus;
  currentVersion: string;
  candidate: UpdateCandidate | null;
  progress: UpdateProgress | null;
  error: string | null;
}

export interface AppUpdateClient {
  getCurrentVersion(): Promise<string>;
  check(): Promise<UpdateCandidate | null>;
  install(
    candidate: UpdateCandidate,
    onProgress: (progress: UpdateProgress) => void,
  ): Promise<void>;
  relaunch(): Promise<void>;
}
