# 组件图

代码库的逐包索引。每个条目都列出了包的
责任、其关键导出符号以及它所依赖的内部包
上。有关*各个部分如何组合在一起*，请阅读[架构](architecture.md)；
此页是目录。



```
cmd/agentsync/        # main(): inject version ldflags, call cli.Execute()
internal/
├── cli/              # cobra command tree (entry layer)
├── source/           # the canonical model + loaders/writers   ← the schema
├── secrets/          # ${secret:}/${env:} resolve · re-reference · mask
├── project/          # .agentsync/ tree overlay discovery + merge
├── adapter/          # the per-agent Adapter interface + registry
│   ├── claude/ opencode/ codex/       # 9 deep adapters (agent-specific,
│   ├── cursor/ gemini/ continuedev/   # often bidirectional; claude is
│   ├── windsurf/ roo/ cline/          # the reference implementation)
│   ├── generic/                       # data-driven breadth tier (22 agents, specs.go)
│   └── noop/                          # placeholder for unimplemented agents
├── render/           # the apply pipeline: plan · write · report
├── capture/          # the single dest▶source write-back funnel
├── drift/            # the 3-way classifier (pure, no IO)
├── state/            # targets.json (last-applied hashes)
├── marketplace/      # fetch marketplaces/plugins · project components
├── git/              # leaf go-git wrapper: local dir rollback history (issue #118)
├── iox/              # atomic write + file lock
├── jsonkeys/         # per-key JSON-pointer merge (preserve foreign keys)
├── paths/            # AGENTSYNC_HOME / TARGET_ROOT / HOME resolution
├── ui/               # presentation: Printer · color · glyphs · diagnostics · WarnWriter
├── log/              # slog setup (installs ui.SlogHandler as the default)
└── testenv/          # hermetic-container test guard
```



---

## 入口层

### `cmd/agentsync`
二进制文件的 `main`。通过 `-ldflags` 注入 `Version`/`Commit`/`Date` 并调用
`cli.Execute()`。这里没有其他东西居住。

### `internal/cli`
将每个 cobra 子命令连接到根树并分派给处理程序；这个
是唯一依赖于几乎所有其他包的包。
- **Key:** `NewRoot() *cobra.Command`, `Execute() int` （返回进程退出
  代码并拥有终端 `✗ ERROR` 行）、`Version`/`Commit`/`Date`。
- **命令：** `init`、`agent {add,remove,list,enable,disable}`、`apply`、
  `revert`、`status`、`diff`、`reconcile`、`import`、`doctor`、`check`、
  `mcp {add,remove,list,enable,disable}`，
  `plugin {add,outdated,upgrade,enable,disable,remove,list,explain}`，
  `marketplace {add,remove,list}`、`secret {edit,get,set,list,remove}`、
  `{skill,subagent,command,hook,lsp} list`，
  `migrate subagents`、`explain <path>`、
  `version`。
- **取决于：**适配器、源、状态、秘密、路径、渲染、市场、
  项目、漂移、git、ui、日志。
- **文件：** `root.go` + 每个命令组一个文件。

---

## 核心模型

### `internal/source` — *架构*
加载并表示 `~/.agentsync/`。这里的 TOML 标记的结构*是*
适配器渲染的规范模型；还提供回写助手和
内存碎片扩展。
- **键：** `Canonical`（根模型：`Config`、`MCPServers`、`Skills`、
  `Subagents`、`Commands`、`Hooks`、`LSPServers`、`Plugins`、`Marketplaces`、
  `Memory`、`Project`）； `Load(fs, home)`； `ParseFrontmatter`； `Write*`
  家庭 (`WriteMCP`、`WriteLSP`、`WritePlugin`、`WriteMarketplace`、`WriteSkill`、
  `WriteSubagent`、`WriteCommand`、`WriteHooks`、`WriteMemory`）； `ReadMCP`/`ReadLSP`
  （仅携带源字段）； `ExpandMemoryImports`； `RenderManagedMemory` /
  `StripManagedBanner`（插入/剥离托管文件横幅 - 请参阅
  `docs/architecture.md`); `NamespacedComponentName` / `PluginTargetsAgent` /
  `AgentTargeted`（插件出处+`agents`/`native_agents`门）；
  `FilterForAgent`（将规范缩小为一个代理所呈现的内容，
  由 `render.Plan` 通过 `secrets.Resolved.ForAgent` 使用——它的唯一调用者。
  import 的捕获拒绝过滤器故意不以同样的方式缩小范围；它的
  拒绝集比渲染集更宽，因为目的地仍然可以
  保留记录延迟之前未回收的输出。不恢复
  它们之间的对称性 - 请参阅 `docs/architecture.md`）。
- **取决于：** iox、jsonkeys。
- **文件：** `schema.go`、`loader.go`、`writer.go`、`memory.go`、
  `provenance.go`、`targeting.go`。

### `internal/secrets`
在应用时解析 `${secret:dotted.key}` 和 `${env:NAME}`；重新引用
明文返回 `${secret:…}` 进行回写；屏蔽显示的解析值。
`Resolved` 包装类型是承重防漏装置。
- **键：** `Resolver`（接口）； `Resolved`（已解析模型包装器）；
  `SubstituteCanonical` (→ `Resolved`); `ReReferenceCanonical`； `CollectResolved`；
  `UnresolvedSecretRefs`； `SecretRefsByComponent`（每个组件引用，对于
  `explain`); `MaskResolved`； `AgeBackend`/`EnvBackend`/`NopResolver`;
  `SelectBackend`； `Resolved.ForAgent`（每个代理在渲染腰部缩小，
  委托给 `source.FilterForAgent`);和单个字段列表
  `walkSecretFields`（在`walk.go`中）。
