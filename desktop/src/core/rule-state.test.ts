import { describe, expect, it } from "vitest";
import type { RuleTarget } from "./model";
import {
  blockedRuleTargets,
  createRulePreviewKey,
  deriveRuleCommandState,
  toggleAgentSelection,
} from "./rule-state";

const safeTarget: RuleTarget = {
  agent: "codex",
  path: "/tmp/AGENTS.md",
  supported: true,
  status: "clean",
  blocked: false,
  willWrite: true,
};

describe("Rule UI state", () => {
  it("returns only targets that require an explicit resolution", () => {
    const targets: RuleTarget[] = [
      {
        agent: "codex",
        path: "/tmp/AGENTS.md",
        supported: true,
        status: "clean",
        blocked: false,
        willWrite: false,
      },
      {
        agent: "claude",
        path: "/tmp/CLAUDE.md",
        supported: true,
        status: "drift",
        blocked: true,
        willWrite: true,
        diff: "-old\n+new\n",
      },
    ];

    expect(blockedRuleTargets(targets).map((target) => target.agent)).toEqual([
      "claude",
    ]);
  });

  it("keeps Agent selection unique and deterministic", () => {
    expect(toggleAgentSelection(["gemini", "codex"], "cursor", true)).toEqual([
      "codex",
      "cursor",
      "gemini",
    ]);
    expect(toggleAgentSelection(["codex", "cursor"], "codex", false)).toEqual([
      "cursor",
    ]);
  });

  it("requires an explicit fresh preview before synchronization", () => {
    const state = deriveRuleCommandState({
      body: "# Rule\n",
      savedBody: "# Rule\n",
      selectedAgents: ["codex"],
      scope: "global",
      previewKey: null,
      targets: [safeTarget],
      busy: false,
    });
    expect(state.canPreview).toBe(true);
    expect(state.canSync).toBe(false);
  });

  it("invalidates preview when target selection changes", () => {
    const previewKey = createRulePreviewKey({
      scope: "global",
      body: "# Rule\n",
      selectedAgents: ["codex"],
    });
    const state = deriveRuleCommandState({
      body: "# Rule\n",
      savedBody: "# Rule\n",
      selectedAgents: ["claude", "codex"],
      scope: "global",
      previewKey,
      targets: [safeTarget],
      busy: false,
    });
    expect(state.previewFresh).toBe(false);
    expect(state.canSync).toBe(false);
  });

  it("invalidates preview when the project path changes", () => {
    const previewKey = createRulePreviewKey({
      scope: "project",
      projectPath: "/projects/alpha",
      body: "# Rule\n",
      selectedAgents: ["codex"],
    });
    const state = deriveRuleCommandState({
      scope: "project",
      projectPath: "/projects/beta",
      body: "# Rule\n",
      savedBody: "# Rule\n",
      selectedAgents: ["codex"],
      previewKey,
      targets: [safeTarget],
      busy: false,
    });
    expect(state.previewFresh).toBe(false);
    expect(state.canSync).toBe(false);
  });

  it("keeps preview keys stable when Agent ordering changes", () => {
    expect(
      createRulePreviewKey({
        scope: "global",
        body: "# Rule\n",
        selectedAgents: ["codex", "claude"],
      }),
    ).toBe(
      createRulePreviewKey({
        scope: "global",
        body: "# Rule\n",
        selectedAgents: ["claude", "codex"],
      }),
    );
  });
});
