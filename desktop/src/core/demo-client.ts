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
} from './model';

const fullComponents = (): CapabilityReport['components'] => ({
    rules: 'full',
    skills: 'full',
    mcp: 'full',
    commands: 'full',
    hooks: 'full',
    subagents: 'full',
});

const partialComponents = (): CapabilityReport['components'] => ({
    rules: 'full',
    skills: 'full',
    mcp: 'full',
    commands: 'partial',
    hooks: 'partial',
    subagents: 'full',
});

const unsupportedComponents = (): CapabilityReport['components'] => ({
    rules: 'full',
    skills: 'unsupported',
    mcp: 'full',
    commands: 'unsupported',
    hooks: 'unsupported',
    subagents: 'unsupported',
});

const agents: AgentSummary[] = [
    {
        id: 'codex',
        name: 'Codex',
        vendor: 'OpenAI',
        version: '本地 CLI',
        health: 'good',
        syncState: 'synced',
        capabilities: { memory: 'full', components: fullComponents() },
        lastApplied: '刚刚',
    },
    {
        id: 'cursor',
        name: 'Cursor',
        vendor: 'Anysphere',
        version: '0.48',
        health: 'good',
        syncState: 'synced',
        capabilities: { memory: 'full', components: partialComponents(), note: 'Hooks 由 Cursor 原生生命周期接管' },
        lastApplied: '12 分钟前',
    },
    {
        id: 'gemini',
        name: 'Gemini CLI',
        vendor: 'Google',
        version: '0.11',
        health: 'warn',
        syncState: 'attention',
        capabilities: { memory: 'full', components: unsupportedComponents(), note: '当前适配器不支持 Skills、Commands、Hooks、Subagents' },
        lastApplied: '昨天 18:32',
    },
    {
        id: 'antigravity',
        name: 'Antigravity',
        vendor: '独立适配器',
        version: '项目配置',
        health: 'warn',
        syncState: 'attention',
        capabilities: { memory: 'partial', components: { ...partialComponents(), hooks: 'unsupported', subagents: 'unsupported' }, note: 'Hooks、Subagents 等待原生能力确认' },
        lastApplied: '3 天前',
    },
];

const rules: GlobalRuleItem[] = [
    { id: 'agent-governance', name: 'agent-governance', description: '先读取项目技术栈、规范和约束，再开始修改代码。', enabled: true, source: 'global' },
    { id: 'api-safety', name: 'api-safety', description: '数据变更 API 必须提供成功提示兜底文案。', enabled: true, source: 'global' },
    { id: 'ui-conventions', name: 'ui-conventions', description: '优先复用现有组件、样式和字体边界，不引入未经确认的新体系。', enabled: true, source: 'global' },
];

const mcp: GlobalMcpItem[] = [
    { id: 'github', name: 'GitHub', description: '读取仓库、Issue、Pull Request 和代码搜索。', transport: 'http', endpoint: 'https://api.github.com/mcp', secretRefs: ['${secret:GITHUB_TOKEN}'], source: 'global' },
    { id: 'linear', name: 'Linear', description: '同步产品任务、项目状态和迭代上下文。', transport: 'sse', endpoint: 'https://mcp.linear.app/sse', secretRefs: ['${secret:LINEAR_API_KEY}'], source: 'global' },
    { id: 'supabase', name: 'Supabase', description: '按项目 Profile 访问数据库元数据和边界工具。', transport: 'stdio', endpoint: 'npx supabase-mcp', secretRefs: ['${env:SUPABASE_ACCESS_TOKEN}'], source: 'project' },
];

const projects: ProjectSummary[] = [
    { id: 'makebestmusic', name: 'MakeBestMusic', path: '~/git/work/makebestmusic-nextjs', profile: 'Next.js 产品前端', stack: ['Next.js', 'React', 'TypeScript', 'Tailwind'], agents: ['Codex', 'Cursor'], syncState: 'synced', updatedAt: '今天 09:41' },
    { id: 'niew', name: 'Niew', path: '~/git/work/niew-nextjs', profile: 'Next.js 应用', stack: ['Next.js', 'React', 'TypeScript'], agents: ['Codex'], syncState: 'synced', updatedAt: '昨天 16:20' },
    { id: 'songai', name: 'SongAI', path: '~/git/work/songai', profile: 'AI 音乐服务', stack: ['Go', 'Vue', 'PostgreSQL'], agents: ['Codex', 'Gemini CLI'], syncState: 'attention', updatedAt: '3 天前' },
];

const activity: ActivityItem[] = [
    { id: 'activity-1', title: '已应用到 Codex', detail: '全局规范 + MakeBestMusic Profile，共 11 个文件', timestamp: '刚刚', tone: 'success' },
    { id: 'activity-2', title: '发现 Gemini 能力差异', detail: 'Skills、Commands、Hooks、Subagents 将被跳过', timestamp: '12 分钟前', tone: 'warning' },
    { id: 'activity-3', title: 'SongAI Profile 更新', detail: '新增 Go 服务目录和 Supabase MCP 绑定', timestamp: '昨天 16:20', tone: 'info' },
];

export function createDemoSnapshot(): WorkspaceSnapshot {
    return {
        schemaVersion: 1,
        mode: 'demo',
        generatedAt: '2026-08-28T09:41:00+08:00',
        metrics: { agents: agents.length, projects: projects.length, components: 24, attention: 2 },
        global: { canonicalState: 'ready', canonicalPath: '~/.agentsync/', rules, mcp, skills: 8, hooks: 4, subagents: 6 },
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
        warnings: ['Gemini 不支持 Skills', 'Antigravity 不支持 Hooks'],
    };
}

export class DemoClient {
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
            canonicalPath: request.scope === 'global' ? '~/.agentsync/memory/AGENTS.md' : `${request.projectPath}/.agentsync/memory/AGENTS.md`,
            body: '# Agent Governance\n\nPrefer the existing project stack and patterns.\n',
            fragments: [],
            agents: request.agents ?? ['codex', 'cursor', 'gemini'],
        };
        return {
            document,
            availableAgents: agents.map((agent) => agent.id),
            blocked: false,
            targets: document.agents.map((agent) => ({ agent, path: `~/.${agent}/RULES.md`, supported: true, status: 'pending', blocked: false, willWrite: true })),
        };
    }

    async saveRule(request: RuleRequest & { body: string }): Promise<RuleDocument> {
        return { ...(await this.getRules(request)).document, body: request.body };
    }

    async syncRules(request: RuleRequest & { resolution?: 'backup-overwrite' }): Promise<RuleSyncResult> {
        return { preview: await this.getRules(request), applied: true, backups: [] };
    }

    async importNativeRule(request: RuleRequest & { agent: string }): Promise<RuleDocument> {
        return { ...(await this.getRules(request)).document, body: `# Imported from ${request.agent}\n` };
    }

    async importProject(path: string): Promise<ProjectRuleImport> {
        return { path, needsAnalysis: false, sources: [{ path: `${path}/AGENTS.md`, agents: ['codex', 'cursor'], body: '# Project Rule\n' }] };
    }

    async analyzeProjectRules(): Promise<RuleProposal> {
        return { body: '# Project Rule\n', notes: ['原生规则内容一致。'], analyzer: 'deterministic', requiresAI: false };
    }
}
