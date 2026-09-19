import type {
  ActivityItem,
  AgentSummary,
  ApplyPreview,
  CapabilityReport,
  GlobalMcpItem,
  GlobalRuleItem,
  ProjectSummary,
  ProjectRuleImport,
  RuleDocument,
  RuleProposal,
  RuleRequest,
  RuleSyncResult,
  RuleWorkspace,
  WorkspaceSnapshot,
} from "./model";

const fullComponents = (): CapabilityReport["components"] => ({
  rules: "full",
  skills: "full",
  mcp: "full",
  commands: "full",
  hooks: "full",
  subagents: "full",
  workflows: "full",
});

const partialComponents = (): CapabilityReport["components"] => ({
  rules: "full",
  skills: "full",
  mcp: "full",
  commands: "partial",
  hooks: "partial",
  subagents: "full",
  workflows: "partial",
});

const unsupportedComponents = (): CapabilityReport["components"] => ({
  rules: "full",
  skills: "unsupported",
  mcp: "full",
  commands: "unsupported",
  hooks: "unsupported",
  subagents: "unsupported",
  workflows: "unsupported",
});

const agents: AgentSummary[] = [
  {
    id: "codex",
    name: "Codex",
    vendor: "OpenAI",
    version: "本地 CLI",
    health: "good",
    syncState: "synced",
    capabilities: { memory: "full", components: fullComponents() },
    lastApplied: "刚刚",
  },
  {
    id: "cursor",
    name: "Cursor",
    vendor: "Anysphere",
    version: "0.48",
    health: "good",
    syncState: "synced",
    capabilities: {
      memory: "full",
      components: partialComponents(),
      note: "Hooks 由 Cursor 原生生命周期接管",
    },
    lastApplied: "12 分钟前",
  },
  {
    id: "gemini",
    name: "Gemini CLI",
    vendor: "Google",
    version: "0.11",
    health: "warn",
    syncState: "attention",
    capabilities: {
      memory: "full",
      components: unsupportedComponents(),
      note: "当前适配器不支持 Skills、Commands、Hooks、Subagents",
    },
    lastApplied: "昨天 18:32",
  },
  {
    id: "antigravity",
    name: "Antigravity",
    vendor: "独立适配器",
    version: "项目配置",
    health: "warn",
    syncState: "attention",
    capabilities: {
      memory: "partial",
      components: {
        ...partialComponents(),
        hooks: "unsupported",
        subagents: "unsupported",
      },
      note: "Hooks、Subagents 等待原生能力确认",
    },
    lastApplied: "3 天前",
  },
];

const rules: GlobalRuleItem[] = [
  {
    id: "agent-governance",
    name: "agent-governance",
    description: "先读取项目技术栈、规范和约束，再开始修改代码。",
    enabled: true,
    source: "global",
  },
  {
    id: "api-safety",
    name: "api-safety",
    description: "数据变更 API 必须提供成功提示兜底文案。",
    enabled: true,
    source: "global",
  },
  {
    id: "ui-conventions",
    name: "ui-conventions",
    description: "优先复用现有组件、样式和字体边界，不引入未经确认的新体系。",
    enabled: true,
    source: "global",
  },
];

const mcp: GlobalMcpItem[] = [
  {
    id: "github",
    name: "GitHub",
    description: "读取仓库、Issue、Pull Request 和代码搜索。",
    transport: "http",
    endpoint: "https://api.github.com/mcp",
    secretRefs: ["${secret:GITHUB_TOKEN}"],
    source: "global",
    scope: "global",
    recipe: "github",
    recipeStatus: "ready",
    credentialState: "configured",
    agents: [],
    category: "global",
    canPromote: false,
    serverId: "github",
    credentials: [{ key: "GITHUB_TOKEN", source: "secret-vault", status: "configured", reference: "${secret:GITHUB_TOKEN}" }],
    adapters: [
      { agent: "codex", status: "supported" },
      { agent: "cursor", status: "supported" },
      { agent: "gemini", status: "partial" },
      { agent: "antigravity", status: "partial" },
    ],
  },
  {
    id: "linear",
    name: "Linear",
    description: "同步产品任务、项目状态和迭代上下文。",
    transport: "sse",
    endpoint: "https://mcp.linear.app/sse",
    secretRefs: ["${secret:LINEAR_API_KEY}"],
    source: "global",
    scope: "global",
    recipe: "linear",
    recipeStatus: "ready",
    credentialState: "configured",
    agents: [],
    category: "global",
    canPromote: false,
    serverId: "linear",
    credentials: [{ key: "LINEAR_API_KEY", source: "secret-vault", status: "configured", reference: "${secret:LINEAR_API_KEY}" }],
    adapters: [
      { agent: "codex", status: "supported" },
      { agent: "cursor", status: "supported" },
      { agent: "gemini", status: "partial" },
      { agent: "antigravity", status: "partial" },
    ],
  },
];

