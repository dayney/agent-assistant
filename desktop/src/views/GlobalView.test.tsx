import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import { DemoClient, createDemoSnapshot } from "../core/demo-client";
import { GlobalView } from "./GlobalView";

describe("GlobalView MCP", () => {
  it("distinguishes parsed servers from unverified and invalid sources", () => {
    const snapshot = createDemoSnapshot();
    snapshot.global.nativeMcp = [
      { agent: "claude", path: "~/.claude.json", state: "unverified", reason: "待核实", serverCount: 0 },
      { agent: "gemini", path: "~/.gemini/settings.json", state: "invalid", reason: "格式错误", serverCount: 0 },
      { agent: "codex", path: "~/.codex/config.toml", state: "parsed", reason: "", serverCount: 2 },
    ];
    const markup = renderToStaticMarkup(
      <GlobalView
        view="mcp"
        snapshot={snapshot}
        client={new DemoClient()}
        onSnapshotRefresh={vi.fn()}
        onNavigateToProjects={vi.fn()}
      />,
    );

    expect(markup).toContain("本机 MCP 来源");
    expect(markup).toContain("母版是唯一编辑源");
    expect(markup).toContain("2 个已解析");
    expect(markup).toContain("已发现 · 待核实");
    expect(markup).toContain("格式无效");
    expect(markup).not.toContain("private-value");
    expect(markup.indexOf("~/.codex/config.toml")).toBeLessThan(markup.indexOf("~/.claude.json"));
  });

  it("deduplicates a native copy of an already canonical MCP", () => {
    const snapshot = createDemoSnapshot();
    const canonical = snapshot.global.mcp.find((item) => item.serverId === "github");
    if (!canonical) throw new Error("demo snapshot must include github");
    snapshot.global.nativeMcpItems = [{
      ...canonical,
      id: "native:github",
      source: "agent",
      scope: "agent",
      category: "agent",
      canPromote: true,
      agents: ["cursor"],
      adapters: [{ agent: "cursor", status: "native" }],
    }];

    const markup = renderToStaticMarkup(
      <GlobalView
        view="mcp"
        snapshot={snapshot}
        client={new DemoClient()}
        onSnapshotRefresh={vi.fn()}
        onNavigateToProjects={vi.fn()}
      />,
    );

    expect(markup.match(/<code>github<\/code>/g)).toHaveLength(1);
    expect(markup).toContain("Cursor");
  });

  it("keeps project MCP outside the global inventory", () => {
    const snapshot = createDemoSnapshot();
    snapshot.global.nativeMcpItems = [];
    snapshot.projects = [{
      id: "demo",
      name: "demo",
      path: "/tmp/demo",
      profile: "Desktop",
      stack: [],
      agents: [],
      mcp: [{
        id: "project-only",
        name: "project-only",
        transport: "stdio",
        endpoint: "本地命令（已脱敏）",
        secretRefs: [],
        recipe: "chrome-devtools",
        recipeStatus: "ready",
        credentialState: "not-required",
        credentials: [],
        adapters: [{ agent: "cursor", status: "project" }],
      }],
      syncState: "discovered",
      updatedAt: "本次读取",
    }];
    const markup = renderToStaticMarkup(
      <GlobalView
        view="mcp"
        snapshot={snapshot}
        client={new DemoClient()}
        onSnapshotRefresh={vi.fn()}
        onNavigateToProjects={vi.fn()}
      />,
    );

    expect(markup).not.toContain("项目覆盖");
    expect(markup).toContain("项目范围 MCP");
    expect(markup).toContain("项目");
    expect(markup).toContain("project-only");
  });

  it("groups native MCP definitions once and lists their Agent targets", () => {
    const snapshot = createDemoSnapshot();
    snapshot.global.nativeMcpItems = [{
      id: "native:f2c-mcp",
      name: "f2c-mcp",
      description: "",
      transport: "stdio",
      endpoint: "本地命令（已脱敏）",
      secretRefs: ["${env:personalToken}"],
      source: "agent",
      scope: "agent",
      recipe: "f2c-mcp",
      recipeStatus: "ready",
      credentialState: "configured",
      agents: ["Antigravity", "Cursor"],
      category: "shared",
      canPromote: true,
      serverId: "f2c-mcp",
      credentials: [{ key: "personalToken", source: "environment", status: "configured", reference: "${env:personalToken}" }],
      adapters: [
        { agent: "antigravity", status: "native" },
        { agent: "cursor", status: "native" },
      ],
    }];
    const markup = renderToStaticMarkup(
      <GlobalView view="mcp" snapshot={snapshot} client={new DemoClient()} onSnapshotRefresh={vi.fn()} onNavigateToProjects={vi.fn()} />,
    );
    expect(markup).toContain("Agent 原生");
    expect(markup).toContain("Antigravity");
    expect(markup).toContain("Cursor");
  });

  it("labels both Antigravity clients as sharing one MCP config", () => {
    const snapshot = createDemoSnapshot();
    snapshot.global.nativeMcp = [
      { agent: "antigravity-ide", path: "~/.gemini/config/mcp_config.json", state: "parsed", reason: "与 CLI 共享", serverCount: 2 },
      { agent: "antigravity-cli", path: "~/.gemini/config/mcp_config.json", state: "parsed", reason: "与 IDE 共享", serverCount: 2 },
      { agent: "antigravity", path: "~/.gemini/antigravity/mcp_config.json", state: "parsed", reason: "本机链接到共享文件", serverCount: 2 },
    ];
    const markup = renderToStaticMarkup(
      <GlobalView
        view="mcp"
        snapshot={snapshot}
        client={new DemoClient()}
        onSnapshotRefresh={vi.fn()}
        onNavigateToProjects={vi.fn()}
      />,
    );

    expect(markup).toContain("Antigravity IDE");
    expect(markup).toContain("Antigravity CLI");
    expect(markup).toContain("Antigravity</strong>");
    expect(markup).toContain("Antigravity 桌面应用也链接到该文件");
  });

  it("does not claim the Antigravity app shares MCP when its link is unverified", () => {
    const snapshot = createDemoSnapshot();
    snapshot.mode = "real";
    snapshot.global.nativeMcp = [
      { agent: "antigravity-ide", path: "~/.gemini/config/mcp_config.json", state: "parsed", reason: "与 CLI 共享", serverCount: 2 },
      { agent: "antigravity-cli", path: "~/.gemini/config/mcp_config.json", state: "parsed", reason: "与 IDE 共享", serverCount: 2 },
      { agent: "antigravity", path: "~/.gemini/antigravity/mcp_config.json", state: "unavailable", reason: "未验证桌面应用配置链接", serverCount: 0 },
    ];
    const markup = renderToStaticMarkup(
      <GlobalView view="mcp" snapshot={snapshot} client={new DemoClient()} onSnapshotRefresh={vi.fn()} onNavigateToProjects={vi.fn()} />,
    );

    expect(markup).toContain("桌面应用的共享链接未核实");
    expect(markup).not.toContain("Antigravity 桌面应用也链接到该文件");
  });
});

