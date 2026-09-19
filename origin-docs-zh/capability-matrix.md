# 能力矩阵

每个代理、每个组件的哪些内容有效、哪些内容有损以及哪些内容被推迟。这个
是测试版的诚实期望页面。如果翻译有损或
已跳过，agentsync 在[翻译报告](concepts.md#translation-report--coverage) 中的应用时如此表示；
没有什么会悄然掉落。

**图例**

|符号|意义|
|:--:|---|
| ✓ | **原生** — 完全保真度；代理商直接有概念|
| ◐ | **预计** — 翻译并记录、报告损失 |
| ✗ | **跳过**——没有诚实的翻译；登录申请报告|
| — |尚未实现（适配器已注册但无操作）|

---

## 代理状态

| Agent | Status (beta) | Notes |
|---|---|---|
| **Claude Code** | ✅ Adapter | MCP, memory, skills, subagents, commands, and hooks. Canonical LSP servers are skipped because Claude Code only reads LSP servers from plugin manifests, not `settings.json`; installed **plugins + marketplaces** are captured by `import` (read from `enabledPlugins` / `extraKnownMarketplaces`); on `apply`, each plugin's components project to Claude's native paths and the enablement keys themselves are deliberately left untouched (`PluginIngester` is read-only — see the [shared invariant](#plugin-importapply-the-shared-invariant) below). |
| **OpenCode** | ✅ Adapter (some components projected/skipped) | MCP, memory, skills, subagents, commands. Hooks and LSP are skipped with a warning. No native plugin/marketplace concept, so nothing for plugin `import` to capture; it still *receives* plugin-projected components (skills, MCP, …) on `apply`. |
| **Codex CLI** | ✅ Adapter (some components projected) | MCP, memory, skills, subagents, slash commands, and hooks. MCP servers and hooks both merge into the TOML `~/.codex/config.toml` (as `[mcp_servers.*]` / inline `[hooks.*]` tables — Codex's documented equivalent to a separate `hooks.json`), so config.toml is the adapter's single key-merge file and the user's other keys (`model`, `sandbox_mode`, `[plugins.*]`, …) are preserved; subagents project to Codex's TOML agent format and slash commands to global-only custom prompts (both ◐). Codex **has a native plugin system**[^codex-plugins] with enable-state in `~/.codex/config.toml` as `[plugins."<name>@<source>"] enabled = …`, so the adapter implements `PluginIngester`: `import codex:plugin` captures that enable-state. The render never re-emits those tables (same [invariant](#plugin-importapply-the-shared-invariant) as Claude). Codex records no marketplace *fetch source* in a documented config location, so each plugin's marketplace is resolved from agentsync's own registered marketplaces (`agentsync marketplace add <source>` first), warning + skipping any it can't — exactly how Claude's auto-available built-in marketplace is handled. |
| **Cursor** | ✅ Adapter (some components projected) | MCP, memory, skills, subagents, slash commands, and hooks. MCP lands in `.cursor/mcp.json` (the same `mcpServers` shape as Claude — full fidelity) and hooks in `.cursor/hooks.json` (Claude's lifecycle events remapped to Cursor's camelCase names; the required top-level `version` is asserted when missing, never overwriting a user-set value; events with no Cursor equivalent are dropped with a report). Memory projects to the repo-root `AGENTS.md` at **project scope only** — Cursor keeps user-level rules in app-local storage, so user-scope memory has no filesystem target (reported as a skip). Subagents project to `.cursor/agents/<name>.md` (Claude's `tools`/`color` have no Cursor field and drop); slash commands to `.cursor/commands/<name>.md` (plain markdown — frontmatter drops). Only LSP is unsupported (Cursor has no LSP concept). Cursor **has a native plugin system**[^cursor-plugins], but where it records local enable-state is undocumented, so the adapter implements **no `PluginIngester` yet** (plugin *discovery* on `import` is deferred); it still *receives* plugin-projected components (skills, MCP, …) on `apply` like any agent. |
| **Gemini CLI** | ✅ Adapter (some components projected) | MCP, memory, subagents, slash commands, and hooks. MCP and hooks both merge into `.gemini/settings.json` (MCP `mcpServers` with Gemini's `url`/`httpUrl` transport split (Gemini expands `$VAR`/`${VAR}` in every settings.json string with no escape, so a resolved value containing `$` is written verbatim and flagged with a report); hooks under `hooks`, the same nested shape as Claude, with events remapped to Gemini's `BeforeTool`/`AfterTool`/… and unmapped ones dropped; consecutive handlers sharing an event+matcher render as one multi-handler group, an empty hook `type` is omitted rather than emitted as `"type":""`, and a matcher on an always-fire event is dropped with a report) — so settings.json is the adapter's single key-merge file and the user's other keys (`theme`, `model`, …) are preserved. Gemini reads settings.json as **JSONC**, so the merge is JSONC-tolerant (`merge-jsonc-keys`): a commented file's foreign keys are preserved, with comments stripped on the first agentsync write (documented in Known limits). On `import`, a hook event agentsync can't fully represent — a Gemini-only event (`BeforeModel`, …) or a handler with unmodeled fields (`timeout`, `name`, `sequential`) — is left uncaptured with a warning, so a later apply never owns an array it would lossily rewrite. Memory projects to `GEMINI.md` (`~/.gemini/GEMINI.md` at user scope, repo-root `GEMINI.md` at project scope — full fidelity). Slash commands become `.gemini/commands/<name>.toml` (`description` + `prompt`; `argument-hint`/`allowed-tools` drop; subdirectory namespaces like `git/commit.toml` → `/git:commit` round-trip native→native, though the flat canonical source can't carry the `/`-bearing name), subagents `.gemini/agents/<name>.md` (Claude's `tools` vocabulary differs from Gemini's, so it and `color` drop). **Skills** (Gemini uses extensions, not Agent Skills) and **LSP** have no Gemini concept and are skipped. Gemini has no native plugin enable-state agentsync models, so there is no `PluginIngester`; it still *receives* plugin-projected components on `apply`. |
| **Continue** | ✅ Adapter (some components projected) | MCP, memory, and slash commands — projected as Continue "blocks" (one file per item under `.continue/`, so the adapter owns no shared key-merge file). MCP servers each become a `.continue/mcpServers/<id>.yaml` block (stdio command/args/env; remote `streamable-http`/`sse` + `url` with auth headers under `requestOptions.headers`; other `requestOptions` subkeys a native block carries — `timeout`, `verifySsl`, … — are preserved through import/apply via `Extra` passthrough — full fidelity). Memory projects to `.continue/rules/agentsync.md`, a frontmatter-less rule Continue always applies (so it behaves as persistent memory; byte-clean round-trip). Slash commands become `.continue/prompts/<name>.md` prompt blocks (`name` + `description` + `invokable`; `argument-hint`/`allowed-tools` drop). On `import`, only prompts with `invokable: true` are captured as slash commands (a plain prompt is left alone, with a warning — re-applying it would otherwise force it invokable). **Skills, hooks, and LSP** have no Continue concept, and Continue's "agents" are top-level assistants rather than per-file **subagents**, so all four are skipped with a report. No `PluginIngester` (Continue composes blocks from its Hub + local files); it still *receives* plugin-projected components on `apply`. |
| **Windsurf** | ✅ Adapter (scope-asymmetric MCP) | MCP, memory, and slash commands. **MCP is global-only** (`~/.codeium/windsurf/mcp_config.json`, JSON `mcpServers`; stdio command/args/env, remote `serverUrl` + `headers` — a native `url` key ingests but re-renders as `serverUrl`), so it renders at **user scope** and is skipped (reported) at project scope. **Memory** renders at both scopes: project → `.windsurf/rules/agentsync.md` carrying the documented `trigger: always_on` activation frontmatter (workspace rules declare their trigger in frontmatter; ingest strips it, so the canonical body round-trips byte-clean); user → the single global rules file `~/.codeium/windsurf/memories/global_rules.md` (always-on, frontmatter-less, verbatim; Windsurf documents a 6,000-character limit it enforces itself). **Commands** render at both scopes as plain-markdown workflows invoked as `/<name>`: project `.windsurf/workflows/`, user `~/.codeium/windsurf/global_workflows/` (command frontmatter drops). Upstream now prefers `.devin/rules|workflows/` with `.windsurf/` as the supported fallback; agentsync targets `.windsurf/`, which every released version reads. **Skills, subagents, hooks, and LSP** have no Windsurf concept and are skipped. No `PluginIngester`; it still *receives* plugin-projected components on `apply`. |
| **Roo Code** | ✅ Adapter (some components projected) | MCP, memory, and slash commands — clean filesystem `.roo/` paths (rulesync and ruler converged on these). **MCP → `.roo/mcp.json`** (project-level, `mcpServers` with explicit `type: streamable-http`/`sse` for remote + `url`/`headers`; merge-by-server-name preserves foreign servers). Roo's *global* MCP lives in VS Code globalStorage (OS/editor-specific), which agentsync intentionally does **not** target — so user-scope MCP is reported as a skip. **Memory → `.roo/rules/agentsync.md`** (plain-markdown always-applied rule) and **commands → `.roo/commands/<name>.md`** (markdown + frontmatter — Roo keeps **both** `description` *and* `argument-hint`; only `allowed-tools` drops), both at **user and project scope** (`~/.roo/` + `<repo>/.roo/`). **Skills, hooks, and LSP** have no Roo concept, and Roo's "custom modes" are not per-file **subagents**, so all four are skipped. No `PluginIngester`; it still *receives* plugin-projected components on `apply`. |
| **Cline** | ✅ Adapter (scope-asymmetric) | MCP, memory, and slash commands. **MCP → `~/.cline/mcp.json`** at **user scope** — the Cline CLI's clean config (`mcpServers`, transport inferred: stdio command/args/env, remote `url` + `headers`). Cline has no project MCP file, and its VS Code-extension MCP lives in OS/editor-specific globalStorage no config-sync tool writes, so project-scope MCP is reported as a skip. **Memory → `.clinerules/agentsync.md`** (plain markdown — Cline concatenates `.clinerules/`) and **commands → `.clinerules/workflows/<name>.md`** (plain markdown workflows invoked as `/<name>.md`; command frontmatter drops), both at **project scope** (Cline's global rules and workflows live in `~/Documents/Cline/`, a non-XDG app path agentsync deliberately does not target). **Skills, subagents, hooks, and LSP** have no Cline concept and are skipped. No `PluginIngester`; it still *receives* plugin-projected components on `apply`. |
| **Breadth tier (22 agents)** | ✅ Generic adapter (memory + MCP + skills) | A long tail of agents supported by one data-driven [generic adapter](#breadth-tier) — **memory** (rules file) for all, **MCP** where the agent reads a JSON server-map agentsync can express (15 of 22), and **Agent Skills** where the agent natively scans a `SKILL.md` directory (18 of 22). Each is a *verified* spec, not a hand-written package; see the [Breadth tier](#breadth-tier) table for per-agent coverage. They flow through the normal apply/import pipeline (drift, secrets, capture), unlike a one-way rules dump. |

## 插件导入/应用：共享不变量

对于**每个**适配器来说，无论现在还是将来，规则都是相同的：

> **`import` 读取代理的插件启用状态以进行发现； `apply`
> 从不写回。应用扇出插件的_组件_，而不是
> 插件本身。**

agentsync 的 `Adapter` 接口有一个 `Render` （规范→本机组件）
并且可选的 `PluginIngester` 扩展是只读的 (`IngestPlugins` —
本机插件启用状态 → 规范）。没有 `RenderPlugins`。每个
适配器以相同的方式处理不对称性：

|适配器|导入时读取 (`PluginIngester`) |写入 apply (`Render`) |
|---|---|---|
|克劳德| `settings.json#/enabledPlugins`、`…#/extraKnownMarketplaces` |仅组件（技能、MCP、命令……）；启用状态键保持不变|
|法典| `~/.codex/config.toml` `[plugins."<name>@<source>"]` |仅组件（MCP、钩子、内存、技能……）； `[plugins.*]` 保持不变 |
|开放代码 | —（没有原生插件概念）|仅组件（像任何用户创作的组件一样接收插件投影的组件）|
|光标| —（PluginIngester 延迟；本机启用状态位置未记录）|仅组件（技能、MCP、命令……）|
|双子座| —（无本机插件启用状态代理同步模型；使用扩展）|仅组件（MCP、内存、命令、子代理、挂钩）|
|继续 | —（无本机插件启用状态代理同步模型；组成 Hub + 本地块）|仅组件（MCP、内存、命令）|
|风帆冲浪 | —（无本机插件启用状态代理同步模型）|仅组件（MCP、内存、命令）|
|袋鼠代码 | —（无本机插件启用状态代理同步模型）|仅组件（MCP、内存、命令）|
|克莱恩 | —（无本机插件启用状态代理同步模型）|仅组件（MCP、内存、命令）|

一旦插件的组件在本机路径中实现（`~/.claude/skills/<name>/`，
`mcpServers` 在代理的配置中，`~/.codex/AGENTS.md`，...)，消费者
代理通过与手动编写相同的代码路径读取它们
组件。插件归属纯粹是agentsync的内部记账。的
故意省略回写：这会与代理自己的人发生冲突
`/plugin disable` UI（每次应用时都会出现乒乓球），模糊之间的所有权
agentsync 和代理的插件管理器，并与代理的插件管理器一起双重安装
自己的每个插件安装目录。参见
[architecture.md § PluginIngester（只读）](architecture.md#pluginingester-read-only)。

这些本机路径是平坦的，因此插件的**子代理、技能和命令是
由他们的插件命名**：`feature-dev`的`code-reviewer`着陆为
`~/.claude/agents/feature-dev-code-reviewer.md`。没有它，两个插件就会发布
一个组件名称将在一个路径上呈现两个文件。组件你
`~/.agentsync/` 中的手工作者从未被重命名。 MCP 和 LSP 服务器保持其
ids — 拒绝跨源的相同 id 差异，而不是分开重命名，
因为它可能是无声端点劫持。参见
[architecture.md § 插件组件命名空间](architecture.md#plugin-component-namespacing)。

---

## 组件 × 代理

跨代理的组件支持。

|组件|克劳德|开放代码 |法典|光标|双子座|继续 |风帆冲浪 |袋鼠 |克莱恩 |
|---|:--:|:--:|:--:|:--:|:--:|:--:|:--:|:--:|:--:|
| **MCP 服务器** | ✓ `~/.claude.json`（用户）·`.mcp.json`（项目）| ✓ `opencode.json` | ✓ `config.toml` | ✓ `.cursor/mcp.json` | ✓ `.gemini/settings.json` | ✓ `.continue/mcpServers/` | ✓ `mcp_config.json`（仅限用户）| ✓ `.roo/mcp.json`（仅限项目）| ✓ `~/.cline/mcp.json`（仅限用户）|
| **内存** | ✓ `CLAUDE.md` | ✓ `AGENTS.md` | ✓ `~/.codex/AGENTS.md` | ◐ `AGENTS.md` | ✓ `GEMINI.md` | ✓ `.continue/rules/` | ✓ `.windsurf/rules/` + `global_rules.md` | ✓ `.roo/rules/` | ◐ `.clinerules/`（仅限项目）|
| **技能** | ✓ `~/.claude/skills/X/`（目录）| ✓ 共享 `.claude/skills/` | ✓ `~/.agents/skills/` | ✓ `.cursor/skills/` | ✗ 无技能概念| ✗ 无技能概念| ✗ 无技能概念| ✗ 无技能概念| ✗ 无技能概念|
| **子代理** | ✓ `~/.claude/agents/X.md` | ◐ 前端内容被修改 | ◐ 降价 → TOML | ◐ `.cursor/agents/` | ◐ `.gemini/agents/` | ✗ 仅限顶级助理| ✗ 无子代理概念| ✗ 仅限自定义模式 | ✗ 无子代理概念|
| **斜线命令** | ✓ `~/.claude/commands/X.md` | ◐ `argument-hint` 已删除 | ◐ `~/.codex/prompts/` | ◐ `.cursor/commands/` | ◐ `.gemini/commands/`（TOML，命名空间）| ◐ `.continue/prompts/` | ◐ `.windsurf/workflows/` + `global_workflows/` | ◐ `.roo/commands/`（`allowed-tools` 已删除）| ◐ `.clinerules/workflows/`（仅限项目）|
| **挂钩** | ◐ 设置中的 JSON（命令挂钩；报告的其他处理程序类型/字段）| ✗ 跳过（JS/TS 插件）| ◐ `config.toml` `[hooks.*]` | ◐ `.cursor/hooks.json` | ◐ `settings.json` `hooks` | ✗ 无钩概念| ✗ 无钩概念| ✗ 无钩概念| ✗ 无钩概念|
| **LSP服务器** | ✗ 跳过（Claude 仅从插件清单加载 LSP）| ✗ 跳过（推迟）| ✗ 无LSP概念| ✗ 无 LSP 配置 | ✗ 无LSP概念| ✗ 无LSP概念| ✗ 无LSP概念| ✗ 无LSP概念| ✗ 无LSP概念|

> ◐/✗ 单元是*功能*，而不是错误：agentsync 拒绝发明
> 可能会误导您的翻译。每个◐和✗都打印在应用中
> 报告并可通过 `agentsync plugin explain <plugin> --json` 进行查询。

## 广度层

上面的九个适配器是**深层、特定于代理的**包。除了他们之外，
agentsync 通过单个 **数据驱动的通用模型覆盖长尾代理
适配器** (`internal/adapter/generic`)：每个代理都是一个*已验证的规范* — 中的一行
一个表格——而不是一个手写的包。故意通用层
项目**内存**（代理的规则/指令文件），代理所在的**MCP**
agentsync 可以读取一个 JSON server-map 来表达，以及 **Agent Skills** 所在的代理
本机扫描 `SKILL.md` 目录；所有其他组件都被报告为跳过。
广度代理与深度代理运行相同的应用/导入管道，因此它们
获得漂移检测、秘密解决和捕获——而不是单向规则转储。

**MCP 覆盖范围（22 中的 15）。** MCP 合并是 **JSONC 容忍** (hujson)，因此
注释的设置文件（Zed、Copilot 的 `.vscode/mcp.json`、Amp）被解析并且其
外键和值被保留而不是被破坏。评论本身
不保留：文件在第一个 agentsync 上以纯 JSON 形式重新发送
写入，密钥重新排序（原始文件已备份） - 对于 **Zed** 该文件是
用户的整个编辑器 `settings.json`，因此预计会重新格式化（请参阅已知
限制）。每个代理方言旋钮盖
方差：顶级键 (`mcpServers` / `servers` / `mcp` / `context_servers` /
平面命名空间 `amp.mcpServers`)，传输字段 (`type` / `transport` /
推断）、stdio 值 (`stdio` / `local`) 和远程 URL 键 (`url` / `httpUrl` /
`serverUrl`）。这七个只有记忆的特工才是真正具有 MCP 的特工
**不是** JSON 服务器映射 — 数组 (Trae)、YAML (Goose)、TOML (OpenHands、Mistral)、
IDE 应用程序存储（JetBrains、AugmentCode）或云仪表板 (Jules) — 其中
通用引擎拒绝发明形状并报告跳过。

**MCP 传输标准化。** **类型键控**方言（copilot、copilot-cli、
工厂，粉碎 - 那些具有 `type`/`transport` 字段）显式记录 `sse`，
因此 SSE 服务器往返 `sse → sse`。 **交通无钥匙**方言
（antigravity、zed、warp、junie、kiro、amazonq、pi、amp — 无传输场；
Transport 是从存在的 url key 推断出来的）没有地方可以记录它，所以
规范 `sse` 服务器仅使用其 url 编写，并 ** 规范化为
`http`** 如果稍后通过 `import`/`reconcile` 捕获 — 相同的确认
`sse → http` 翻转深层 OpenCode/Windsurf/Cline 适配器（仅适用于
流量不受影响）。双子座方言 **qwen** 方言是个例外：它分裂
跨两个 url 键的两个远程传输 (`httpUrl` = 可流式 HTTP，`url` =
SSE），因此它保留 `sse`。

**检测。** 检测仅供参考（它驱动 `doctor` 的每个代理线路；
它永远不会应用门），并且大多数广度代理都是由 `PATH` 上的二进制文件检测到的
或用户主目录配置目录。其中两个**不可自动检测**并且仅处于活动状态
显式启用时：**copilot**（没有 `DetectBin` 且没有
稳定的用户主目录标记 - 它的配置是每个项目的 `.github/`/`.vscode/`），并且
**jetbrains** （其 `.aiassistant/` 是一个 *项目* 相关规则目录，而不是用户主目录
安装标记 — JetBrains 在 `~/.config/JetBrains/…` 下保留自己的状态。特雷
*检测到*：`~/.trae` 是 Trae Desktop 的用户主目录标记。

**技能覆盖范围（22 中的 18）。** 特工技能是开放的
[agentskills.io](https://agentskills.io) 规范 — 技能是一个*目录* (`SKILL.md`
frontmatter + body，加上捆绑的 `scripts/`/`references/`/`assets/`）。与MCP不同的是，
磁盘上的格式在各个代理之间是**统一的**（没有可建模的方言）；
只有扫描的目录有所不同，因此广度层重用*相同的*投影
深度适配器使用 (`claude.SkillFileOps`) — 捆绑文件和可执行位
逐字节生存。大多数代理都会阅读跨供应商 **`.agents/skills/`**
约定（Codex 目标目录相同，因此渲染管道会删除重复数据）
字节相同的操作而不是争夺路径）；少数人只扫描自己的
`.<agent>/skills/`（Qwen、Junie、Kiro、Factory、Copilot 的 `.github/skills/`）。的
**四个没有技能**是那些本身不扫描 `SKILL.md` 的人
目录：**Jules** 和 **Firebase Studio** *为*其他*代理*发布*技能，
**Amazon Q** 仅通过 MCP 服务器（不同的组件）消耗技能，并且
**OpenHands** 以编程方式加载技能，无需自动扫描目录 - 每个技能
将技能留空并报告跳过。技能没有秘密，所以投影
完全脱离了秘密解决路径。

下面的每条路径都与代理的上游文档**和**交叉引用
包含之前的现有技术配置同步工具（ruler、rulesync）。范围或
没有经过验证的目标的组件会被排除（并报告为跳过），永远不会
猜到了。

|代理|记忆（规则）| MCP|技能 (`SKILL.md` 目录) |
|---|---|---|---|
| **放大器** | `AGENTS.md` · `~/.config/amp/AGENTS.md` | ✓ `~/.config/amp/settings.json`（命名空间 `amp.mcpServers` 键）| ✓ `.agents/skills/` · `~/.config/agents/skills/` |
| **鹅** | `.goosehints` | ✗ YAML `~/.config/goose/config.yaml` | ✓ `.agents/skills/`（+ 用户）|
| **qwen** | `QWEN.md` · `~/.qwen/QWEN.md` | ✓ `.qwen/settings.json`（双 URL 拆分：`httpUrl` = 可流传输的 HTTP，`url` = SSE）| ✓ `.qwen/skills/`（+ 用户）|
| **扭曲** | `WARP.md` | ✓ `.warp/.mcp.json` · `~/.warp/.mcp.json` | ✓ `.agents/skills/`（+ 用户）|
| **朱尔斯** | `AGENTS.md` | ✗ 仅仪表板（云）| ✗ 为其他特工发布技能 |
| **朱妮** | `AGENTS.md`（项目；JetBrains 文档没有全局指南文件）| ✓ `.junie/mcp/mcp.json`（+ 用户）| ✓ `.junie/skills/`（+ 用户）|
| **张开双手** | `AGENTS.md` | ✗ TOML `[mcp]` 数组 | ✗ 编程加载（无自动扫描目录）|
| **亚马逊** | `.amazonq/rules/` | ✓ `.amazonq/mcp.json` · `~/.aws/amazonq/mcp.json` | ✗ 只能通过 MCP 服务器获得技能 |
| **zed** | `AGENTS.md`·`~/.config/zed/AGENTS.md`| ✓ settings.json `context_servers` | ✓ `.agents/skills/`（+ 用户）|
| **千码** | `.kilocode/rules/` | ✓ `.kilocode/mcp.json`（项目）| ✓ `.agents/skills/` · `~/.kilo/skills/` |
| **基罗** | `.kiro/steering/`（+ 用户）| ✓ `.kiro/settings/mcp.json`（+ 用户）| ✓ `.kiro/skills/`（+ 用户）|
| **特雷** | `.trae/rules/project_rules.md` | ✗ 非标准阵列形状 | ✓ `.agents/skills/`（仅限项目）|
| **jetbrains** | `.aiassistant/rules/` | ✗ IDE 应用程序存储 | ✓ `.agents/skills/`（项目；通过配置的代理）|
| **火力基地** | `.idx/airules.md` | ✓ `.idx/mcp.json`（项目）| ✗ 为其他特工发布技能 |
| **反重力** | `AGENTS.md` | ✓ `~/.gemini/config/mcp_config.json`（远程密钥 `serverUrl`；共享 IDE+CLI 路径源自 Codelab — 较旧的安装读取 `~/.gemini/antigravity-cli/mcp_config.json`，其中此写入操作为空操作；当 Antigravity 的文档确定后重新验证。位于 `~/.gemini/`（深 Gemini 适配器的目录）内，但文件不同 — 无冲突）| ✓ `.agents/skills/`（项目；全局路径有争议，省略）|
| **增强代码** | `.augment/rules/`（+ 用户）| ✗ IDE 应用程序存储 | ✓ `.agents/skills/`（+ 用户）|
| **副驾驶** | `.github/copilot-instructions.md` | ✓ `.vscode/mcp.json`（`servers`键，`type`）| ✓ `.github/skills/` · `~/.copilot/skills/` |
| **副驾驶-cli** | `AGENTS.md` | ✓ `~/.copilot/mcp-config.json` (`type`, stdio=“本地”) | ✓ `.agents/skills/`（+ 用户）|
| **粉碎** | `AGENTS.md` | ✓rush.json `mcp`密钥（+用户）| ✓ `.agents/skills/` · `~/.config/crush/skills/` |
| **工厂** | `AGENTS.md` | ✓ `.factory/mcp.json` · `~/.factory/mcp.json` (`type`) | ✓ `.factory/skills/`（+ 用户）|
| **pi** | `AGENTS.md` · `~/.pi/agent/AGENTS.md` | ✓ `~/.pi/agent/mcp.json` | ✓ `.agents/skills/`（+ 用户）|
| **米斯特拉尔** | `AGENTS.md` | ✗ TOML `.vibe/config.toml` | ✓ `.agents/skills/` · `~/.vibe/skills/` |

**故意排除：** **Aider**（无本机 MCP；仅通过内存
`.aider.conf.yml` `read:` 指针 — 需要一个内容+配置指针适配器，超出
通用层）和 **Firebender** （单源 JSON 清单模型）不是
包括等待忠实执行。 Replit/Rovo Dev 同样被推迟
（云/配置字符串内存）。

## 阅读报告

每个 `apply` 和 `plugin explain` 均以覆盖率报告结尾 — 每个插件、每个代理
— 使用相同的三个标记（`check` 不打印一个；它仅 schema-lints
来源并验证秘密）：



```
plugin: atlassian@anthropic
  claude    ✓ full    (1 mcp, 5 commands)
  opencode  ◐ partial (1 mcp; 5 commands → projected)
```



- **✓ 原生** — 组件以完全保真度着陆。
- **◐ 预计** — 它着陆了，但记录的损失如下（例如
  OpenCode 斜杠命令删除其 `argument-hint`）。
- **✗ 跳过** — 不存在诚实的翻译，所以什么也没写；跳过
  已记录，从不沉默。

---

## 每个 ◐ 失去什么

上面每个投影的 (◐) 单元格都是经过深思熟虑、报告的翻译。这是什么
不会延续下去。

请注意，**MCP/LSP 捕获不是场有损**：本机服务器字段 agentsync
不建模（例如 `timeout`、`disabled`、`cwd`）通过
导入/协调时直通 `[server.extra]` 表并在应用时重新渲染，
而不是掉落。 （`Extra` 仅逐字记录 - `${secret:…}` 写着
从字面上看，从未解决。）

**克劳德**

- **Hook** — agentsync 仅适用于 `command` 挂钩 (`matcher` + `command`)，其中
  往返无损。使用非 `command` 处理程序类型的 Claude 钩子，或者
  带有agentsync不建模的字段（例如`timeout`），是**报告的，而不是
  默默地丢弃**：在渲染非命令处理程序时表面为 Skip 并且是
  从未发出（因此agentsync的拥有数组写入不能破坏本机处理程序），
  并且在摄取时，整个事件不会被捕获并带有警告。如果代理同步
  *以前*捕获了该事件（当时很干净，此后本地丰富），
  `import` 还废弃了现已过时的规范 `hooks/<event>.toml` — 因此
  下一个应用永远不会*拥有*，因此永远不会重写，您更富有的本机
  条目。规范挂钩是共享的，因此退休将事件交还给
  该范围内的*每个*钩子渲染代理（每个本机条目按原样冻结）；
  结构畸形的原生形状会发出警告，但绝不会引发退休。
  这种保护和警告行为是由工件锚定的
  `TestIngest_HookArtifactRoundTrip` (`internal/adapter/claude`)。

**开放代码**

- **Subagent** — OpenCode 支持的 frontmatter 键 agentsync 不建模
  显式 (`temperature`、`top_p`、`permission`、`disable`、`prompt`、`steps`)
  逐字逐句地传递；代理 `mode` (`primary`/`all`/`subagent`) 是
  在 `import`/`reconcile` → `apply` 往返过程中保留（原生
  `primary`/`all` 代理不再降级为 `subagent`；克劳德形
  没有 `mode` 的子代理仍默认为 `subagent`）。克劳德独有的钥匙，没有
  OpenCode 主页 — `tools`（其允许列表没有清晰地映射到 OpenCode 的
  `permission` 模型）和 `color` — 会因报告的 Skip 而被丢弃，从不
  默默地。
- **斜杠命令** — OpenCode 支持的命令键 agentsync 不建模
  (`agent`, `subtask`) 逐字传递；克劳德的 `argument-hint` 没有
  OpenCode 字段并被丢弃并报告跳过（没有命令级
  `allowed-tools`；范围是每个代理）。没有前置键被删除
  默默地——每个人要么被渲染，要么以 Skip 的形式出现。
- **摄取所有权** — `import` 仅捕获 agentsync 拥有的代理/命令
  从共享的 `agents/`/`commands/` 目录（所有权是从 apply 读取的）
  状态）；用户手工编写的文件保持不变，并且
  从未被拉入规范源。

**法典**

- **Subagent** — Codex 自定义代理是 TOML，而不是 Markdown：散文正文
  变为 `developer_instructions`，以及 `name`（Codex 要求），
  `description` 和 `model` frontmatter 在**两个**方向上的往返 — a
  故意偏离文件主干的 frontmatter `name` 在
  `apply` → `import`/`reconcile` 往返，因为摄取会重新填充
  frontmatter `name` 来自 TOML `name`，而不是默默地从
  文件名。两个有效名称冲突的子代理在渲染时被拒绝
  命名两者时出错，而不是默默地合并。没有每个代理 `tools`
  允许列表（工具范围只能通过 `[mcp_servers]` / 技能切换来表达），
  因此 `tools` （和 `color`） 会随着报告的跳过而被删除。仅 Codex 代理密钥
  agentsync 没有 (`model_reasoning_effort`, `sandbox_mode`,
  `nickname_candidates`, …) 根本不被发射。
- **斜线命令** — 映射到 Codex *自定义提示* (`~/.codex/prompts/*.md`)，
  确实保留了 `description` + `argument-hint`，但它们是全局的，所以
  **project-scope** 命令没有目标并被跳过；他们也不可能是
  在子目录中命名，并且该功能已被弃用，以支持技能。
- **Hook** — Codex 将 Claude 的声明性钩子架构镜像为内联 `[hooks.*]`
  `~/.codex/config.toml` 中的表（Codex 从 `hooks.json` 读取钩子
  或内联 `[hooks]` 表； agentsync 使用 config.toml 形式，因此适配器
  有一个键合并文件），但识别一组固定的生命周期事件
  （SessionStart、SubagentStart、PreToolUse、PermissionRequest、PostToolUse、
  Pre/PostCompact、UserPromptSubmit、SubagentStop、Stop）；克劳德户外活动
  该集合（例如 `SessionEnd`、`Notification`）没有目标并丢弃。摄取
  与其他钩子适配器具有相同的防护和警告姿势 - 一个事件
  其表包含agentsync不建模的字段被整个拒绝，从不
  有损地捕获 — 并且适配器实现了 `HookIngestGuard`，因此
  本地丰富的事件触发导入的陈旧钩子退休。一
  故意分歧：非`command`处理程序*类型*被**不**拒绝 -
  Codex 在运行时解析并跳过未知类型，agentsync 重新渲染
  逐字输入（据报告减少了跳过），因此它可以无损地往返。

**光标**

- **内存** — 项目内存着陆为 `AGENTS.md`，但光标保持*用户级别*
  应用程序本地存储（而不是文件系统）中的规则，因此用户范围内存没有
  投影目标（报告为跳过）。
- **子代理** — `.cursor/agents/` 下的降价。光标识别
  `name`/`description`/`model`/`readonly`/`is_background`；克劳德的 `tools`
  允许列表和 `color` 没有 Cursor 字段，并随报告一起删除。
- **斜线命令** — 光标命令 (`.cursor/commands/*.md`) 是普通的降价
  没有 frontmatter，所以 `argument-hint`、`description` 和 `allowed-tools` 是
  全部掉落——只有即时尸体幸存。
- **Hook** — 光标使用声明性 `.cursor/hooks.json`，但有自己的
  驼峰式事件名称和扁平条目形状，因此 Claude 的事件被重新映射
  (`PreToolUse`→`preToolUse`, `UserPromptSubmit`→`beforeSubmitPrompt`, …) 以及任何
  没有 Cursor 等效项（例如 `Notification`、`PostCompact`）会被删除
  报告。如果缺少并且用户设置，则所需的顶级 `version` 被断言
  值被保留。 agentsync 仅对 Cursor 的 `command` 挂钩建模，并且仅对
  `command`/`matcher`/`type` 输入字段；在 `import` 上，游标本机事件
  agentsync 无法渲染 (`afterFileEdit`, `beforeShellExecution`, …) — 或
  包含无法完全表示的条目的事件（`prompt` 类型的钩子，或
  像 `timeout`/`failClosed`) 这样的字段 - 未被捕获并带有警告。并且
  因为 Cursor 实现了 `HookIngestGuard`，这是一个在
  干净和*后来*丰富的本机触发导入的陈旧钩子退休
  （以其*规范*名称报告 - `preToolUse` 退休
  `hooks/PreToolUse.toml`），因此后来的 `apply` 永远不会保留
  它将有损地重写数组；结构错误的 hooks.json 形状
  警告但从不触发退休。

**双子座 CLI**

- **子代理** — `.gemini/agents/` 下的降价。捕获的 frontmatter 已通过
  通过逐字记录，所以双子座自己的原生字段 - `kind`/`temperature`/`max_turns`/
  `timeout_mins`/`mcpServers`（以及任何其他本机键）- 生存 `apply` 而不是
  被剥离：`import`/`reconcile` 捕获到规范中的键被重新发出
  在下一次渲染时，不会被整个文件替换剪辑。仅 Claude 的 `tools` 列表
  （它的工具词汇*不同于*Gemini 的 - `read_file`/`grep_search`，而不是
  `Read`/`Grep`，因此逐字复制它会命名 Gemini 没有的工具）和
  `color`（无 Gemini 代理字段）随报告一起删除，并且报告的 `Skip`
  仅列出这些键。 `name` 不存在时默认为文件名（Gemini
  需要它）。直通是*故意的秘密机器异常*（例如
  MCP `extra` passthrough）：子代理 frontmatter — 包括命令/env 形状
  `mcpServers` 块 — 永远不会被秘密解析，也不会被重新引用，所以
  `${secret:…}` 写在那里保留了一个文字字符串，以及一个手工粘贴的实时秘密
  像任何其他手工编写的文本一样，逐字捕获到 *native* frontmatter
  组件 — 将您的 dotfiles 存储库保持私有。
- **斜线命令** — Gemini 命令为 TOML (`.gemini/commands/*.toml`)
  `description` + `prompt`。主体变为 `prompt` 且 `description` 带有
  结束； `argument-hint`/`allowed-tools`没有双子座领域和掉落。双子座的
  参数占位符是 `{{args}}` （不是 Claude 的 `$ARGUMENTS`/`$1`）；身体是
  逐字编写，因此占位符语法不会自动翻译。 Gemini 命名空间
  按子目录的命令 - `commands/git/commit.toml` 是 `/git:commit` - 所以摄取
  递归地遍历树并将子目录路径编码为 `Command.Name` 作为
  正斜杠 relpath (`git/commit`)，渲染反转回子目录
  文件（字节稳定本机→本机往返）。这是适配器的范围
  往返：扁平规范加载器/写入器（`source.ValidateComponentID` 拒绝
  `/`) 不能携带命名空间名称，因此命名空间命令无法在完整的
  `import`→规范源→`apply`循环-批量`import` **用a跳过它
  警告**（它永远不会中止其余的运行，并且命名的单项导入
  大声失败），并且名称空间保留在磁盘上，永远不会默默地被截断。
- **Hook** — Gemini 钩子位于 *相同嵌套的 `hooks` 下的 `settings.json` 中
  shape* 为 Claude，因此仅重新映射事件名称 (`PreToolUse`→`BeforeTool`，
  `PostToolUse`→`AfterTool`、`UserPromptSubmit`→`BeforeAgent`、`Stop`→`AfterAgent`、
  `PreCompact`→`PreCompress`);没有双子座对应的事件 (`SubagentStart`/
  `SubagentStop`/`PostCompact`/`PermissionRequest`) 随报告一起删除。的
  磁盘上的组形状被保留：连续的规范 Hook 共享一个
  （事件，匹配器）通过多处理程序 `hooks` 数组合并为一个组（
  忠实地逆摄取，将其展平），因此摄取→渲染往返是
  幂等，而不是将手工编写的多处理程序组分解为 N 个
  单处理程序的。空钩子 `type` 被省略（绝不是流浪 `"type":""`），
  并且因为只有工具事件 `BeforeTool`/`AfterTool` 接受匹配器，而
  每个生命周期事件 (`BeforeAgent`/`AfterAgent`/`Session*`/`PreCompress`/
  `Notification`) 始终触发，始终触发事件上的非空匹配器是
  丢弃报告的 `SkipReduced`，而不是在 Gemini 忽略它的地方发出。
  Ingest 具有与 Claude 相同的守卫和警告姿势——一个事件携带
  未建模字段（`sequential`、`name`、`timeout`）或非命令处理程序是
  拒绝完整，从未有损地捕获 - 并且适配器实现
  `HookIngestGuard`，因此本机丰富的事件会触发导入的陈旧挂钩
  退休（以其*规范*名称报告）与克劳德的退休情况完全相同。

**继续**

- **斜线命令** — 继续提示块 (`.continue/prompts/*.md`) 带有
  `name` + 可选 `description` + `invokable`;身体成为提示。
  `argument-hint`/`allowed-tools` 没有“继续”字段并放置。
- **子代理 / Hook / Skill / LSP** - Continue 没有每个文件的子代理（其
  “代理”是顶级助手），没有声明性的钩子概念，没有代理技能
  （它使用 Hub 块/扩展），并且没有 LSP 配置，因此每个都被跳过
  报告而不是给出误导性的翻译。

**风帆冲浪**

- **范围分割** - Windsurf 的 MCP 配置仅限全局
  (`~/.codeium/windsurf/mcp_config.json`)：MCP 仅在 **用户范围** 渲染
  （在项目范围内跳过+报告）。内存和命令在**两者**处呈现
  范围（项目 `.windsurf/` 树；用户 `~/.codeium/windsurf/`）。
- **内存** — 在项目范围内，`.windsurf/rules/agentsync.md` 与
  记录了 `trigger: always_on` 激活 frontmatter （工作区规则声明
  它们在 frontmatter 中触发——无 frontmatter 规则的激活是
  未定义）；摄取条**任何**前导 `trigger:` frontmatter 栅栏，以便
  重新应用绝不重复该规则。确切的agentsync `always_on`块
  往返字节干净；手工更改的非 `always_on` 触发器被剥离
  *和*警告（它的激活模式没有规范的家，所以它没有被捕获）。
  在用户范围内，单个全局规则文件
  `~/.codeium/windsurf/memories/global_rules.md` — 始终在线且无 frontmatter，
  逐字书写。它像 Claude 的 `~/.claude/CLAUDE.md` 一样拥有整个文件
  （首次应用时会备份预先存在的手写副本）和 Windsurf
  强制执行其记录的 6,000 个字符限制。 **工作区**规则
  (`.windsurf/rules/`) 有一个单独的、更大的记录限制 -
  **每个文件 12,000 个字符** — agentsync 也将其留给 Windsurf：它
  逐字写入规则主体，既不截断也不标记它。
- **斜线命令** - Windsurf 工作流程是简单的降价调用，如 `/<name>`
  （项目 `.windsurf/workflows/*.md`，用户
  `~/.codeium/windsurf/global_workflows/*.md`），所以命令 `description`/
  `argument-hint`/`allowed-tools` 正面内容掉落——只有正文幸存。
  Windsurf 记录了工作流程的**每个文件 12,000 个字符**限制；代理同步
  逐字写入正文并将执行工作留给 Windsurf。
- **MCP 远程** — Windsurf 不区分 SSE 与可流式 HTTP
  配置（只是 `serverUrl`），因此规范的 `sse` 服务器标准化为 `http` if
  后来通过 `import`/`reconcile` 捕获回来。使用手动编写的服务器
  备用本机 `url` 密钥（摄取也读取）同样被规范化
  重新渲染时到 `serverUrl` — 良性，因为 Windsurf/Devin 接受两个键。

**Roo代码**

- **MCP 范围** — `.roo/mcp.json` 是项目级别； Roo 的“全球”MCP 位于 VS
  代码 globalStorage （特定于操作系统/编辑器），agentsync 不针对该代码，因此
  用户范围 MCP 被报告为跳过。 （rulesync 和 Ruler 进行相同的调用。）
- **MCP 远程/传输标准化** — Roo 记录远程服务器的
  显式 `type` 中的传输（HTTP 为 `streamable-http`，SSE 为 `sse`），因此
  无损捕获 `sse` 服务器往返 `sse → sse`。规范服务器
  没有传输 - `type = ""` 与 `url` 且没有 `command` - 被视为远程且
  呈现为 `type: streamable-http`，因此它在捕获时标准化为 `http`，并且是
  此后稳定（`http → streamable-http → http`）；手工创作的原生条目
  仅携带 `url` 以相同的方式规范化。所以规则，在两个方向上，
  是“一个带有 url、无命令、没有显式传输的服务器是远程的 `http`”。
  `stdio` 不携带 `type` 键。
- **斜线命令** — `.roo/commands/*.md` 保留 `description` 和 `argument-hint`
  （Roo 两者都支持）；只有 `allowed-tools` （以及任何其他未建模的键）会掉落。
- **Subagent / Hook / Skill / LSP** - Roo 的“自定义模式”不是针对每个文件的
  子代理，而 Roo 没有钩子、代理技能或 LSP 概念，因此每个都被跳过
  并附上一份报告。

**克莱因**

- **范围分割** — Cline 没有项目 MCP 文件（其 VS Code 扩展 MCP 是
  操作系统/编辑器特定的 globalStorage 没有工具写入），但它的 CLI 读取干净
  `~/.cline/mcp.json`，因此 MCP 在 **用户范围** 渲染（跳过 + 报告于
  项目范围）。内存+命令在**项目范围**渲染（`.clinerules/`）；
  Cline 的全局规则位于 `~/Documents/Cline/` 中，这是一个非 XDG 应用程序路径 agentsync
  不定位，因此用户范围的内存/命令被跳过+报告。
- **内存** — 一个简单的 `.clinerules/agentsync.md` 规则（Cline 连接
  `.clinerules/` markdown)，字节干净的往返。
- **斜线命令** — Cline 工作流程 (`.clinerules/workflows/*.md`) 很简单
  markdown 被调用为 `/<name>`，因此命令 frontmatter 会被删除——只有正文
  幸存下来。每个渲染的工作流程都带有一个领先的可逆所有权标记
  （惰性 HTML 注释），因此 `import`/`reconcile` 仅捕获 agentsync 拥有的
  工作流程，并在该目录中保留人工编写的工作流程 -
  相同的所有权范围内存从其固定的 `agentsync.md` 文件名中获取。
- **MCP 远程/传输标准化** — Cline 从中推断出传输
  键存在（没有 `type` 字段）：stdio 保留 command/args/env，a
  远程服务器使用 `url` + `headers`。由于没有记录传输，因此
  如果稍后通过以下方式捕获，则规范 `sse` 服务器标准化为 `http`
  `import`/`reconcile`（仅适用流不受影响）。

## 为什么 OpenCode 跳过钩子和 LSP

- **Hooks** — OpenCode hooks 是订阅事件的 JS/TS 插件，而不是
  像 Claude 那样的声明性 shell 命令。没有机械翻译；
  如果您需要 OpenCode 上的钩子，请手动编写一个小插件。
- **LSP** — OpenCode 确实有本机 `lsp` 配置，但 agentsync 延迟
  将 Claude 之外的 LSP 服务器计划到更高版本；今天你会看到
  `lsp server X skipped` 有关非克劳德特工的报告。

## OpenCode MCP 服务器如何规划

OpenCode 的本机 MCP 架构与规范模型不同，因此适配器
翻译而不是逐字复制字段：

- **传输** — 规范 `type = "stdio"` → OpenCode `"type": "local"`;
  `"http"`/`"sse"` → `"type": "remote"`。 OpenCode 没有单独的 SSE 传输，
  因此，如果稍后通过以下方式捕获 `sse` 服务器，则它会标准化为 `http`
  `import`/`reconcile`（仅适用流不受影响）。
- **命令** — 规范的 `command` + `args` 被扁平化为 OpenCode 的单个命令
  `command` 字符串数组 (`["npx", "-y", "pkg"]`)，并在摄取时拆分回来。
- **环境** — 规范 `env` 是在 OpenCode 的 `environment` 下编写的
  键（不是 `env`）。
- **远程** — `url` 和 `headers` 保持不变。

## 全保真投影（✓ 带变换）

一些 ✓ 单元格在退出时仍然会改变形状 - 内容相同，没有损失：

- **Codex MCP** — Claude 的 JSON `mcpServers` 变为 TOML `[mcp_servers.X]` (stdio
  和streamable-HTTP 都可以表示）。
- **光标 MCP** — `.cursor/mcp.json` `mcpServers`。 stdio 服务器与 Claude 的相匹配
  形状（`type`/`command`/`args`/`env`，直至`${env:…}`引用）；一个遥控器
  服务器遵循 Cursor 记录的远程模式 — `url` + `headers` 且 **no
  `type` key** — 因此，agentsync 不会在远程服务器上写入 `type`，并且
  远程服务器的传输标签在捕获时标准化（光标推断
  来自 `url` 的“远程”； `url`/`headers` 仍然是往返）。摄取停留
  读取时允许 `type` 键。 （开放上游问题**164**：是否真实
  光标*拒绝*或默默地*忽略*未知的远程`type` - 删除它是
  保守的、规格匹配的选择有待测试。）
- **Gemini MCP** — `.gemini/settings.json` `mcpServers`：stdio 保持
  命令/args/env；远程服务器使用 Gemini 的传输分割 — `url` 用于 SSE，
  `httpUrl` 用于 HTTP 流 - 两者都往返规范 `type`。双子座
  将 `$VAR`/`${VAR}`/`${VAR:-default}` 变量扩展应用于 *every*
  settings.json 字符串（env、标头和 `url`/`httpUrl`），并且不提供转义
  文字 `$`（上游 `envVarResolver.ts`；没有 `$$`）。因此代理同步
  逐字写入一个解析值——捏造的逃逸本身就会破坏它——
  并报告 `SkipReduced` 命名任何 env/headers/url 值，其 `$` Gemini 会
  展开，因此（数据相关的）读取时损坏就会浮出水面，而不是沉默。这个
  是目的地的值损坏，而不是明文持久性泄漏：
  规范源仍然保留 `${secret:…}` 引用。
- **继续 MCP** — 每台服务器一个 `.continue/mcpServers/<id>.yaml` 块：stdio
  保留命令/args/env；远程服务器使用Continue的`streamable-http`/`sse`
  输入 + `url`，auth 标头位于 `requestOptions.headers` 下。该块的
  必需的 `name`/`version`/`schema` 标头 **往返** — 手工编写
  非默认的 `version`/`schema` 被保留（通过保留的 `Extra` 键）
  而不是重新生成为 `0.0.1`/`v1` 默认值。一个规范服务器同时承载
  `command` 和没有显式 `type` 的 `url` 是不明确的（Continue 块是
  single-transport）：agentsync 将其呈现为 stdio（命令获胜）并报告
  通过简化的 `Skip` 删除 `url`，而不是默默地删除它；一个
  还带有 `url` 的显式类型 `stdio` 服务器会丢弃未使用的
  `url` 具有相同的报告。
- **继续记忆** — 身体着陆为 `.continue/rules/agentsync.md`，a
  Frontmatter-less 规则Continue 始终适用（字节干净的往返）。
- **风帆MCP** — `~/.codeium/windsurf/mcp_config.json` `mcpServers`：stdio
  命令/args/env；远程 `serverUrl` + `headers` （本机 `url` 键也
  摄取，但重新渲染为 `serverUrl` — 两者都被上游接受）。
- **Roo MCP** — `.roo/mcp.json` `mcpServers`：stdio 命令/args/env；远程
  显式 `type: streamable-http`/`sse` + `url` + `headers` （按服务器名称合并
  保留用户自己的服务器）。
- **Roo 记忆** — 身体以 `.roo/rules/agentsync.md` 的形式着陆，这是一条简单的 Roo 规则
  在用户或项目范围内递归应用（字节干净的往返）。
- **Cline MCP** — Cline CLI 的 `~/.cline/mcp.json` `mcpServers`：stdio
  命令/args/env；远程 `url` + `headers` （传输推断，无 `type` 键）。
- **Codex 和 Gemini 内存** — 相同的降价落在 `~/.codex/AGENTS.md` /
  `~/.gemini/GEMINI.md`（项目范围内的存储根目录`GEMINI.md`）。
- **技能（Codex、Cursor 和 18 个广度层代理）** — 相同的技能 *目录*
  根据[特工技能](https://agentskills.io)规范：`SKILL.md`（姓名+
  描述）**加上任何捆绑的 `scripts/`/`references/`/`assets/` 和嵌套
  文件**，全部逐字携带（包含二进制文件，保留可执行位）
  应用、导入和协调——agentsync 对于除
  目录本身。从源回收中删除一项技能（或一个捆绑文件）
  它来自下一个 `apply` 上的每个目的地（首先备份的漂移文件；
  空目录已修剪）。 Codex 将它们安装在 `~/.agents/skills/` 下（由
  默认值 - 无功能标志），光标位于 `.cursor/skills/` 下，并且两者都读取
  共享的 `.claude/skills/`。广度层投影*相同*目录
  （通过共享 `claude.SkillFileOps`）到每个代理经过验证的技能路径 —
  大多数跨供应商 `.agents/skills/`，它逐字节删除重复数据
  法典；请参阅[广度层](#breadth-tier) 表了解每个代理的路径。

## 逃生舱口

您明确控制扇出：

- MCP 服务器或插件条目上的 `agents = ["claude", "opencode"]` → 扇出
  只针对那些代理。
- 插件条目上的 `native_agents = ["claude"]` → 这些代理安装
  插件通过他们自己的插件管理器，所以agentsync不会投影它的
  那里的组件。没有它，你将获得每项技能中的两项：子代理和命令
  每个钩子都会触发两次，因为 `apply` 永远不会禁用内部插件
  另一个工具的插件管理器。 `import` 提议为您录制此内容。

（指定了每个组件 `[plugin.overrides.<agent>]` 跳过，但 **不是
连接 v1** — 投影机不参考它。使用上面的按键。）

---

## 已知限制

这些是有记录的权衡，而不是回归。权威榜单在世
在[自述文件](../README.md#known-limits)中；亮点：

- **评论保留** — 评论在 `mcp/*.toml`、`opencode.json`、
  Gemini 的 `.gemini/settings.json`，在广度层的 JSONC 设置文件中
  （Zed/Amp `settings.json`、Copilot `.vscode/mcp.json`）以及 Codex 中的
  `~/.codex/config.toml` 在回写/导入往返过程中不会保留；
  JSONC 文件以纯 JSON 形式重新发送（保留外键、注释
  已删除，原件已备份）。
- **拥有的密钥手动编辑** — 如果您在共享中手动编辑 agentsync 拥有的密钥
  文件，下一个 `apply` 会在没有备份的情况下覆盖它（agentsync 认为它是它的
  自己的）。首先运行 `agentsync reconcile` 以捕获编辑。
- **不安全来源** — `http://` 和 `git://` 插件/市场来源是
  默认拒绝（MITM 保护）；覆盖与
  `AGENTSYNC_ALLOW_INSECURE_URLS=1`。
- **符号链接的目的地**默认被拒绝；覆盖与
  `AGENTSYNC_ALLOW_SYMLINK_DEST=1`。
- **计划/推迟**：助手和火宗（请参阅[广度层](#breadth-tier)
  §“故意排除”）。

请参阅[用户指南](user-guide.md)将其付诸实践。

[^codex-plugins]：Codex插件系统：
    [developers.openai.com/codex/plugins](https://developers.openai.com/codex/plugins)。
    启用状态位于 `~/.codex/config.toml` 下
    `[plugins."<name>@<source>"]` 表（`enabled` 布尔值） — 相同
    `name@source` 的形状与 Claude 的 `enabledPlugins` 相同，因此未来的 Codex
    `PluginIngester` 将这些表（加上其市场来源）解析为
    已经有相同的 `NativeMarketplace` / `NativePlugin` 描述符 `import`
    消耗。

[^cursor-plugins]：光标插件系统：
    [cursor.com/docs/reference/plugins](https://cursor.com/docs/reference/plugins)。
    插件捆绑规则、技能 (`SKILL.md`)、代理、命令、挂钩和 MCP
    服务器；清单是 `.cursor-plugin/plugin.json` 和多插件存储库
    使用 `.cursor-plugin/marketplace.json` — 与 Claude 几乎相同
    `.claude-plugin/*`，因此agentsync的投影层很大程度上进行了传输。不
    尚未记录：光标记录了哪些插件已安装/启用
    本地（`enabledPlugins`-相当于`PluginIngester`）；
    鉴于 Cursor 将用户规则保留在应用程序本地存储中，这可能不是一个简单的
    配置文件。因此，光标适配器在出厂时**没有** `PluginIngester`
    （`import` 上的插件发现被推迟，直到该位置被记录为止）；
    它仍然像其他适配器一样在 `apply` 上扇出插件*组件*。