import {
  Bot,
  FolderKanban,
  GitFork,
  History,
  LayoutDashboard,
  Network,
  ScrollText,
  Sparkles,
  Webhook,
  Workflow,
  type LucideIcon,
} from "lucide-react";
import type { ViewId } from "../app/navigation";

const icons: Record<ViewId, LucideIcon> = {
  overview: LayoutDashboard,
  rules: ScrollText,
  mcp: Network,
  skills: Sparkles,
  workflows: Workflow,
  hooks: Webhook,
  subagents: GitFork,
  agents: Bot,
  projects: FolderKanban,
  activity: History,
};

export function NavigationIcon({ view }: { view: ViewId }) {
  const Icon = icons[view];

  return (
    <Icon
      aria-hidden="true"
      data-nav-icon="true"
      width={18}
      height={18}
      strokeWidth={1.8}
    />
  );
}
