# 野外市场分布

有关供应商如何在当今多个 AI 编码代理（Claude Code、Codex、Cursor、Gemini 等）之间分发 MCP/插件集成的研究说明。用于验证 `PLAN.md` 中的“市场首要地位”论点并为 M2 提供测试装置。

## 长篇大论；博士

2026 年将不存在跨代理插件市场。我们研究的每个供应商都集中在相同的形状上：

1. **一个规范的托管 MCP 服务器** 作为真正的工件。
2. **Anthropic 官方市场中的一个 Claude Code 插件**，它包装了 MCP 服务器（并且可以添加技能/斜线命令）。
3. **每个代理复制粘贴配置片段**供其他人使用（Codex、Gemini CLI、Cursor IDE 规则路径、Copilot CLI 等）。没有发布的工件，没有注册表——只有文档。

供应商手动扇出。碎片是真实存在的，并且在他们的仓库布局中是可见的。

## 图案目录

观察到的三种分布模式：

1. **单一远程 MCP、BYO 客户端配置** — 最低公分母。供应商托管受 OAuth 保护的端点，文档页面每个代理都有一个可复制粘贴的 JSON/TOML 片段。由 Atlassian、Slack、GitHub、Linear、Notion 使用。这是每个非人性代理人实际消耗的东西。
2. **Claude Code 插件 = MCP + 技能/命令包装器** — 对于具有真实市场（Claude Code、OpenCode）的代理，供应商发布了一个精简包，用于注册相同的 MCP 服务器并在斜线命令/技能/子代理上分层。
3. **聚合器目录** — Docker MCP Catalog、Smithery、mcpmarket.com、claudepluginhub、“awesome-*”列表。通过每台主机的一键安装程序重新发布供应商 MCP 的二级索引。不权威。

## 案例 1 — Atlassian

Atlassian 不提供“Codex 插件”。他们运送两件事：

- **Rovo MCP 服务器**：托管在 `https://mcp.atlassian.com/v1/mcp/authv2`，OAuth 2.1，尊重 Jira/Confluence 权限。规范的神器。
- **Claude Code 插件**：`/plugin install atlassian@claude-plugins-official`（撰写本文时安装量约为 57k）。包装了上面的MCP服务器并添加了Claude特有的技能：
  - `/capture-tasks-from-meeting-notes`
  - `/spec-to-backlog`
  - `/generate-status-report`
  - `/search-company-knowledge`
  - `/triage-issue`

对于 **Codex**，没有 Atlassian 发布的插件或市场列表。官方路径：编辑 `~/.codex/config.toml`，添加指向托管端点的 `[mcp_servers.atlassian]` 块，将 Markdown 技能放在 `~/.codex/skills/` 和 `[features].skills = true` 下。 Atlassian 的“设置客户端”文档明确指出：*“根据您使用的客户端，设置过程可能会有所不同。”*

同样的形状适用于 Gemini CLI、Cursor、Copilot CLI、Amazon Q — 所有都列为受支持的客户端，但每个都是单独的复制粘贴集成指南。

## 案例 2 — Slack

Slack 的设置在结构上与 Atlassian 相同，但有一个明显的区别：他们在存储库布局中明确了多市场问题。

**规范工件**：`https://mcp.slack.com/mcp`（OAuth 2.0，仅托管，无 SSE）。