describe("GlobalView Skills", () => {
  it("shows Antigravity app, IDE and CLI as separate global skill sources", () => {
    const snapshot = createDemoSnapshot();
    snapshot.global.nativeSkills = [
      { agent: "antigravity-cli", path: "~/.gemini/antigravity-cli/skills", state: "missing", reason: "未找到目录", items: [] },
      { agent: "antigravity", path: "~/.gemini/config/skills", state: "present", reason: "", items: [{ name: "review", path: "~/.gemini/config/skills/review", state: "present" }] },
      { agent: "antigravity-ide", path: "~/.gemini/antigravity/skills", state: "present", reason: "", items: [{ name: "review", path: "~/.gemini/antigravity/skills/review", state: "present" }] },
    ];
    const markup = renderToStaticMarkup(
      <GlobalView
        view="skills"
        snapshot={snapshot}
        client={new DemoClient()}
        onSnapshotRefresh={vi.fn()}
        onNavigateToProjects={vi.fn()}
      />,
    );
    expect(markup).toContain("Antigravity IDE");
    expect(markup).toContain("Antigravity CLI");
    expect(markup).toContain("Antigravity</strong>");
    expect(markup).toContain("~/.gemini/antigravity/skills");
    expect(markup.indexOf("~/.gemini/config/skills")).toBeLessThan(markup.indexOf("~/.gemini/antigravity-cli/skills"));
    expect(markup).toContain("未找到目录");
  });
});

