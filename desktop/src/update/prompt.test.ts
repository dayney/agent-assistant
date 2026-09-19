import { describe, expect, it } from "vitest";
import { shouldAutoOpenUpdateDialog } from "./prompt";

describe("automatic update prompt", () => {
  it("opens for a candidate found by the startup check", () => {
    expect(
      shouldAutoOpenUpdateDialog({
        source: "automatic",
        hasCandidate: true,
        dismissedThisSession: false,
      }),
    ).toBe(true);
  });

  it("does not reopen after the user chose later in this session", () => {
    expect(
      shouldAutoOpenUpdateDialog({
        source: "automatic",
        hasCandidate: true,
        dismissedThisSession: true,
      }),
    ).toBe(false);
  });

  it("keeps manual checks in the explicit banner flow", () => {
    expect(
      shouldAutoOpenUpdateDialog({
        source: "manual",
        hasCandidate: true,
        dismissedThisSession: false,
      }),
    ).toBe(false);
  });

  it("does not open when no update was found", () => {
    expect(
      shouldAutoOpenUpdateDialog({
        source: "automatic",
        hasCandidate: false,
        dismissedThisSession: false,
      }),
    ).toBe(false);
  });
});