const projects: ProjectSummary[] = [
];

const activity: ActivityItem[] = [
  {
    id: "activity-1",
    title: "已应用到 Codex",
    detail: "全局规范 + MakeBestMusic Profile，共 11 个文件",
    timestamp: "刚刚",
    tone: "success",
  },
  {
    id: "activity-2",
    title: "发现 Gemini 能力差异",
    detail: "Skills、Commands、Hooks、Subagents 将被跳过",
    timestamp: "12 分钟前",
    tone: "warning",
  },
  {
    id: "activity-3",
    title: "SongAI Profile 更新",
    detail: "新增 Go 服务目录和 Supabase MCP 绑定",
    timestamp: "昨天 16:20",
    tone: "info",
  },
];

export function createDemoSnapshot(): WorkspaceSnapshot {
  return {
    schemaVersion: 1,
    mode: "demo",
    generatedAt: "2026-08-28T09:41:00+08:00",
    metrics: {
      agents: agents.length,
      projects: projects.length,
      components: 24,
      attention: 2,
    },
    global: {
      canonicalState: "ready",
      canonicalPath: "~/.agentsync/",
      rules,
      nativeRules: [
        { agent: "antigravity-ide", kind: "auxiliary", path: "~/.gemini/GEMINI.md", state: "present", reason: "与 Antigravity CLI 和 Gemini CLI 共享" },
        { agent: "antigravity-cli", kind: "auxiliary", path: "~/.gemini/GEMINI.md", state: "present", reason: "与 Antigravity IDE 和 Gemini CLI 共享" },
        { agent: "antigravity", kind: "auxiliary", path: "~/.gemini/GEMINI.md", state: "present", reason: "与 Antigravity IDE 共享" },
        { agent: "cursor-account", kind: "auxiliary", path: "", state: "unavailable", reason: "账户 User Rules 无可核实本机文件" },
        { agent: "cursor-local", kind: "auxiliary", path: "~/.cursor/rules", state: "missing", reason: "示例目录未找到" },
      ],
      mcp,
      nativeMcpItems: [
        {
          id: "native:chrome-devtools",
          name: "chrome-devtools",
          description: "来自本机 Agent 原生 MCP 配置，尚未进入全局 canonical。",
          transport: "stdio",
          endpoint: "本地命令（已脱敏）",
          secretRefs: [],
          source: "agent",
          scope: "agent",
          recipe: "chrome-devtools",
          recipeStatus: "ready",
          credentialState: "not-required",
          agents: ["antigravity", "antigravity-ide", "antigravity-cli"],
          category: "shared",
          canPromote: true,
          serverId: "chrome-devtools",
          credentials: [],
          adapters: [
            { agent: "antigravity", status: "native" },
            { agent: "antigravity-ide", status: "native" },
            { agent: "antigravity-cli", status: "native" },
          ],
        },
        {
          id: "native:f2c-mcp",
          name: "f2c-mcp",
          description: "来自本机 Agent 原生 MCP 配置，尚未进入全局 canonical。",
          transport: "stdio",
          endpoint: "本地命令（已脱敏）",
          secretRefs: ["${env:personalToken}"],
          source: "agent",
          scope: "agent",
          recipe: "f2c-mcp",
          recipeStatus: "ready",
          credentialState: "configured",
          agents: ["antigravity"],
          category: "agent",
          canPromote: true,
          serverId: "f2c-mcp",
          credentials: [{ key: "personalToken", source: "environment", status: "configured", reference: "${env:personalToken}" }],
          adapters: [{ agent: "antigravity", status: "native" }],
        },
      ],
      nativeMcp: [
        { agent: "codex", path: "~/.codex/config.toml", state: "parsed", reason: "", serverCount: 2 },
        { agent: "claude", path: "~/.claude.json", state: "unverified", reason: "已发现配置文件，未核实导入兼容性", serverCount: 0 },
        { agent: "cursor", path: "~/.cursor/mcp.json", state: "missing", reason: "未找到目标路径", serverCount: 0 },
        { agent: "antigravity-ide", path: "~/.gemini/config/mcp_config.json", state: "parsed", reason: "与 CLI 共享", serverCount: 2 },
        { agent: "antigravity-cli", path: "~/.gemini/config/mcp_config.json", state: "parsed", reason: "与 IDE 共享", serverCount: 2 },
        { agent: "antigravity", path: "~/.gemini/antigravity/mcp_config.json", state: "parsed", reason: "本机链接到共享文件", serverCount: 2 },
      ],
      skills: 8,
      nativeSkills: [
        { agent: "antigravity", path: "~/.gemini/config/skills", state: "present", reason: "仅发现目录条目，未验证导入兼容性", items: [
          { name: "shared", path: "~/.gemini/config/skills/shared", state: "present" },
        ] },
        { agent: "antigravity-ide", path: "~/.gemini/antigravity/skills", state: "present", reason: "仅发现目录条目，未验证导入兼容性", items: [
          { name: "shared", path: "~/.gemini/antigravity/skills/shared", state: "present" },
        ] },
        { agent: "antigravity-ide", path: "~/.gemini/antigravity/global_skills", state: "missing", reason: "未找到目录", items: [] },
        { agent: "antigravity-cli", path: "~/.gemini/antigravity-cli/skills", state: "missing", reason: "未找到目录", items: [] },
        { agent: "shared", path: "~/.agents/skills", state: "present", reason: "仅发现目录条目，未验证导入兼容性", items: [
          { name: "code-review", path: "~/.agents/skills/code-review", state: "present" },
          { name: "release-notes", path: "~/.agents/skills/release-notes", state: "present" },
        ] },
        { agent: "codex", path: "~/.codex/skills", state: "partial", reason: "仅发现目录条目，未验证导入兼容性", items: [
          { name: "code-review", path: "~/.codex/skills/code-review", state: "present" },
          { name: "external", path: "~/.codex/skills/external", state: "unsafe" },
        ] },
        { agent: "cursor", path: "~/.cursor/skills", state: "missing", reason: "未找到目录", items: [] },
      ],
      hooks: 4,
      nativeHooks: [
        { agent: "antigravity", path: "~/.gemini/config/hooks.json", state: "missing", reason: "示例文件未找到" },
        { agent: "antigravity-ide", path: "~/.gemini/config/hooks.json", state: "missing", reason: "示例文件未找到" },
        { agent: "antigravity-cli", path: "~/.gemini/antigravity-cli/settings.json", state: "missing", reason: "示例文件未找到" },
      ],
      subagents: 6,
      nativeSubagents: [
        { agent: "antigravity", path: "~/.gemini/config/agents", state: "missing", reason: "示例目录未找到", items: [] },
        { agent: "antigravity-cli", path: "~/.gemini/config/agents", state: "missing", reason: "示例目录未找到", items: [] },
        { agent: "antigravity-ide", path: "", state: "unavailable", reason: "未核实 IDE 独立的全局 Subagent 目录", items: [] },
        { agent: "cursor", path: "~/.cursor/agents", state: "empty", reason: "示例目录为空", items: [] },
      ],
      workflows: 2,
      nativeWorkflows: [
        { agent: "antigravity-ide", path: "~/.gemini/antigravity/global_workflows", state: "present", reason: "仅发现 Markdown 文件，尚未导入", items: [
          { name: "review.md", path: "~/.gemini/antigravity/global_workflows/review.md", state: "present" },
        ] },
        { agent: "antigravity", path: "", state: "unavailable", reason: "无已核实的独立 Workflow 目录", items: [] },
        { agent: "antigravity-cli", path: "", state: "unavailable", reason: "无已核实的独立 Workflow 目录", items: [] },
      ],
    },
    agents,
    projects,
    activity,
  };
}