**官方“插件”存储库**：[`slackapi/slack-mcp-plugin`](https://github.com/slackapi/slack-mcp-plugin)。描述：*“要添加到其他客户端的 Slack MCP 的配置信息。”* 布局：



```
slack-mcp-plugin/
├── .claude-plugin/    # Claude Code shim
├── .cursor-plugin/    # Cursor shim
├── .cursor-mcp.json
└── .mcp.json
```



这是agentsync想要自动化的手动维护版本。相同的供应商、相同的后端 MCP、并行的 shim 文件夹并排，因为没有共享格式。如果 Slack 明天添加 Codex，他们将添加一个 `.codex-plugin/` 文件夹。

**官方支持的客户端**：Claude.ai、Claude Code、Perplexity、Cursor。 **法典不在列表中。**

**第二个工件问题**：有*两个*“官方的”Slack MCP 服务器：

- Slack 的托管（OAuth，推荐）
- Anthropic 的 `@modelcontextprotocol/server-slack`（npm、bot-token、较旧）
- 加上著名的社区分叉 (`korotovsky/slack-mcp-server`)

当用户输入 `agentsync mcp add slack` 时，它们是什么意思？需要一个规范 ID 命名空间（`slack@slackapi` vs `slack@modelcontextprotocol` vs `slack@korotovsky`）来消除歧义。 Atlassian 有一个明显的赢家：松弛则不然。

##并排

| |阿特拉斯 |松弛|
|---|---|---|
|规范 MCP |一个托管 (Rovo) |一个托管 (`mcp.slack.com`) |
|克劳德代码插件 |是的，带有技能包装|是的，只需 MCP 注册 |
|多代理 shim 存储库 |否 — 每个客户单独的文档 | yes — 带有 `.claude-plugin/` + `.cursor-plugin/` | 的单个存储库
| Codex 官方路径 |无 |无 |
|竞争“官方”服务器|一|两个（托管 vs npm）+ 社区分叉 |

## 对 agentsync 的影响

1. **市场至上是真实存在的，但仅限于 Claude Code 和 OpenCode。** 对于其他人来说，agentsync 项目并不是市场安装 — 它与当今供应商希望用户手动粘贴的手动 MCP/技能配置相同。 `PLAN.md` 中的 `Plugin content × Agent` 转换表正是这个间隙。

2. **Atlassian 是规范的 M2 测试装置。** 采用 `atlassian@claude-plugins-official`，通用地投影 MCP 服务器，将 5 个斜线命令投影为光标/继续规则，在 Codex 上跳过技能并记录原因。如果agentsync能够在所有七个代理中干净地处理这个插件，那么它就解决了真正的问题。

3. **`slackapi/slack-mcp-plugin` 是每个代理直通的概念验证装置。** 并排包含 `.claude-plugin/` 和 `.cursor-plugin/` 的真实存储库正是 agentsync 的源布局应该干净地摄取的内容。值得用作 M2 中的黄金测试用例。

4. **规范 ID 需要处理同一逻辑服务的多个实现。** Slack 暴露了差距：hosted-OAuth 与 npm-bot-token 与社区分叉。 `PLAN.md` 中当前的 `id@marketplace` 方案适用于一个市场具有一个规范条目的情况；当“官方”分裂时，它是模糊的。建议扩展至 `id@marketplace`，并在实现上进行平局，或由上游所有者指定命名空间。

## 来源

- [Atlassian – Claude 插件 (claude.com)](https://claude.com/plugins/atlassian)
- [通过市场发现并安装预构建插件 (Claude 代码文档)](https://code.claude.com/docs/en/discover-plugins)
- [anthropics/claude-plugins-official (GitHub)](https://github.com/anthropics/claude-plugins-official)
- [atlassian/atlassian-mcp-server (GitHub)](https://github.com/atlassian/atlassian-mcp-server)
- [Atlassian Rovo MCP 服务器入门](https://support.atlassian.com/atlassian-rovo-mcp-server/docs/getting-started-with-the-atlassian-remote-mcp-server/)
- [设置客户端 — Atlassian Rovo MCP 服务器](https://support.atlassian.com/atlassian-rovo-mcp-server/docs/setting-up-clients/)
- [使用 MCP 将 Atlassian 扩展到任何 AI 助手](https://www.atlassian.com/platform/remote-mcp-server)
- [Atlassian MCP 服务器：使用您最喜欢的代理 (Docker) 快速入门](https://www.docker.com/blog/atlassian-remote-mcp-server-getting-started-with-docker/)
- [Slack MCP 服务器概述（Slack 开发人员文档）](https://docs.slack.dev/ai/slack-mcp-server/)
- [连接到 Claude（Slack 开发人员文档）](https://docs.slack.dev/ai/slack-mcp-server/connect-to-claude/)
- [Slack MCP 服务器指南 (slack.com)](https://slack.com/help/articles/48855576908307-Guide-to-the-Slack-MCP-server)
- [slackapi/slack-mcp-plugin (GitHub)](https://github.com/slackapi/slack-mcp-plugin)
- [@modelcontextprotocol/server-slack (npm)](https://www.npmjs.com/package/@modelcontextprotocol/server-slack)
- [korotovsky/slack-mcp-server (GitHub)](https://github.com/korotovsky/slack-mcp-server)
- [如何将 Slack MCP 与 Codex (Composio) 集成](https://composio.dev/toolkits/slack/framework/codex)