export type UpdateCheckSource = "automatic" | "manual";

export function shouldAutoOpenUpdateDialog({
  source,
  hasCandidate,
  dismissedThisSession,
}: {
  source: UpdateCheckSource;
  hasCandidate: boolean;
  dismissedThisSession: boolean;
}): boolean {
  return source === "automatic" && hasCandidate && !dismissedThisSession;
}
