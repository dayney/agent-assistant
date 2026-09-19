export type ViewId =
  | "overview"
  | "rules"
  | "mcp"
  | "skills"
  | "workflows"
  | "hooks"
  | "subagents"
  | "agents"
  | "projects"
  | "activity";

export interface NavigationItem {
  id: ViewId;
  label: string;
  description: string;
}

export const navigationItems: NavigationItem[] = [
  {
    id: "overview",
    label: "总览",
    description: "查看全局健康度和待处理事项",
  },
  {
    id: "rules",
    label: "rules",
    description: "管理全局 AI 规则和纪律",
  },
  {
    id: "mcp",
    label: "MCP",
    description: "管理模型上下文协议服务器",
  },
  {
    id: "skills",
    label: "Skills",
    description: "管理全局可用技能库",
  },
  {
    id: "workflows",
    label: "Workflows",
    description: "管理全局工作流配置",
  },
  {
    id: "hooks",
    label: "Hooks",
    description: "管理生命周期钩子脚本",
  },
  {
    id: "subagents",
    label: "Subagents",
    description: "管理子代理预设配置",
  },
  {
    id: "agents",
    label: "Agent",
    description: "查看适配能力和渲染状态",
  },
  {
    id: "projects",
    label: "项目",
    description: "管理项目 Profile 和绑定关系",
  },
  {
    id: "activity",
    label: "活动记录",
    description: "查看应用、漂移和变更记录",
  },
];
