export type ViewId = 'overview' | 'global' | 'agents' | 'projects' | 'activity';

export interface NavigationItem {
    id: ViewId;
    label: string;
    icon: string;
    description: string;
}

export const navigationItems: NavigationItem[] = [
    { id: 'overview', label: '总览', icon: '⌂', description: '查看全局健康度和待处理事项' },
    { id: 'global', label: '全局配置', icon: '◎', description: '管理所有项目共享的规则和 MCP' },
    { id: 'agents', label: 'Agent', icon: '✦', description: '查看适配能力和渲染状态' },
    { id: 'projects', label: '项目', icon: '▣', description: '管理项目 Profile 和绑定关系' },
    { id: 'activity', label: '活动记录', icon: '◷', description: '查看应用、漂移和变更记录' },
];
