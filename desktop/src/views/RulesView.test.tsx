import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import { DemoClient, createDemoSnapshot } from "../core/demo-client";
import { RulesView } from "./RulesView";

describe("RulesView", () => {
  it("keeps project scope actionable when no project has been imported", () => {
    const markup = renderToStaticMarkup(
      <RulesView
        snapshot={createDemoSnapshot()}
        client={new DemoClient()}
        onSnapshotRefresh={vi.fn()}
        onNavigateToProjects={vi.fn()}
      />,
    );
    const projectScopeButton = markup.match(
      /<button[^>]*>项目母版<\/button>/,
    )?.[0];

    expect(projectScopeButton).toBeDefined();
    expect(projectScopeButton).not.toContain("disabled");
  });

  it("shows detected, missing, and unavailable native global rules", () => {
    const snapshot = createDemoSnapshot();
    snapshot.global.nativeRules = [
      { agent: "codex", kind: "primary", path: "~/.codex/AGENTS.md", state: "present", reason: "" },
      { agent: "gemini", kind: "primary", path: "~/.gemini/GEMINI.md", state: "missing", reason: "" },
      { agent: "cursor", kind: "primary", path: "", state: "unavailable", reason: "无全局文件目标" },
    ];
    const markup = renderToStaticMarkup(
      <RulesView
        snapshot={snapshot}
        client={new DemoClient()}
        onSnapshotRefresh={vi.fn()}
        onNavigateToProjects={vi.fn()}
      />,
    );

    expect(markup).toContain("原生全局规则");
    expect(markup).toContain("~/.codex/AGENTS.md");
    expect(markup).toContain("~/.gemini/GEMINI.md");
    expect(markup).toContain("无全局文件目标");
  });

  it("places distinct Antigravity and Cursor sources before the mother editor", () => {
    const snapshot = createDemoSnapshot();
    snapshot.mode = "real";
    snapshot.global.nativeRules = [
      { agent: "antigravity-ide", kind: "auxiliary", path: "~/.gemini/GEMINI.md", state: "present", reason: "共享全局规则" },
      { agent: "antigravity-cli", kind: "auxiliary", path: "~/.gemini/GEMINI.md", state: "present", reason: "共享全局规则" },
      { agent: "antigravity", kind: "auxiliary", path: "~/.gemini/GEMINI.md", state: "present", reason: "共享全局规则" },
      { agent: "cursor-account", kind: "auxiliary", path: "", state: "unavailable", reason: "Cursor 账户 User Rules 未提供可核实的本机文件" },
    ];
    const markup = renderToStaticMarkup(
      <RulesView
        snapshot={snapshot}
        client={new DemoClient()}
        onSnapshotRefresh={vi.fn()}
        onNavigateToProjects={vi.fn()}
      />,
    );

    expect(markup).toContain("Antigravity IDE");
    expect(markup).toContain("Antigravity CLI");
    expect(markup).toContain("Antigravity · 附加");
    expect(markup).toContain("Cursor 账户 User Rules 未提供可核实的本机文件");
    expect(markup.indexOf("原生全局规则")).toBeLessThan(markup.indexOf("母版 Rule"));
    expect(markup).toContain("查看原文");
  });

  it("puts present native sources before missing sources without dropping either", () => {
    const snapshot = createDemoSnapshot();
    snapshot.global.nativeRules = [
      { agent: "missing-first", kind: "primary", path: "~/missing.md", state: "missing", reason: "" },
      { agent: "available-later", kind: "primary", path: "~/present.md", state: "present", reason: "" },
    ];
    const markup = renderToStaticMarkup(
      <RulesView snapshot={snapshot} client={new DemoClient()} onSnapshotRefresh={vi.fn()} onNavigateToProjects={vi.fn()} />,
    );
    expect(markup.indexOf("~/present.md")).toBeLessThan(markup.indexOf("~/missing.md"));
  });

  it("explains that demo sources cannot be previewed instead of showing a dead button", () => {
    const markup = renderToStaticMarkup(
      <RulesView snapshot={createDemoSnapshot()} client={new DemoClient()} onSnapshotRefresh={vi.fn()} onNavigateToProjects={vi.fn()} />,
    );

    expect(markup).toContain("仅 Desktop 应用可查看原文");
    expect(markup).not.toMatch(/<button[^>]*>查看原文<\/button>/);
  });

  it("offers preview and rescan controls for real native Rule sources", () => {
    const snapshot = createDemoSnapshot();
    snapshot.mode = "real";
    const markup = renderToStaticMarkup(
      <RulesView snapshot={snapshot} client={new DemoClient()} onSnapshotRefresh={vi.fn()} onNavigateToProjects={vi.fn()} />,
    );

    expect(markup).toMatch(/<button[^>]*class="button button-quiet"[^>]*>.*?重新扫描<\/button>/);
    expect(markup).toMatch(/<button(?![^>]*disabled)[^>]*>查看原文<\/button>/);
  });
});