describe("GlobalView Workflows", () => {
  it("shows IDE global workflows without treating the other Antigravity products as sources", () => {
    const snapshot = createDemoSnapshot();
    snapshot.global.nativeWorkflows = [
      { agent: "antigravity-ide", path: "~/.gemini/antigravity/global_workflows", state: "present", reason: "", items: [{ name: "review.md", path: "~/.gemini/antigravity/global_workflows/review.md", state: "present" }] },
      { agent: "antigravity", path: "", state: "unavailable", reason: "没有已验证的独立 Workflow 路径", items: [] },
      { agent: "antigravity-cli", path: "", state: "unavailable", reason: "没有已验证的独立 Workflow 路径", items: [] },
    ];
    const markup = renderToStaticMarkup(
      <GlobalView view="workflows" snapshot={snapshot} client={new DemoClient()} onSnapshotRefresh={vi.fn()} onNavigateToProjects={vi.fn()} />,
    );
    expect(markup).toContain("本机 Workflow 来源");
    expect(markup).toContain("review.md");
    expect(markup).toContain("Antigravity IDE");
    expect(markup).toContain("Antigravity CLI");
    expect(markup).not.toContain("尚未进入当前迭代");
  });

  it("does not count empty Workflow files as discovered or label an empty source unreadable", () => {
    const snapshot = createDemoSnapshot();
    snapshot.global.nativeWorkflows = [
      { agent: "antigravity-ide", path: "~/.gemini/antigravity/global_workflows", state: "present", reason: "", items: [
        { name: "review.md", path: "~/workflows/review.md", state: "present" },
        { name: "draft.md", path: "~/workflows/draft.md", state: "empty" },
      ] },
      { agent: "antigravity-cli", path: "~/other-workflows", state: "empty", reason: "没有 Markdown 文件", items: [] },
    ];
    const markup = renderToStaticMarkup(
      <GlobalView view="workflows" snapshot={snapshot} client={new DemoClient()} onSnapshotRefresh={vi.fn()} onNavigateToProjects={vi.fn()} />,
    );

    expect(markup).toContain("1 个文件");
    expect(markup).toContain("1 个已发现");
    expect(markup).toContain("空文件");
    expect(markup).toContain("没有 Workflow 文件");
    expect(markup).not.toContain("2 个已发现");
  });
});

describe("GlobalView Hooks", () => {
  it("lists both desktop apps and the CLI without claiming unverified settings contain hooks", () => {
    const snapshot = createDemoSnapshot();
    snapshot.global.nativeHooks = [
      { agent: "antigravity", path: "~/.gemini/config/hooks.json", state: "present", reason: "仅发现文件" },
      { agent: "antigravity-ide", path: "~/.gemini/config/hooks.json", state: "present", reason: "仅发现文件" },
      { agent: "antigravity-cli", path: "~/.gemini/antigravity-cli/settings.json", state: "unverified", reason: "文件存在，Hooks 配置尚未核对" },
    ];
    const markup = renderToStaticMarkup(
      <GlobalView view="hooks" snapshot={snapshot} client={new DemoClient()} onSnapshotRefresh={vi.fn()} onNavigateToProjects={vi.fn()} />,
    );
    expect(markup).not.toContain("本机 Hook 来源");
    expect(markup).toContain("Canonical source");
  });

  it("separates settings with no Hooks from malformed settings", () => {
    const snapshot = createDemoSnapshot();
    snapshot.mode = "real";
    snapshot.global.nativeHooks = [
      { agent: "claude", path: "~/.claude/settings.json", state: "no-hooks", reason: "配置文件没有 Hook 定义" },
      { agent: "gemini", path: "~/.gemini/settings.json", state: "invalid", reason: "配置文件格式无效" },
      { agent: "antigravity-cli", path: "~/hooks.json", state: "present", reason: "文件已发现" },
    ];
    const markup = renderToStaticMarkup(
      <GlobalView view="hooks" snapshot={snapshot} client={new DemoClient()} onSnapshotRefresh={vi.fn()} onNavigateToProjects={vi.fn()} />,
    );

    expect(markup).not.toContain("本机 Hook 来源");
    expect(markup).toContain("Canonical source");
  });
});

describe("GlobalView Subagents", () => {
  it("shows shared Antigravity definitions and an unverified IDE target separately", () => {
    const snapshot = createDemoSnapshot();
    snapshot.global.nativeSubagents = [
      { agent: "antigravity", path: "~/.gemini/config/agents", state: "present", reason: "", items: [{ name: "review.md", path: "~/.gemini/config/agents/review.md", state: "present" }] },
      { agent: "antigravity-cli", path: "~/.gemini/config/agents", state: "present", reason: "", items: [{ name: "review.md", path: "~/.gemini/config/agents/review.md", state: "present" }] },
      { agent: "antigravity-ide", path: "", state: "unavailable", reason: "无已核实独立路径", items: [] },
    ];
    const markup = renderToStaticMarkup(
      <GlobalView view="subagents" snapshot={snapshot} client={new DemoClient()} onSnapshotRefresh={vi.fn()} onNavigateToProjects={vi.fn()} />,
    );
    expect(markup).not.toContain("本机 Subagent 来源");
    expect(markup).toContain("Canonical source");
  });
});
