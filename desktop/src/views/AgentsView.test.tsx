import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import { createDemoSnapshot } from "../core/demo-client";
import { AgentsView } from "./AgentsView";

describe("AgentsView", () => {
  it("distinguishes configured adapters from three Antigravity products", () => {
    const snapshot = createDemoSnapshot();
    snapshot.agents = [];
    const markup = renderToStaticMarkup(<AgentsView snapshot={snapshot} onRefresh={vi.fn()} />);

    expect(markup).toContain("0 个已配置适配器");
    expect(markup).toContain("未配置适配器");
    expect(markup).toContain("Antigravity IDE");
    expect(markup).toContain("Antigravity CLI");
    expect(markup).toContain("Antigravity</strong>");
    expect(markup).toContain("配置来源，不代表已安装或已同步");
    expect(markup).toContain("重新检测");
  });

  it("summarizes all inventoried products without counting empty or unverified sources", () => {
    const snapshot = createDemoSnapshot();
    snapshot.agents = [];
    snapshot.global.nativeRules = [
      { agent: "cursor-account", kind: "auxiliary", path: "", state: "unavailable", reason: "" },
      { agent: "cursor-local", kind: "auxiliary", path: "~/.cursor/rules", state: "missing", reason: "" },
    ];
    snapshot.global.nativeMcp = [
      { agent: "cursor", path: "~/.cursor/mcp.json", state: "parsed", reason: "", serverCount: 2 },
      { agent: "claude", path: "~/.claude.json", state: "unverified", reason: "", serverCount: 0 },
      { agent: "gemini", path: "~/.gemini/settings.json", state: "parsed", reason: "", serverCount: 0 },
    ];
    snapshot.global.nativeSkills = [
      { agent: "cursor", path: "~/.cursor/mcp.json", state: "present", reason: "", items: [] },
      { agent: "shared", path: "~/.agents/skills", state: "present", reason: "", items: [] },
      { agent: "codex", path: "~/.codex/skills", state: "present", reason: "", items: [
        { name: "empty", path: "~/.codex/skills/empty", state: "empty" },
      ] },
    ];
    snapshot.global.nativeSubagents = [
      { agent: "cursor", path: "~/.cursor/agents", state: "empty", reason: "", items: [] },
    ];
    snapshot.global.nativeWorkflows = [
      { agent: "antigravity-ide", path: "~/.gemini/antigravity/global_workflows", state: "present", reason: "", items: [
        { name: "empty.md", path: "~/.gemini/antigravity/global_workflows/empty.md", state: "empty" },
      ] },
    ];

    const markup = renderToStaticMarkup(<AgentsView snapshot={snapshot} onRefresh={vi.fn()} />);

    expect(markup).toContain("本机产品来源");
    expect(markup).toMatch(/<strong>Cursor<\/strong><span>发现 1 处配置来源<\/span>/);
    expect(markup).toMatch(/<strong>Claude<\/strong><span>1 处待核实<\/span>/);
    expect(markup).toMatch(/<strong>Codex<\/strong><span>未发现已核实配置<\/span>/);
    expect(markup).toMatch(/<strong>Gemini CLI<\/strong><span>未发现已核实配置<\/span>/);
    expect(markup).toMatch(/<strong>Antigravity IDE<\/strong><span>未发现已核实配置<\/span>/);
    expect(markup).not.toContain("Cursor Local</strong>");
    expect(markup).not.toContain("Shared</strong>");
    expect(markup).toContain("0 个已配置适配器");
  });
});