export function createDemoApplyPreview(): ApplyPreview {
  return {
    files: 11,
    partial: 2,
    unsupported: 1,
    warnings: ["Gemini 不支持 Skills", "Antigravity 不支持 Hooks"],
  };
}

export class DemoClient {
  async previewNativeRule(_agent: string, _path: string): Promise<string> {
    throw new Error("演示模式不读取本机原生 Rule；请启动 Desktop 应用。");
  }

  async getSnapshot(): Promise<WorkspaceSnapshot> {
    return createDemoSnapshot();
  }

  async previewApply(): Promise<ApplyPreview> {
    return createDemoApplyPreview();
  }

  async getRules(request: RuleRequest): Promise<RuleWorkspace> {
    const document: RuleDocument = {
      scope: request.scope,
      projectPath: request.projectPath,
      canonicalPath:
        request.scope === "global"
          ? "~/.agentsync/memory/AGENTS.md"
          : `${request.projectPath}/.agentsync/memory/AGENTS.md`,
      body: "# Agent Governance\n\nPrefer the existing project stack and patterns.\n",
      fragments: [],
      agents: request.agents ?? ["codex", "cursor", "gemini"],
    };
    return {
      document,
      availableAgents: agents.map((agent) => agent.id),
      blocked: false,
      targets: document.agents.map((agent) => ({
        agent,
        path: `~/.${agent}/RULES.md`,
        supported: true,
        status: "pending",
        blocked: false,
        willWrite: true,
      })),
    };
  }

