import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import { createDemoSnapshot } from "../core/demo-client";
import type { CoreClient } from "../core/client";
import { ProjectsView } from "./ProjectsView";

describe("ProjectsView MCP", () => {
  it("shows a project MCP as a deliberate promotion candidate", () => {
    const snapshot = createDemoSnapshot();
    snapshot.projects = [
      {
        id: "demo",
        name: "demo",
        path: "/tmp/demo",
        profile: "Desktop",
        stack: [],
        agents: ["cursor"],
        mcp: [
          {
            id: "chrome-devtools",
            name: "chrome-devtools",
            transport: "stdio",
            endpoint: "本地命令（已脱敏）",
            secretRefs: [],
            recipe: "chrome-devtools",
            recipeStatus: "ready",
            credentialState: "not-required",
            credentials: [],
            adapters: [{ agent: "cursor", status: "project" }],
          },
        ],
        syncState: "discovered",
        updatedAt: "本次读取",
      },
    ];
    const client = {
      promoteProjectMcp: vi.fn(),
    } as unknown as CoreClient;

    const markup = renderToStaticMarkup(
      <ProjectsView
        snapshot={snapshot}
        client={client}
        onSnapshotRefresh={vi.fn()}
      />,
    );

    expect(markup).toContain("项目 MCP");
    expect(markup).toContain("chrome-devtools");
    expect(markup).toContain("Recipe：chrome-devtools");
    expect(markup).toContain("无需 Token");
    expect(markup).toContain("转为全局");
    expect(markup).toContain("项目 MCP 只在本项目生效");
  });
});