- **取决于：**来源，iox。
- **文件：** `secrets.go`、`age.go`、`resolved.go`、`substitute.go`、
  `rereference.go`、`mask.go`、`refs.go`、`walk.go`、`secretpaths.go`、`leakscan.go`
  （`ResidualSecretCleartext` 后盾），`runtime.go`。

### `internal/project`
发现存储库的项目范围源树 - `.agentsync/` **目录**
（与用户范围 `~/.agentsync/` 相同的磁盘布局）通过向上查找找到
来自 Cwd — 并覆盖其规范（项目代理、MCP/LSP/技能/
子代理/命令/挂钩、额外内存）到基本用户规范上。退休者
不再读取 M5 单文件 `.agentsync.toml` 标记：`Discover` 表面
如果发现没有 `.agentsync/` 树，则 **迁移错误**。
- **键：** `DirName` (`.agentsync`); `LegacyMarkerFile` (`.agentsync.toml`,
  仅迁移）； `Home(root)`； `Discover(start) (root, found, err)`；
  `Merge(base, proj) source.Canonical`。
- **取决于：**来源。
- **文件：** `project.go`。

---

## 翻译层

### `internal/adapter`
声明每个代理 `Adapter` 合约和注册表； `DestWriter`
接口通过外部冲突备份汇集所有目标写入。
**可选** `VersionedDirs` 扩展允许适配器声明磁盘上的
apply tail 应该使用 git-back-up 进行本地回滚的目录——一个适配器
实现 `VersionRoots(scope, project)` 返回其配置目录以及任何
它写入的共享跨代理目录，并且必须在项目范围内返回 nil （请参阅
[体系结构 § VersionedDirs](architecture.md#versioneddirs-optional))。
- **键：** `Adapter`（接口）； `DestWriter`（界面）；
  `VersionedDirs`（可选接口，`VersionRoots`）； `NonEmptyDirs`（助手）；
  `Scope` (`ScopeUser`/`ScopeProject`); `FileOp`； `Skip`（与`SkipKind`）；
  `Registry`（`NewRegistry`、`Register`、`Lookup`、`Names`）。组件支持是
  由 `Render` 发出的内容表示 — 不受支持的组件会产生 `Skip`，
  不是缺席的能力标志。
- **文件：** `adapter.go`、`registry.go`。

### `internal/adapter/claude`
参考适配器 — MCP、内存、技能、子代理、命令和挂钩，以及每键合并
到共享 JSON 文件（`~/.claude.json`、`settings.json` 和项目的
repo-root `.mcp.json` 用于项目范围的 MCP 服务器），保留外部
键。 `IngestPlugins` 读取 `enabledPlugins` / `extraKnownMarketplaces`
发现 `import` 上的插件；渲染将每个插件的组件投影到
Claude 的本机路径 (`~/.claude/skills/<name>/`, `mcpServers` 在
`.claude.json`，...）并故意留下启用密钥本身
未受影响。不对称是跨适配器规则，而不是克劳德的怪癖 - 请参阅
[architecture.md § PluginIngester（只读）](architecture.md#pluginingester-read-only)。
钩子保真度：规范的 `Hook` 仅模型命令处理程序，所以（就像
Gemini) 摄取会留下未捕获的 `settings.json` 挂钩事件，并在以下情况下发出警告：
它带有未建模的定义/处理程序字段（例如 `timeout`）或
非命令处理程序，并且 Render 报告任何非命令的丢弃的 `Skip`
挂钩而不是发出空命令条目。捕获的事件
虽然干净并且“后来”本地丰富会留下陈旧的规范
`hooks/<event>.toml` 后面的下一个 apply — 拥有整个
每个事件数组——仍然会有损地重写； `import` 关闭该漏洞
为每个“语义上”拒绝的事件淘汰陈旧的规范文件
(`RefusedHookEvents`，`adapter.HookIngestGuard` 扩展；
结构畸形的原生形状——settings.json 拼写错误——发出警告，但从不发出警告
删除规范配置）。因为规范钩子是在代理之间共享的，
退休将事件交还给**每个**钩子渲染代理
范围：每个代理的钩子状态键都被否认 - 在代理的 *native* 下
重命名代理的事件拼写（Gemini `BeforeTool`、Cursor `preToolUse`、
通过 `adapter.HookEventNamer`) — 所以没有孤儿
清理工作将被触发，并且每个本地条目都将保持原样冻结。所以一个
导入→应用往返永远不会重写用户的本机 `/hooks/<event>`
有损阵列。它还拥有
每个支持 MCP 的适配器重用的共享 `Extra` 直通帮助程序：
`ExtraNativeKeys`（将未建模的本机字段捕获到`source.*Spec.Extra`中）和
`MergeExtra`（将它们投影回渲染）。两者都保留 `__` 前缀作为
agentsync-internal 命名空间 — 捕获和渲染对称 — 因此一个适配器的
合成元数据（Continuedev 的 `__block_version`/`__block_schema`）永远不能
泄漏到另一个代理的配置中；看到
[architecture.md § 8](architecture.md#8-secrets--how-the-leak-is-prevented)。
- **键：** `New(Options) *Adapter`； `Adapter` + `PluginIngester` 方法；
  `ParseFrontmatter`/`EncodeFrontmatter`; `MergeKeys`； `MergeExtra`/`ExtraNativeKeys`。
- **取决于：**适配器、秘密、源、路径、iox、jsonkeys。
- **文件：** `claude.go`、`homedir.go`、`render.go`、`ingest.go`、`ingest_plugins.go`、
  `apply.go`、`paths.go`、`frontmatter.go`、`skill.go`、`command.go`、
  `subagent.go`、`hook.go`、`lsp.go`、`memory.go`、`settings.go`、`extra.go`。

### `internal/adapter/opencode`
OpenCode 适配器 — MCP、内存、技能、子代理、通过 JSONC 的命令
往返 (`tailscale/hujson`)。跳过 Hook 和 LSP（报告时带有警告）。
- **键：** `New(Options) *Adapter`； `Adapter` 方法。
- **取决于：**适配器、秘密、源、路径、iox。
- **文件：** `opencode.go`、`homedir.go`、`render.go`、`ingest.go`、`apply.go`、`paths.go`、
  `skill.go`、`subagent.go`、`command.go`、`memory.go`、`settings.go`。

### `internal/adapter/codex`
Codex CLI 适配器 — MCP、内存、技能、子代理、斜线命令和
钩子。 MCP 服务器 (`[mcp_servers.*]`) 和挂钩 (内联 `[hooks.*]`) 都合并
通过 `merge-toml-keys` 策略进入 TOML `~/.codex/config.toml`
（`settings.go` 中的 `MergeTOML`，保留用户的外键） — 所以
config.toml 是适配器的单键合并文件；技能落入共享
`~/.agents/skills/`；子代理项目采用 Codex 的 TOML 代理格式和命令
仅限全局的自定义提示。
实现 `PluginIngester` （解析 `[plugins."<name>@<source>"]` 启用状态
在 `import` 上）；渲染不会在 `apply` 上重新发出这些表，以匹配
跨适配器不变式 — 请参阅
[architecture.md § PluginIngester（只读）](architecture.md#pluginingester-read-only)。
跳过 LSP（Codex 没有 LSP 概念）。 Hook 摄取具有共享的防护和警告
姿势并实现 `adapter.HookIngestGuard` (`RefusedHookEvents`
相同的 `toml.Unmarshal` 解析 Ingest 使用），但有一个故意的分歧
claude/gemini/cursor 双胞胎：非 `command` 处理程序 *type* 是可表示的
（Codex 解析并跳过未知类型；Render 逐字重新发出 `Type`
报告减少了 Skip），因此从未拒绝 - 仅未建模的字段
触发退休。
- **键：** `New(Options) *Adapter`； `Adapter` + `PluginIngester` 方法；
  `MergeTOML`； `IngestMCPSpec`。
- **取决于：**适配器、适配器/克劳德（frontmatter helpers）、秘密、来源、
  路径、iox、jsonkeys、go-toml/v2。
- **文件：** `codex.go`、`homedir.go`、`render.go`、`mcp.go`、`ingest.go`、`ingest_plugins.go`、
  `apply.go`、`paths.go`、`skill.go`、`command.go`、`subagent.go`、`hook.go`、
  `memory.go`、`settings.go`。

### `internal/adapter/cursor`
光标适配器 — MCP、内存、技能、子代理、斜线命令和挂钩。
MCP 落在 `.cursor/mcp.json`（与 Claude 相同的 `mcpServers` 形状）并钩住
在 `.cursor/hooks.json` (`{ "version": 1, "hooks": { … } }`) 中 — 都是 JSON，所以
适配器的单键合并策略是`merge-json-keys`。所需的挂钩
`version` 合并后注入到 `applyWrite` 中（从未渲染到 `op.Content` 中，
所以它永远不是一个孤儿可剥离的拥有的密钥）。内存项目到 repo-root
`AGENTS.md` 仅在项目范围内（用户级规则位于 Cursor 的应用程序本地
存储）； `.cursor/skills/` 的技能； `.cursor/agents/<name>.md` 的子代理
（`tools`/`color` 已删除）；命令 `.cursor/commands/<name>.md` （普通
markdown——frontmatter 被删除）。跳过LSP（光标没有LSP概念）。
尚未实现 `PluginIngester` — 光标的本机插件启用状态位置
未记录，因此 `import` 上的插件发现被推迟； `apply` 仍然是粉丝
像每个适配器一样输出插件组件。 Hook 摄取具有相同的
克劳德/双子座的守卫和警告姿态——一个无法代表的事件是
拒绝完整，从未有损地捕获 - 并实施
`adapter.HookIngestGuard` (`RefusedHookEvents`，报告拒绝事件
他们的*规范*名称），因此游标端本机丰富会触发导入
就像克劳德或双子座的人一样，过时的退休生活。
- **键：** `New(Options) *Adapter`； `Adapter` 方法； `IngestMCPSpec`。
- **取决于：**适配器、适配器/克劳德（frontmatter/skill/extra helpers）、
  秘密、来源、路径、iox、jsonkeys、afero。
- **文件：** `cursor.go`、`homedir.go`、`render.go`、`mcp.go`、`ingest.go`、`apply.go`、
  `paths.go`、`skill.go`、`command.go`、`subagent.go`、`hook.go`、`memory.go`。

### `internal/adapter/gemini`
Gemini CLI 适配器 — MCP、内存、斜线命令、子代理和挂钩。 MCP
(`mcpServers`，带有 Gemini 的 `url`/`httpUrl` 传输分割) 和 hooks (`hooks`，
与 Claude 相同的嵌套形状）都通过合并到 `.gemini/settings.json`
`merge-jsonc-keys` — settings.json 是适配器的单键合并文件，因此
用户的其他键（`theme`、`model`、...）被保留。内存项目
`GEMINI.md` (`~/.gemini/GEMINI.md` 用户/repo-root `GEMINI.md` 项目);命令
到 `.gemini/commands/<name>.toml` (`description` + `prompt`);分代理
`.gemini/agents/<name>.md`。跳过技能（Gemini 使用扩展，而不是 Agent
技能）和 LSP（无 LSP 概念）——两者都 ✗ 跳过。没有 `PluginIngester`（没有
本机插件启用状态代理同步模型）。 Hook 摄取具有相同的
像克劳德那样的警惕和警告姿态——一个无法代表的事件被整个拒绝，
从未有损地捕获 — 并且实现了 `adapter.HookIngestGuard`
（`RefusedHookEvents`，以其*规范*名称报告被拒绝的事件），
所以双子座一方的本地丰富触发了进口的陈旧挂钩退休
就像克劳德那边的那样。
- **键：** `New(Options) *Adapter`； `Adapter` 方法； `IngestMCPSpec`。
- **取决于：**适配器、适配器/克劳德（frontmatter helpers）、秘密、来源、
  路径、iox、jsonkeys、go-toml/v2。
- **文件：** `gemini.go`、`homedir.go`、`render.go`、`mcp.go`、`ingest.go`、`apply.go`、
  `paths.go`、`command.go`、`subagent.go`、`hook.go`、`memory.go`。

### `internal/adapter/continuedev`
Continue 适配器（包 `continuedev` — `continue` 是 Go 关键字；
代理名称仍为 `continue`）。 MCP、内存和斜杠命令，投影为
继续“块”——每个项目一个文件，因此**没有密钥合并**
（`KeyMergeStrategy()` 返回 `""`）：MCP → `.continue/mcpServers/<id>.yaml`
（stdio 命令/args/env；远程 `streamable-http`/`sse` + `url` +
`requestOptions.headers`）；内存 → `.continue/rules/agentsync.md` (a
Frontmatter-less 始终适用规则）；命令 → `.continue/prompts/<name>.md`
提示块。技能/子代理/挂钩/LSP 没有忠实的“继续”目标并且
会被报告跳过（Skill/Subagent/Hook/LSP）。否
`PluginIngester`。
- **键：** `New(Options) *Adapter`； `Adapter` 方法； `IngestMCPSpec`。
- **取决于：**适配器、适配器/claude（frontmatter/Extra helpers）、秘密、
  源、路径、iox、sigs.k8s.io/yaml。
- **文件：** `continue.go`、`homedir.go`、`render.go`、`mcp.go`、`ingest.go`、`apply.go`、
  `paths.go`、`command.go`、`memory.go`。

### `internal/adapter/windsurf`
Windsurf（级联）适配器 — MCP、内存和斜线命令、**范围-
不对称**以匹配 Windsurf 的布局：只有 **MCP** 仅限用户范围
(`~/.codeium/windsurf/mcp_config.json`, JSON `mcpServers` 通过 `merge-json-keys`;
远程使用 `serverUrl`），在项目范围内跳过（报告）。 **内存**和
**命令**在**两个**范围内渲染 - 项目 → `.windsurf/rules/agentsync.md`
(工作空间规则) / `.windsurf/workflows/<name>.md`, 用户 → 全局规则文件
`~/.codeium/windsurf/memories/global_rules.md` / `~/.codeium/windsurf/global_workflows/`
- 所有简单的降价。技能/子代理/挂钩/LSP 没有 Windsurf 概念，并且是
跳过了。它**实现`WarnEmitter`**：`Ingest`在工作空间时发出警告
规则缺少 agentsync-rendered `trigger: always_on` frontmatter。否
`PluginIngester`。
- **键：** `New(Options) *Adapter`； `Adapter` 方法； `IngestMCPSpec`。
- **取决于：**适配器、适配器/克劳德（额外助手）、秘密、来源、
  路径、iox、jsonkeys。
- **文件：** `windsurf.go`、`homedir.go`、`render.go`、`mcp.go`、`ingest.go`、`apply.go`、
  `paths.go`、`command.go`、`memory.go`。

### `internal/adapter/roo`
Roo Code 适配器 — 通过干净文件系统的 MCP、内存和斜线命令
`.roo/` 路径。 MCP → `.roo/mcp.json`（项目级，`mcpServers` 通过
`merge-json-keys`；远程使用显式 `type: streamable-http`/`sse`)；记忆 →
`.roo/rules/agentsync.md`（简单的降价规则）和命令→
`.roo/commands/<name>.md`（markdown + frontmatter — 保留 `description` +
`argument-hint`），都在用户*和*项目范围内。 Roo 的全局 MCP 是 VS Code
globalStorage（非目标 — 用户范围 MCP 被报告为跳过）。跳过
技能/子代理/Hook/LSP。没有`PluginIngester`。
- **键：** `New(Options) *Adapter`； `Adapter` 方法； `IngestMCPSpec`。
- **取决于：**适配器、适配器/claude（frontmatter/Extra helpers）、秘密、
  源、路径、iox、jsonkeys。
- **文件：** `roo.go`、`homedir.go`、`render.go`、`mcp.go`、`ingest.go`、`apply.go`、`paths.go`、
  `command.go`、`memory.go`。

### `internal/adapter/cline`
Cline 适配器 — MCP、内存和斜线命令，**范围不对称**：MCP
在用户范围内渲染到 Cline CLI 的干净 `~/.cline/mcp.json`
(`merge-json-keys`; 传输推断，无 `type` — 远程使用 `url`+`headers`),
while 内存（`.clinerules/agentsync.md`，普通 markdown）和命令
(`.clinerules/workflows/<name>.md`, plain markdown) 在项目范围内渲染；的
不适用的范围报告跳过。 Cline 没有项目 MCP 文件（其 VS Code
扩展使用操作系统/编辑器特定的全局存储代理同步不写）及其
全局规则位于 `~/Documents/Cline/` 中（也不是目标）。技能/子代理/
hooks/LSP 没有 Cline 概念，因此会被跳过。不发出摄取警告
（规则/工作流程是普通的降价），因此它没有实现 `WarnEmitter`。否
`PluginIngester`。
- **键：** `New(Options) *Adapter`； `Adapter` 方法； `IngestMCPSpec`。
- **取决于：**适配器、适配器/克劳德（额外助手）、秘密、来源、
  路径、iox、jsonkeys。
- **文件：** `cline.go`、`homedir.go`、`render.go`、`mcp.go`、`ingest.go`、`apply.go`、
  `paths.go`、`command.go`、`memory.go`。

### `internal/adapter/generic`
**广度层**适配器：一种数据驱动的 `Adapter` 实现，服务于
许多代理来自经过验证的 `Spec` (`specs.go`) 表，而不是包
每个。涵盖**内存**（规则/说明文件，纯降价），**MCP** 其中
代理读取agentsync可以表达的JSON服务器映射，以及**代理技能**
（`SKILL.md` 目录），代理本机扫描技能目录 — 每个
其他组件被报告为跳过。 `Spec` 声明每个作用域的内存/MCP/技能
路径加上捕获尾部方差的 MCP“方言”旋钮（顶级键
`mcpServers`/`servers`/`mcp`/`context_servers`/平面命名空间 `amp.mcpServers`；
传输字段 `type`/`transport`/推断； stdio 值 `stdio`/`local`；远程
网址键 `url`/`httpUrl`/`serverUrl`）。 MCP 合并是 JSONC 容忍的 (hujson)，因此
注释的设置文件（Zed/Copilot/Amp）被保留，而不是被破坏（重新发出
作为纯 JSON，如 OpenCode）。技能不需要方言——磁盘上的格式是
统一 - 因此该层重用深层适配器的共享 `claude.SkillFileOps`
投影；代理的 `Skills` 目标通常是跨供应商 `.agents/skills/`
（与 Codex 字节相同，因此渲染管道会删除操作的重复数据）。广度剂
通过普通注册表注册并通过应用/导入流程（漂移、秘密、
捕捉）。添加代理是经过验证的表行，而不是包。
- **键：** `Spec`，`New(Spec, Options) *Adapter`； `Adapter` 方法； `Specs()`。
- **取决于：**适配器、适配器/claude（额外+ SkillFileOps 帮助程序）、秘密、
  源、路径、iox、jsonkeys。
- **文件：** `generic.go`、`homedir.go`、`render.go`、`ingest.go`、`apply.go`、`specs.go`。

### `internal/adapter/noop`
占位符适配器检测 true 并且不渲染任何内容。用作注册表
测试中的替身；今天没有生产代理注册为 noop（每个有效的
代理有一个真正的适配器）。 `agent add`/`import` 仍然拒绝任何未来
noop 注册代理，除非 `AGENTSYNC_ALLOW_UNIMPLEMENTED=1`。
- **取决于：**适配器、秘密、源。 **文件：** `noop.go`。

---

## 管道和状态

### `internal/render`
编排应用：规范+注册表→每个代理`FileOp`/`Skip`，运行
碰撞检测和备份、记录状态并构建翻译
报告。它回收两种孤儿：清空的键合并部分（合成的
孤立拥有的密钥的清理操作）和**整个文件组件**，其
`source_id` 位于 `skills/`、`subagents/`、`commands/` 下，或已停用
`agents/` 拼写 — 源不再渲染的目标将被删除，
首先备份手动编辑，然后跳过（保留状态条目，因此下一个
当无法读取时应用重试。 `Plan` 也验证
每个成为目标文件名的组件 ID — 子代理/命令/技能
`Name` 加上 MCP/LSP 服务器 ID（每个服务器文件适配器，如 continuedev 加入
id 到路径中）——针对任何适配器之前的 `source.ValidateComponentID`
将其加入目标文件名 - 渲染时路径遍历防护，
与目标→源写入边界对称（参见架构§7）。
- **键：** `Plan`； `Apply`； `PreviewApply`（试运行：碰撞预览 +
  同步/将改变判决）； `Writer`
  (`NewWriter`/`NewPreviewWriter`); `TranslationReport` (`PrintText`/`PrintJSON`);
  `BuildReport`； `RecordOpsState`； `PruneStaleState`；
  `BackupFile`/`PruneBackups`; `CollisionReport`；以及孤儿回收
  trio — `OrphanFiles`（哪个国家拥有但该计划不再渲染），
  `OrphanDeletes`（删除应用将执行），`OrphanIsReclaimable`（是
  该组件 KIND 完全被回收 — 驱动 reconcile 的提示措辞）并且
  `OrphanDeleteWillProceed`（这个目的地实际上会被删除吗？
  run — 使应用摘要不计算跳过的删除）。 `IsRegularOrAbsent`
  与 `internal/cli` 的目标读取共享，因此 FIFO 无法阻止它们。
- **取决于：**适配器、秘密、源、状态、路径、iox、漂移。
- **文件：** `pipeline.go`、`writer.go`、`state_apply.go`、`report.go`。

### `internal/capture`
单一目标→源回写路径：重新引用秘密，保留
仅源字段，通过 `source.Write*` 写入。由 `import` 使用并协调。
- **键：** `Capture(home, ingested, opts) (Result, error)`； `Opts`； `Result`。
- **取决于：**源、秘密、路径、iox。
- **文件：** `capture.go`、`leak_fixture.go`（编译时泄漏防护）。

### `internal/drift`
纯 3 路分类器——无 IO。
- **键：** `Class` (`Clean`、`Pending`、`Drift`、`Converged`、`Conflict`、`New`、
  `ForeignCollision`、`Orphan`、`OrphanDrifted`）； `Classify(hsrc, happlied, hdest)`；
  `SafeForAutoApply(c)`。
- **文件：** `classifier.go`。

### `internal/state`
保留最后应用的哈希值和插件/市场引脚
`.state/targets.json`；使用迁移器进行模式版本控制。还拥有
每台机器的运行记录 `.state/last-run.json`，它支持一次性
升级后首次运行通知 - 故意的单独文件：它必须是
可通过只读命令写入，并且绝不能选通（或碰撞）漂移
状态的 `SchemaVersion`。
- **密钥：** `SchemaVersion`； `Targets` (`Files`, `Keys`, `Marketplaces`,
  `Plugins`）； `FileEntry`； `KeyEntry`； `Load`/`Save`; `migrate`；
  `LastRun`/`LoadLastRun`/`SaveLastRun`。
- **取决于：** iox。 **文件：** `schema.go`、`store.go`、`migrate.go`、
  `lastrun.go`。

### `internal/marketplace`
对 Claude 市场/插件格式进行建模，获取源代码和项目插件
体现为规范组件。
- **键：** `Marketplace`、`PluginEntry`、`Source`、`PluginManifest`；
  `ProjectionResult`； `Project`/`ProjectWithReader`; `ProjectInstalled`
  （一个独立安装的插件 - 让 `explain <id>` 将覆盖范围归因于
  命名的插件而不是扁平化的联合）； `Fetcher`（界面）与
  `GitFetcher`/`NPMFetcher`/`RelativeFetcher`; `LoadProjected`/
  `LoadProjectedLenient`/`LoadProjectedExcluding`; `namespaceProjected`（重命名
  每个插件提供的子代理/技能/命令到 `<plugin>-<name>` 并标记其
  出处 - 包括提供插件的 `agents`/`native_agents`
  定位，它与组件一起移动，因为扁平化的规范
  删除关联 - 因此两个传递同一个名称的插件不能同时发生冲突
  目标路径 — 请参阅 Architecture.md § 插件组件命名空间）。
- **取决于：** 来源、日志。
- **文件：** `manifest.go`、`treehash.go`（`tree:v1:` 内容哈希），
  `projection.go`、`loadprojected.go`、`fetcher.go`、`fetch_git.go`、
  `fetch_npm.go`、`fetch_relative.go`、`update.go`。

---

## 基础设施和演示

没有内部依赖项的叶包，加上精简的 `ui` 演示
层（仅基于 `untrusted` 构建）。

### `internal/git`
代码库中**唯一** `go-git` 表面：仅限本地、目录级
目标 git 备份的回滚历史记录（问题 #118）。每个管理
目标目录成为其自己的存储库，带有 `[agentsync] managed = true`
标记，以便 agentsync 只自动提交到它创建的存储库中；检查点是
每次应用后记录，并且 `revert` 回滚目录仅追加。它暴露了
**无** 远程/推送 API — 由源扫描 `TestNoPushSurface` 强制执行
保护 - 因此备份永远不会离开机器（历史记录可能保存
渲染文件已包含的明文秘密）。
- **键：** `Detect`/`State` (`StateUntracked`/`StateAgentsyncOwned`/`StateForeign`;
  `State.String()` → `agentsync-versioned` / `foreign source control` / `untracked`);
  `Init`/`Open`/`OwnsExactly`/`HasNestedRepoBelow`； `Stage`/`StageTrackedDeletions`/
  `CommitStaged`/`SnapshotDirtyTracked`/`IsClean`; `Log`/`Resolve`/`Plan`/`Restore`；
  `Identity`； `NoticeFile`。
- **取决于：**没有任何内部（叶）。
- **文件：** `git.go`、`init.go`、`commit.go`、`log.go`、`restore.go`、`perms.go`。

### `internal/iox`
原子文件 IO 和锁定。
- **键：** `AtomicWrite(dest, data, mode)`； `Lock`/`AcquireLock`/
  `AcquireLockTimeout`； `ErrSymlinkDest`； `AllowSymlinkDestEnv`。
- **文件：** `atomic.go`、`lock.go`。

### `internal/jsonkeys`
每键 JSON 指针合并，保留外键并使用 `json.Number`
（没有 float64 舍入）。
- **键：** `DecodeObject`； `DecodeYAML`； `MergeKeys(existing, ours, ownedPointers)`。
- **文件：** `jsonkeys.go`。

### `internal/paths`
解决`AGENTSYNC_HOME`、`AGENTSYNC_TARGET_ROOT`和`$HOME`；之间转换
可移植状态的绝对形式和 `${HOME}` 相对形式。
- **键：** `Env`（接口），`OSEnv`，`MapEnv`； `HomeDir`； `AgentsyncHome`；
  `HomeRelative`/`FromHomeRelative`。
- **文件：** `paths.go`。

### `internal/log`
slog 设置。在 `ui.SlogHandler` 上构建一个记录器并将其安装为
来自 root 命令的 `PersistentPreRunE` 的进程范围默认值，这就是
将库端 `slog` 调用 (`internal/render`, `internal/marketplace`) 路由到
CLI 的诊断词汇表而不是 stdlib 默认的时间戳
线。 **关键：** `New(w, p, verbose) *slog.Logger`； `Install(w, p, verbose)`；
`Detach()`（将进程默认解除绑定到丢弃处理程序 - 接缝测试
二进制文件需要，因为每个 `Execute()` 否则留下 `slog.Default()` 指向
完成的测试的缓冲区）。 **取决于：** 用户界面。 **文件：** `log.go`。

### `internal/untrusted`
获取的/本机元数据的显示信任边界。拥有 `Sanitize`（条
终端控制 + 欺骗性的 bidi/零宽度符文）和 `Text` 定义的字符串
`String()` 清理的类型 — 因此是插件/市场 ID、版本或名称
输入的 `untrusted.Text` 可以通过构造安全地通过 `fmt` 进行打印；原始的
值只能通过显式的 `Unverified()` 访问。 `ui.Sanitize` 代表
在这里。请参阅[体系结构 §7](architecture.md#7-safety-primitives) 和 `SECURITY.md`。
- **键：** `Text` (`.String()` / `.Unverified()` / `.Empty()`); `Wrap`； `Sanitize`。
- **文件：** `untrusted.go`。

### `internal/ui`
表示层 - 每个命令都通过 `*Printer` 呈现样式输出
因此颜色、字形和间距决策都集中在一处。拥有策划的字形
词汇表 (`✓`/`◐`/`✗`/`⚠`/`•`/`→`)、`--color` 模式分辨率、
**诊断词汇**（如下），以及重写的 `WarnWriter`
`warning: ` 发射器哨兵进入共享 WARN 标签。 `Sanitize` 代表
`internal/untrusted`，因此通过 `ui` 打印的不受信任的元数据被删除
通过构造实现终端控制和欺骗性格式符文。

**诊断与结果。**输出分为两部分，每个 `ui` API 属于
一侧：

| |它是什么 |流 |渲染|
| ---| ---| ---| ---|
| **诊断** | *关于*这次跑步的通知 |标准错误| `✗ ERROR` / `⚠ WARN` / `ℹ INFO` / `• DEBUG`，第 9 列消息 |
| **结果** |要求命令生成什么 |标准输出|无等级标签；精心策划的表情符号带来成功的结果 |

级别标签是 `<glyph> <WORD padded to 5>` 加上两列装订线，所以
每个级别的消息都从同一列开始，并且 `Detailf` 继续
线条悬挂在它们下面。成功行带有 **无** 级别词 — 上的 `INFO`
`added agent: claude` 是噪音，会淡化重要的标签 - 相反
使用 `EmojiSuccess ✅` / `EmojiApplied 🎉` / `EmojiRemoved 🧹` / `EmojiImported 📥`
/ `EmojiReverted 🔙` / `EmojiInit ✨`，由发生的事情而不是由
运行了哪个命令。

`SlogHandler` 通过相同的词汇表呈现 `log/slog` 记录，并且是
由 root 命令作为进程范围默认安装，因此 `slog.Warn` 来自
`internal/render` 或 `internal/marketplace` 与命令本身的字节相同
`p.Warnf`。记录时间戳被删除：这些是面向用户的 CLI 诊断，
不是任何人按时间 grep 的日志流。
`Handle` 锁定其写入者（一条记录是一个标签行加上一个可选属性
行，不得交错）并返回写入错误而不是吞咽
他们。 `slog.Warn` / `WarnWriter` / `p.Warnf` 三元组字节相同是
由 `TestWarnPathsAreByteIdentical` 固定。

**谁清理。** `SlogHandler.Handle` 和 `reportErrorTo` 的终端错误行
（通过`sanitizeLines`）清理自己的输入，因为两者都没有调用站点
可以：日志记录和包装的错误链都在其他地方组装。其他地方的
**调用者** 在显示边界应用 `ui.Sanitize`，因为其余部分
代码库确实如此 - `Diagf`/`Detailf`/`Successf` 系列故意不这样做，所以
调用者可以传递预先设置样式的文本（升级通知横幅组成
`p.Bold`/`p.Yellow` 片段成 `Warnf`) ，没有自己的转义码
被剥夺了。

`WarnWriter` 是第三个自我清理网站，原因与其他网站相同
二：它的输入来自适配器和捕获包，*不能*调用
`ui.Sanitize`（它们不得导入 `ui` — 该约束是
`warning: ` 哨兵存在），因此如果 `ui` 不进行清理，则不会执行任何操作。它
仅清理消息**正文**；直通分支保持原样，因为
该分支带有 agentsync 自己的已样式化行，其 ANSI 清理
会脱衣。

该逆止器不会取代发射器处的 `%q`，其界限值得说明
确切地说，因为上面的不对称很容易被过度解读。 `WarnWriter.Write` 分裂
在 `\n` *之前* `emit` 运行，因此清理后的主体永远不会包含内部
换行符。将原始换行符插入 `warning: ` 行的发射器
第 2..n 行穿过 **passthrough** 分支 — 未标记且未经消毒 — 因此对于
换行符 `%q` （转义它）仍然是控件。几乎每个发射器
已使用 `%q` 作为不受信任的标识符；剩下的就是少数
`%v`错误站点，其中多行换行错误可能携带本机配置文本
进入直通分支。已有的；不是由诊断引入的
词汇，而不是被它封闭。

请注意，`%s` 位于 `untrusted.Text` 值上 — 这就是插件和市场的方式
身份字段被键入 - 从来都不是一个漏洞：`Text.String()` 清理
建设。差距只是从本机配置中读取的普通 `string` 值。
- **Key:** `Printer` (`New`, `Color`, `Section`, 颜色助手；颜色已解析
  每个流，以及其目标流的诊断编写器样式）；
  `Level` + `Label`; `Errorf`/`Warnf`/`Infof`/`Diagf`/`Fdiagf`；
  `Detailf`/`Fdetailf`; `Successf`/`Fsuccessf` + `Emoji*` 词汇表；
  `SlogHandler`/`NewSlogHandler`;包级 `Pad` 帮助器；
  `ColorMode`/`ParseColorMode`; `Glyph*` 词汇表； `WarnWriter`
  (`NewWarnWriter`, `RouteTo`, `Flush`); `Sanitize`。
- **取决于：** 不受信任。
- **文件：** `ui.go`、`diag.go`、`slog.go`、`spinner.go`。

### `internal/testenv`
保护 FS 接触测试，因此它们仅在密封容器中运行。
- **密钥：** `RequireContainer(t)`； `MustRunInContainer()`； `InContainer() bool`；
  `EnvVar` (`AGENTSYNC_TEST_IN_CONTAINER`)。
- **文件：** `container.go`。

---

## 依赖方向一目了然

`cli` 位于一切之上。 `render`、`capture` 和适配器取决于
`source` + `secrets`。 `source`/`secrets`/`state` 仅取决于叶子基础设施
包（`iox`、`jsonkeys`、`paths`，以及 - 对于规范插件/市场
身份字段输入 `untrusted.Text` — `untrusted`）。 `git`（目的地
仅从 `cli` 到达的回滚历史记录同样是叶子。 `drift`、`git`、
`iox`、`jsonkeys`、`paths` 和 `untrusted` 不依赖于任何内部 —
他们是基础； `ui`（演示文稿）仅基于 `untrusted` 构建，并且
`log` 仅在 `ui` 上（它以默认方式连接 `ui.SlogHandler`）。请参阅
渲染的依赖图
[架构§12](architecture.md#12-package-layering)。

### `internal/governance`

嵌入式跨项目Agent治理模型。它拥有通用的Baseline，
项目技术配置文件、明确的每个代理能力注册表，以及
`governance` CLI 预检使用的出处清单。它不拥有
业务规则或直接编写原生Agent文件。

### `internal/adapterregistry`

所有深度和通用适配器的共享生产布线。 CLI 和桌面
核心在这里构建他们的注册表，以便代理覆盖和验证目的地
接口之间的路径不能分叉。

### `internal/desktopcore`

macOS Tauri 客户端的应用程序边界。快照读取组成
具有有限/手动项目发现和编辑本机的规范加载器
库存。规则命令仅编辑规范内存，从中导出目标路径
共享适配器，对漂移进行分类，通过 `render.Writer` 应用，并持久化
正常目标状态。可选的 Codex 分析器返回可审查的建议；
它没有写权限。

### `cmd/agent-assistant-core`

桌面 shell 使用的本地 JSON 行 sidecar 进程。它接受
快照/预览加上规则加载、保存、同步、本机导入、项目导入和
标准输入上的项目分析方法并向标准输出写入一个 JSON 响应。它有
没有网络侦听器； Tauri 通过 `AGENT_ASSISTANT_CORE_BIN` 找到它，
同级应用程序二进制文件，或 `PATH`。项目发现默认为 `$HOME/git/work`
并且可以使用 `AGENT_ASSISTANT_PROJECTS_ROOT` 绑定到另一个根。