  async saveRule(
    request: RuleRequest & { body: string },
  ): Promise<RuleDocument> {
    return { ...(await this.getRules(request)).document, body: request.body };
  }

  async syncRules(
    request: RuleRequest & { resolution?: "backup-overwrite" },
  ): Promise<RuleSyncResult> {
    return {
      preview: await this.getRules(request),
      applied: true,
      backups: [],
    };
  }

  async importNativeRule(
    request: RuleRequest & { agent: string },
  ): Promise<RuleDocument> {
    return {
      ...(await this.getRules(request)).document,
      body: `# Imported from ${request.agent}\n`,
    };
  }

  async importProject(path: string): Promise<ProjectRuleImport> {
    return {
      path,
      needsAnalysis: false,
      sources: [
        {
          path: `${path}/AGENTS.md`,
          agents: ["codex", "cursor"],
          body: "# Project Rule\n",
        },
      ],
    };
  }

  async analyzeProjectRules(): Promise<RuleProposal> {
    return {
      body: "# Project Rule\n",
      notes: ["原生规则内容一致。"],
      analyzer: "deterministic",
      requiresAI: false,
    };
  }

  async promoteProjectMcp(): Promise<never> {
    throw new Error("演示模式不写入全局 MCP；请启动 Desktop 应用。");
  }

  async promoteNativeMcp(): Promise<never> {
    throw new Error("演示模式不写入全局 MCP；请启动 Desktop 应用。");
  }

  async importNativeMcpForce(): Promise<never> {
    throw new Error("演示模式不写入全局 MCP；请启动 Desktop 应用。");
  }

  async saveGlobalMcp(): Promise<never> {
    throw new Error("演示模式不支持编辑；请启动 Desktop 应用。");
  }

  async pushMcpToAgent(): Promise<never> {
    throw new Error("演示模式不支持适配操作；请启动 Desktop 应用。");
  }

  async verifyMcpInAgent(): Promise<never> {
    throw new Error("演示模式不支持测试操作；请启动 Desktop 应用。");
  }
}
