import { useState, useMemo } from "react";
import type { CoreClient } from "../core/client";
import type {
  ProjectRuleImport,
  RuleProposal,
  WorkspaceSnapshot,
} from "../core/model";
import { toggleAgentSelection } from "../core/rule-state";

export function useProjectImport(
  client: CoreClient,
  snapshot: WorkspaceSnapshot,
  onSnapshotRefresh: () => Promise<void>,
) {
  const [projectPath, setProjectPath] = useState("");
  const [imported, setImported] = useState<ProjectRuleImport | null>(null);
  const [proposal, setProposal] = useState<RuleProposal | null>(null);
  const [proposalBody, setProposalBody] = useState("");
  const [selectedAgents, setSelectedAgents] = useState<string[]>([]);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");

  const detectedAgents = useMemo(
    () =>
      imported
        ? [
            ...new Set(imported.sources.flatMap((source) => source.agents)),
          ].sort()
        : [],
    [imported],
  );

  async function run(action: () => Promise<void>) {
    setBusy(true);
    setError("");
    setMessage("");
    try {
      await action();
    } catch (cause) {
      console.error("[useProjectImport] Action failed:", cause);
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(false);
    }
  }

  function handleImport() {
    void run(async () => {
      const result = await client.importProject(projectPath.trim());
      setImported(result);
      setProjectPath(result.path);
      const preferred = snapshot.agents
        .map((agent) => agent.id)
        .filter((agent) =>
          result.sources.some((source) => source.agents.includes(agent)),
        );
      setSelectedAgents(
        preferred.length > 0
          ? preferred
          : (result.sources[0]?.agents.slice(0, 1) ?? []),
      );
      setProposal(null);
      setProposalBody("");
      setMessage(
        `已登记项目并发现 ${result.sources.length} 个原生 Rule 文件。`,
      );
      await onSnapshotRefresh();
    });
  }

  function handleAnalyze() {
    if (!imported) return;
    void run(async () => {
      const result = await client.analyzeProjectRules(imported.path);
      setProposal(result);
      setProposalBody(result.body);
      setMessage(
        result.analyzer === "codex-cli"
          ? "Codex 已生成候选，请人工审核。"
          : "原生 Rule 内容一致，已生成确定性候选。",
      );
    });
  }

  function handleSaveProposal() {
    if (!imported || !proposalBody.trim()) return;
    void run(async () => {
      if (selectedAgents.length === 0)
        throw new Error("至少选择一个项目目标 Agent。");
      await client.saveRule({
        scope: "project",
        projectPath: imported.path,
        agents: selectedAgents,
        body: proposalBody,
      });
      setMessage("项目母版已保存。请到“全局配置 / 规则”预览并同步。");
      await onSnapshotRefresh();
    });
  }

  function toggleAgent(agent: string, checked: boolean) {
    setSelectedAgents((current) =>
      toggleAgentSelection(current, agent, checked),
    );
  }

  return {
    projectPath,
    setProjectPath,
    imported,
    proposal,
    proposalBody,
    setProposalBody,
    selectedAgents,
    toggleAgent,
    detectedAgents,
    busy,
    message,
    error,
    handleImport,
    handleAnalyze,
    handleSaveProposal,
  };
}
