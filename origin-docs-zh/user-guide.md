<div align="center">

# agentsync — 用户指南

**您机器上每个 AI 编码代理的一个事实来源。**

*一次*定义您的 MCP 服务器、内存、技能和市场插件。
运行`agentsync apply`。看着他们着陆——正确翻译——穿过**31
代理**：九个深度适配器（Claude Code、OpenCode、Codex CLI、Cursor、Gemini CLI、
Continue、Windsurf、Roo Code、Cline）加上 22 个代理广度层（amp、goose、
qwen、warp、zed、kiro、junie、factory、copilot、crush、...）。

[为什么选择 agentsync](#why-agentsync) · [安装](#install) · [您的第一次同步](#your-first-sync-5-minutes) · [已有配置？](#already-have-configs) · [每日循环](#the-daily-loop) · [构建配置](#building-your-config) · [命令参考](#command-reference)

</div>

---

## 为什么代理同步

如果您使用多个 AI 编码代理，您会感觉到这一点：您添加了一个 MCP 服务器
给 Claude，然后手动将其复制到 OpenCode 的 JSON 中，然后再次复制到 Codex 的 TOML 中。
您在其中一个中安装了一个插件，而在其他中则忘记了它。您对令牌进行硬编码
写入配置文件并祈祷它永远不会出现在 git 中。您的 `~/.claude.json` 和您的
OpenCode 配置慢慢地分开，您不知道哪一个是“正确的”。

agentsync 修复了扇出问题。您在 `~/.agentsync/` 中保留**一个规范配置**
— 小型、可手动编辑的 TOML 和 Markdown 文件，您可以提交到 dotfiles 存储库
— 并且agentsync 将其投影为每个代理的*本机* 格式。添加服务器
一次；它无处不在。安装一次插件；每个了解的代理商
它的组件得到它们。将秘密引用为 `${secret:github.token}`；这是
在应用时解决并且**从未**以明文形式写回。

因为代理编辑自己的配置，所以代理同步是**双向的**：它
注意到本机文件何时偏离其上次写入的内容并提供
chezmoi 式合并 — 将编辑内容采用到源代码中，或重新强加源代码。
没有任何东西会在你背后被覆盖，也没有任何东西会丢失。

> **承诺：**在一处编辑，应用一次，相信结果 - 与您一起
> 秘密安全，你的倾向可见。

---

## 60 秒心智模型

三种状态，一种比较。 （完整版本位于[概念](concepts.md)。）



```
   ~/.agentsync/            apply            ~/.claude.json
   (your source)   ───────────────────▶     ~/.config/opencode/…
   TOML + markdown     render + translate    (what agents read)
        ▲                                            │
        └──────────── reconcile / import ────────────┘
                  (capture native edits back)
```



- **来源** — 您在 `~/.agentsync/` 中提交的内容。
- **`apply`** — 渲染源并写入每个代理的本机配置。
- **漂移** — 代理（或您）编辑了本机文件； `status`/`diff` 显示它。
- **`reconcile`** — 将该编辑合并回源代码，或覆盖它。

这就是整个工具。下面的一切都是细节。

---

## 安装

> **测试版说明：** 下面的包管理器通道已连接并发布
> 从第一个标记版本开始。在那之前，**从源代码构建。**

**来自来源（今天有效）：**



```bash
go install github.com/spxrogers/agentsync/cmd/agentsync@latest
# or: git clone … && go build ./cmd/agentsync
```



**macOS — 自制程序**



```bash
brew tap spxrogers/tap
brew install agentsync
```



**Windows — 勺子/巧克力**



```bash
scoop bucket add spxrogers https://github.com/spxrogers/scoop-bucket
scoop install agentsync
# or:
choco install agentsync
```



**Linux** — [发布页面](https://github.com/spxrogers/agentsync/releases) 上的 `.deb`/`.rpm`。
（AUR 包装已连接，但尚未发布 - [问题 #13](https://github.com/spxrogers/agentsync/issues/13)。）

验证：



```bash
agentsync --version
```



---

## 您的第一次同步（5 分钟）

这是全新的路径 — 干净地开始，以两个 MCP 服务器结束
代理。



```bash
# 1. Create ~/.agentsync/ and its layout.
agentsync init

# 2. Register the agents you use.
agentsync agent add claude
agentsync agent add opencode

# 3. Add an MCP server once — it will fan out to both agents.
agentsync mcp add github \
  --command npx \
  --args "-y,@modelcontextprotocol/server-github"

# 4. Preview before writing anything. Always safe; never touches disk.
agentsync apply --dry-run

# 5. Apply for real.
agentsync apply
```



确认它已登陆两个本机配置：



```bash
jq '.mcpServers.github' ~/.claude.json
jq '.mcp.github'        ~/.config/opencode/opencode.json
```



> **`apply --dry-run` 是你的朋友。** 它列出了适用的每个目的地
> 会触摸，标记每个 `✓ synced` （已经保存了我们的确切字节）或
> `→ write`（将被创建或更改）-带有`— N to write, M already
> synced`计数-因此干净的重新应用读取为无操作而不是一堵墙
> “写”。它还打印完整的[翻译报告](#multi-agent-fan-out) —
> 哪些是原生的 (✓)，哪些是预计会损失的 (◐)，以及哪些是被跳过的
> (✗) — 并预览任何外部冲突备份，所有这些都无需写入字节。
> 在每次实际应用之前运行它，直到您信任输出。

---

## 已经有配置了吗？

大多数人一开始都不是干净的——你已经带着服务器和插件来了
在 Claude 或 OpenCode 中配置。将它们置于管理之下，而不是
重新输入它们。



```bash
# See what's on disk vs. what agentsync would write.
agentsync status

# Pull native config into your canonical source.
# Selector grammar: <agent>[:<component>[:<name>]] — drop parts to widen scope.
agentsync import claude --dry-run       # preview what a full import would write
agentsync import claude                 # the agent's full native config
agentsync import claude:mcp             # every MCP server
agentsync import claude:mcp:github      # a single MCP server
agentsync import claude:plugin          # every installed plugin + marketplace
agentsync import opencode:subagent:reviewer
agentsync import opencode:mcp:linear

# Now it's in ~/.agentsync/ — apply to fan it out to your other agents.
agentsync apply
```

删除名称会导入组件的每个条目；删除组件
也导入代理拥有的所有内容（MCP、技能、子代理、命令、挂钩、
LSP、内存、**和插件**）一次性完成。批量导入找不到任何内容
组件报告它并干净地退出而不是出错。将 `--dry-run` 添加到
列出导入将在不触及 `~/.agentsync/` 的情况下写入的源文件。

**导入永远不会重新捕获插件自己的组件** - 子代理、技能、
命令、MCP 服务器、LSP 服务器和类似的挂钩（每个 *handler* 挂钩，因此
插件挂钩您也挂钩的事件永远不会花费您自己的处理程序）。申请后，插件的
组件位于代理的本机配置中，与其他组件没有区别
你自己写的 - 代理的配置没有提示agentsync放置了哪些
那里。导入它们会创建冲突的规范副本
在下一个应用中使用插件自己的，因此 `import` 会跳过它们并说明哪个
插件提供了每个。明确地命名一个
(`agentsync import claude:subagent:feature-dev-code-reviewer`) 是一个错误而不是
比静默无操作。您自己编写到本机配置中的组件是
仍能正常拍摄。要更改插件的组件，请在上游更改它，或者
运行 `agentsync plugin disable <id>` 以停止投影。 `reconcile` 拒绝
`[w]rite-back` 出于同样的原因 - 使用 `[o]verride` 恢复
插件的版本。

**插件是一种特殊情况。** `plugin` 组件（Claude 和 Codex）读取
代理安装的插件及其市场，并将每个插件重新提取到
agentsync 缓存，固定清单 SHA — 产生相同的工件 `marketplace
add` + `plugin add`。因为它重新获取，所以真正的插件导入（不是
`--dry-run`) 需要网络访问。插件的市场是从
首先是agentsync自己的注册市场，然后是代理的本机配置。一个
其市场未在两者中注册的插件 - 例如来自
Claude 的内置 `claude-plugins-official`（未出现在
`extraKnownMarketplaces`) 在您注册之前 — 被报告并
跳过；使用 `agentsync marketplace add <source>` 注册并重新导入。

导入的插件**不会投影回其来源的代理**，除非
你要求它。该代理已安装插件，并且应用永远不会禁用
插件位于另一个工具的插件管理器中 - 因此投影相同的组件
它们中的每一个都会重复。 `import` 提出录制
`native_agents = ["claude"]`；接受是默认的，拒绝则警告
您必须自己禁用 Claude 内部的插件。无论哪种方式其他
启用的代理获得完全扇出。参见
[哪个代理插件扇出](#which-agents-a-plugin-fans-out-to)。

**导入项目的本机配置。** `import <agent> --scope project`
（可选 `--project <path>`）读取代理的*本机项目范围*配置
（例如 `<root>/.claude/`）并将其捕获到项目源代码树中
`<root>/.agentsync/` 而不是您的用户 `~/.agentsync/`。它播种中央国家
与项目范围+根，所以下一个应用不会将这些文件视为
外来碰撞。插件被排除：命名的 `import claude:plugin:<name>
--scope project` 错误和批量 `import claude:plugin --scope project`
默默地跳过——插件是跨线束的用户范围概念。

在已填充的计算机上，**第一个**应用将看到预先存在的本机文件
没有写入并将它们视为 `foreign-collision`：它支持每个
`~/.agentsync/.state/backups/<timestamp>/` *在*写入之前。什么都没有丢失。
首先预览将使用 `agentsync apply --dry-run` 备份哪些文件。

---

## 每日循环

四个命令涵盖日常使用：

|命令 |当你运行它时|
|---|---|
| `agentsync apply` |编辑源代码后 - 将更改推送给代理。 |
| `agentsync status` | “什么不同步？” — 所有代理的摘要。 |
| `agentsync diff` | “让我看看究竟发生了什么变化。”秘密已被编辑。 |
| `agentsync reconcile` |代理编辑其配置 - 合并或覆盖漂移。 |

代理在您手下编辑文件后的典型会话：



```bash
agentsync status              # spot the drift
agentsync diff                # inspect it (resolved secrets are masked)
agentsync reconcile           # interactively resolve
```



在 `reconcile` 内，对于每个漂移项目：



```
~/.claude/settings.json#$.permissions.bash[2]   (drift)
  source:      "Bash(git push:*)"
  destination: "Bash(git push:*) Bash(npm publish:*)"

  [w]rite-back   [o]verride   [s]kip   [i]gnore   [d]iff   [q]uit
```

- **`w`** 将目标编辑采用到源中（以及所有的 `W`）。
- **`o`** 重新强加源，放弃编辑（`O` 全部）。
- **`i`** 停止跟踪此路径（将其添加到 `~/.agentsync/ignore.toml`）。
- **`s`/`q`** 跳过/退出。

编写脚本吗？ `--auto-writeback`、`--auto-override` 或 `--auto-safe`（仅
自动解决不会丢失工作的更改）。

---

## 回滚错误的应用

`apply` 可以将每个用户范围目标目录（`~/.claude`、`~/.codex`、...）保留在
它**自己的仅限本地 git 存储库**，在每次应用后记录检查点提交
在那里更改托管文件。 **即使是第一次应用也是可恢复的：**之前
apply 会覆盖目录，agentsync 会记录 **pre-apply benchmark** 提交
它要管理的文件的先前内容，因此应用检查点的父级
是真正的预申请状态——不存在“第一次申请无法撤消”的间隙。
预先存在的文件agentsync**没有**写入（代理的凭据、对话
转录本，您自己的草稿文件）被故意排除在版本控制之外
历史，所以它永远不会成为你的秘密的持久副本——它们是未被追踪的，所以
无论如何，恢复不会影响它们。另一方面：此类文件被**保留，而不是
版本化**——恢复永远不会删除它们，但因为它们从未被提交，
备份历史记录无法恢复*您*删除的内容。如果涂抹出现问题，请滚动它
背部：



```bash
agentsync revert claude              # undo the most recent apply to ~/.claude
agentsync revert claude --to HEAD~3  # roll back to an older checkpoint
agentsync revert --all --dry-run     # preview reverting every managed dir
```



`revert` 是 **仅追加** - 它记录新的提交而不是重写
历史记录，因此错误的应用保留在日志中，并且恢复本身是可恢复的。如果
您在上次应用后手动编辑了 **跟踪** 文件，恢复编辑的快照
首先进入历史记录，所以**跟踪的任何内容都不会丢失**（使用 `revert --to
<snapshot>` 恢复它）。快照故意覆盖**仅跟踪的文件** - 未跟踪的
您放入托管目录中的临时文件永远不会提交并且位于外部
恢复快照（它们也不会被回滚影响，不会被删除）。
该快照是由回滚引擎本身拍摄的，因此保证仍然成立
调用恢复；在极少数情况下，回滚中途失败（例如磁盘已满），
错误命名快照提交和预恢复检查点，以便您可以恢复。
您放入目录中的任何**未跟踪的文件**（以及 gitignored 文件）都**留下
未触及** - 恢复仅倒回文件 agentsync 本身的版本，因此您自己的版本
暂存文件永远不会被删除。 `revert --dry-run` 注释此类文件何时
存在。如果您稍后将自己的 git 存储库克隆到*内部*托管目录（例如
`~/.claude/skills/.git`)，恢复拒绝对您的存储库签出进行硬重置
文件：命名代理 (`agentsync revert claude`) **错误**，而 `--all`
**跳过该目录**并发出警告并继续其余部分。 （严格遵循
调用 — 没有 `--strict` 标志。）它只移动*目的地*，所以之后它会提醒您
**在下一个 `apply` 重新渲染之前进行协调**（或修复规范源）
它。

第一个应用于未跟踪的目录**在初始化存储库之前**询问**
（选择退出）；当您同意时，它会采用预申请基线，因此首先申请
也被覆盖了。回答一次，它就会被记住在 `agentsync.toml` 中：



```toml
[destination_directory_git_backup]
mode = "on"          # "prompt" (default) | "on" | "off"
# author_name  = "agentsync"      # optional commit-identity overrides
# author_email = "agentsync@localhost"
```



`mode` 完全接受 `prompt`、`on` 或 `off`（区分大小写；省略
`mode` 默认为 `prompt`）。任何其他值 — 拼写错误，如 `"On"`、`"yes"` 或
`"true"` — 现在 **在加载时被拒绝**，并出现路径前缀错误，因此每个
读取配置的命令 (`apply`, `doctor`, …) 同意。 （之前有一个
无效值被默默忽略，只有 `doctor` 对此发出警告。）

`apply --no-git-backup` 跳过一次运行（CI/脚本）而不触及
config 和 `agentsync doctor` 显示当前模式和每个目录的状态。

单位是**目录**，而不是代理：每个代理的配置目录加上任何
它写入的共享跨代理目录（例如 `~/.agents/skills`，Codex 和几个
代理共享）是版本化的 - 共享目录被删除重复到单个存储库，并且
嵌套在另一个目录下的目录（例如 `~/.claude` 下的 `~/.claude/skills` ）被折叠到
父级，因此回购内永远不会有回购。因为共享目录是一个
共享目录的 repo，`revert <agent>` 会回滚其中*每个*代理的文件 —
当发生这种情况时，revert 会警告您。

这些存储库**从未被推送**。他们版本的渲染文件包含秘密
解析为**明文**（与规范源不同，它保留 `${secret:…}`
参考文献），这样历史就可以保守秘密——这很好，因为它
保持本地化。你提交并推送的东西仍然是`~/.agentsync/`，参考文献
仅。您已经保存在自己的 git 下的目标目录（例如 `~/.claude` 在
dotfiles repo）被检测到并且保持不变。 (`~/.claude.json`，直接写
在 `$HOME` 中，没有版本控制 — agentsync 永远不会在 `$HOME` 处初始化存储库；它保留了
现有的`.state/backups`安全网。）

---

## 构建你的配置

`~/.agentsync/` 只是文件。使用 CLI 或在 `$EDITOR` 中编辑它们 - 两者都是
一流的。布局：



```
~/.agentsync/
├── agentsync.toml            # agents, update defaults, secrets backend, [memory] banner, [destination_directory_git_backup]
├── mcp/<server>.toml         # one MCP server per file
├── lsp/<server>.toml         # one LSP server per file
├── subagents/<name>.md       # one subagent per file (NOT `agents/` — that
│                             #   word names the harness registry in agentsync.toml)
├── commands/<name>.md        # one slash command per file
├── hooks/<event>.toml        # one hook per file
├── marketplaces/<name>.toml  # one marketplace per file (its `head_sha`/`name`
│                             #   keys are CLI fetch-cache metadata, regenerated on
│                             #   fetch — not modeled in the canonical schema)
├── plugins/<id>.toml         # one plugin enablement per file (`agents` /
│                             #   `native_agents` decide which agents it renders to)
├── memory/AGENTS.md          # canonical memory (+ fragments/*.md)
├── skills/<name>/            # a skill is a DIRECTORY: SKILL.md + bundled
│   ├── SKILL.md              #   scripts/, references/, assets/, nested files —
│   └── scripts/ …            #   all carried verbatim, executable bit preserved
└── secrets/secrets.age       # age-encrypted secrets
```



存储库 `<root>/.agentsync/` 处的 **项目源代码树** 具有 *相同*
磁盘布局（由 `agentsync init --scope project` 创建） — `agentsync.toml`
加 `mcp/`、`lsp/`、`subagents/`、`commands/`、`hooks/`、`memory/` （
`fragments/`）、`skills/`、`plugins/` 和 `secrets/`。唯一的区别是：它有
**无 `.state/`** — 在 `~/.agentsync/.state/` 下集中应用记录状态，
由项目根键控。将 `.agentsync/` 树提交到存储库以共享项目
与协作者的代理配置。请参阅[项目本地配置](#project-local-config)。

### 代理



```bash
agentsync agent add claude        # register
agentsync agent list              # see registry + enabled state
agentsync agent list --all        # every supported agent (registered or not)
agentsync agent disable opencode  # stop applying to it (keeps source)
agentsync agent disable opencode --purge   # also remove what it wrote
```



每个代理命令还需要 `--scope project` / `--project <path>` 来管理
`<root>/.agentsync/agentsync.toml` 中的 **项目自己的** `[agents]` 声明
而不是用户注册表 - 项目范围仅呈现给声明的代理
那里（请参阅[项目本地配置](#project-local-config)）。在项目范围内，
`disable --purge` 仅删除该项目的渲染文件；在用户范围内，
`--purge` 清理该代理在每个范围和项目中渲染的文件
（历史行为）。

> 所有九个深度适配器（`claude`、`opencode`、`codex`、`cursor`、`gemini`、
> `continue`、`windsurf`、`roo`、`cline`）加上 22 个广度层代理
> `agent add` — 运行 `agentsync agent list --all` 获取完整集，或查看
> [能力矩阵](capability-matrix.md)。

### MCP 服务器



```bash
# stdio transport
agentsync mcp add github \
  --command npx \
  --args "-y,@modelcontextprotocol/server-github" \
  --env "GITHUB_TOKEN=\${secret:github.token}"

# http/sse transport
agentsync mcp add linear --type http --url https://mcp.linear.app/sse

# limit fan-out to specific agents
agentsync mcp add company-api --command npx --args "-y,@company/mcp" \
  --agents "claude,opencode"

agentsync mcp list
agentsync mcp remove github
```



默认情况下，服务器扇出到**所有启用的代理** (`--agents "*"`)。的
它写入的 `mcp/<name>.toml` 文件很小，可以手动编辑。

### 内存

您的规范内存位于 `memory/AGENTS.md` 中并呈现给每个代理的
本机文件（Claude 为 `CLAUDE.md`，OpenCode 为 `AGENTS.md`）。组成它来自
可重复使用的片段：



```markdown
<!-- ~/.agentsync/memory/AGENTS.md -->
# Coding conventions

@import ./fragments/style.md
@import ./fragments/security-rules.md
```



碎片**双向往返**。在 `apply` 上，agentsync 包装每个内联
原生文件中 HTML 注释边界标记中的片段：



```markdown
<!-- agentsync:fragment style.md -->
Be concise.
<!-- /agentsync:fragment style.md -->
```



因此 `import`/`reconcile` 可以反转扩展 - 本机内存编辑*内部*
片段块被捕获回该**片段文件**，并且 `@import`
结构被保留（编辑永远不会平铺到 `AGENTS.md` 中）。的
标记读取为元数据，而不是指令。如果标记缺失（a
其自己的文本包含标记令牌的片段会禁用它们）或者是
手工破坏成不平衡/模糊状态，agentsync 拒绝回写
而不是猜测；漂移仍然显示在 `status`/`diff` 中，你将其折叠到
`memory/` 手动。

### 在 macOS 客户端中管理规则

桌面规则工作台将规范内存视为规则母模板。
选择全局范围 (`~/.agentsync/memory/AGENTS.md`) 或导入项目的
范围，选择目标Agent，保存母规则，然后预览
同步。预览显示每个本地目的地及其漂移等级。

如果本机规则具有母模板中没有的编辑，则同步
停止。查看差异并选择一种明确的解决方案：导入本机
规则到母模板中进行审核，或者备份下的原生文件
`~/.agentsync/.state/backups/` 并覆盖它。不支持的代理/作用域对
不能强迫；取消选择它们或单独管理该本机功能。

项目视图在中注册本地路径
`~/.agentsync/.state/agent-assistant/projects.json`；单独导入写入
项目中没有任何内容。它使用经过验证的方法发现现有规则文件
适配器路径。相同的文件产生确定性的母规则候选。
当它们不同时，可选分析操作将使用已安装的 Codex CLI
只读的临时会话并返回提案。查看并保存
建议在同步任何代理文件之前。

**托管横幅。** 每个渲染的内存文件都前面加上一个简短的
agentsync 通知 — 命名文件的块引用（例如 `CLAUDE.md`）并指向
在 `.agentsync/memory/AGENTS.md` + `agentsync apply` 处编辑。它是由
agentsync，**不**存储在您的规范 `memory/AGENTS.md` 中：它包含在
`<!-- agentsync:managed memory-banner -->` 标记，已去除
`import`/`reconcile`，并重新渲染每个应用 - 所以它永远不会复合
（静态）永远不会显示为漂移。默认情况下它是打开的；选择退出
`agentsync.toml` 中的 `[memory]` 表：



```toml
[memory]
banner = false
```



`agentsync:managed` 标记是**保留** — 如果您的 `memory/AGENTS.md` 或
片段包含它，agentsync 错误并要求您删除它（所以它不能
与横幅碰撞）。反之亦然：捕获仅剥离agentsync的
自己的横幅，因此您保留的任何其他内容永远不会被删除。

### 市场和插件——扇出回报

这就是agentsync 赖以生存的地方。添加市场，安装插件一次，
每个启用的代理都会获得它理解的组件：



```bash
agentsync marketplace add github:anthropics/claude-plugins-official
agentsync plugin add atlassian@anthropic

agentsync plugin outdated  # fetch from the network (refresh cache, show bumps)
agentsync plugin upgrade --all   # re-pin every pending bump and re-apply
```



插件是一包组件（MCP 服务器、技能、子代理、命令、挂钩、
LSP 服务器）。每个代理都独立翻译 - 完全翻译、有损翻译或
跳过 - 报告会准确告诉您哪些内容：



```
▸ atlassian@anthropic
  → claude    ✓ full        1 mcp · 5 commands · 3 subagents · 1 lsp
  → codex     ◐ partial     1 mcp · 5 commands · 3 subagents · 1 lsp  (3 reduced · 1 dropped)
      → codex couldn't fully translate — reduced = rendered without some fields; dropped = not emitted:
        • subagent atlassian-ai-architect   reduced  Codex agents are TOML with no per-agent tools allowlist; dropped tools, color
        • subagent atlassian-deploy-expert  reduced  Codex agents are TOML with no per-agent tools allowlist; dropped tools, color
        • subagent atlassian-perf-optimizer reduced  Codex agents are TOML with no per-agent tools allowlist; dropped tools, color
        • lsp atlassian-lsp                 dropped  Codex has no LSP configuration concept
```



每行的计数尾部列出插件为该代理托管的每个组件类型
— MCP 服务器、命令、技能、子代理、挂钩和 LSP 服务器（仅限
显示非零种类）——因此库存是完全描述性的，而不仅仅是 `mcp`
+ `commands`。计数描述了插件*托管*的内容；覆盖字形和
尾随注释描述了代理可以用它“做什么”。

该尾随注释按类型分开，因此它永远不会读作“N 个完整组件
丢弃”：仍呈现 **减少** 部分，只是没有代理的某些字段
没有家（这里每个子代理都作为 Codex TOML 登陆，只有它的 Claude-only
`tools`/`color` 前面的内容已删除）； **丢弃的**部分没有本机目标
所有并且没有被发出（LSP 服务器 - Codex 没有 LSP 概念）。仅 LSP
Codex 上的插件读取 `✗ none  1 lsp  (1 dropped)`，告诉你们那里有什么
但没有一个落地。

`◐ partial` 行永远不会是死胡同：代理无法完全理解的每个部分
翻译在框架标题下方逐项列出，每个标记为 `reduced` 或
`dropped` 并说明原因，以便您可以准确了解 `apply` 会产生什么损失。
`--json` 携带每种计数 (`mcp`, `commands`, `skills`, `subagents`,
每行上的 `hooks`、`lsp`）和 `skipDetails` 数组（每个条目 `{component, name,
reason, kind}`）。 `kind` 是 `"reduced"` 或 `"dropped"` — 显式
机器表面进行分割，因此消费者永远不会从机器表面重新导出它
组件字符串。 `component` 是普通组件类型 (`subagent`, `command`,
`lsp`，...）；它不带有 `-frontmatter` 后缀。

检查任何插件的覆盖范围而不应用：



```bash
agentsync plugin explain atlassian@anthropic                   # one plugin
agentsync plugin explain atlassian@anthropic superpowers@obra  # space-separated
agentsync plugin explain --all                                 # every installed plugin
agentsync plugin explain atlassian@anthropic --json            # machine-readable
```



(`agentsync plugin list` 打印已安装的 ids。顶级 `explain` 名称
回答了另一个问题 - 请参阅[此文件来自哪里？](#where-did-this-file-come-from)。）

#### 插件组件由其插件命名

插件的**子代理、技能和斜杠命令**在下面呈现
`<plugin>-<name>`，这就是上面的报告显示为 `atlassian-ai-architect` 的原因
而不是 `ai-architect`：

|插件 |船舶 |降落于 |您调用 |
|---|---|---|---|
| `atlassian` | `agents/ai-architect.md` | `~/.claude/agents/atlassian-ai-architect.md` | `@agent-atlassian-ai-architect` |
| `atlassian` | `commands/jira.md` | `~/.claude/commands/atlassian-jira.md` | `/atlassian-jira` |

每个代理从一个平面目录读取其组件，因此没有这两个
发布同名组件的插件会在一个路径写入两个文件。那
不是假设的：`feature-dev` 和 `pr-review-toolkit` — 均为官方股票
插件 — 每艘船 `agents/code-reviewer.md`，以及在命名空间之前
`agentsync apply` 失败并且没有出路，因为这两个文件都不是您可以重命名的。
Claude Code 本身也达到了同样的目的，将插件的代理寻址为
`plugin:agent`； agentsync 使用连字符，因为冒号在
子代理 `name`（或 Windows 上的文件名）。

**您自己编写的组件永远不会重命名。**您自己编写的任何组件
`~/.agentsync/subagents/`、`skills/` 或 `commands/` 保留您指定的名称，
即使已安装的插件附带了同名的插件。在极少数情况下
插件的*派生*名称落在您的其中一个上（`feature-dev` 运输
`code-reviewer` 与您自己的 `feature-dev-code-reviewer`），agentsync 是这么说的
并列出双方的名字，而不是选出胜利者。 MCP 和 LSP 服务器也保留它们的 id：两个来源声称一个
服务器 ID 被拒绝而不是分开重命名，因为重新指向受信任的服务器
服务器端点是劫持，而不是命名冲突。

### 插件扇出到哪个代理

`plugins/<id>.toml` 中的两个键决定插件组件的渲染位置。他们
在渲染腰部强制执行一次，因此每个代理和每个组件类型
同样地服从它们：



```toml
[plugin]
id            = "feature-dev@claude-plugins-official"
agents        = ["*"]        # your fan-out choice: which agents may receive it
native_agents = ["claude"]   # agents that install it THEMSELVES — do not project there
```



`agents` 是您控制的白名单：`["*"]`（默认值，与
意思是省略密钥）将插件发送到每个启用的代理或名称
代理明确地缩小范围。

`native_agents` 是关于世界的陈述而不是偏好。当
代理自己的插件管理器已经安装了一个插件 - 您运行了 `/plugin install`
在 Claude 代码中 - agentsync 不得将该插件的组件也投影到
该特工的独立路径，或者您获得每种技能、子特工和
命令，每个钩子都会触发两次。 agentsync永远不会禁用内部插件
另一个工具的插件管理器（参见 [architecture.md § PluginIngester
（只读）]（architecture.md#pluginingester-read-only）），所以唯一的方法
避免重复就是不要在那里投影。

**`import` 询问每个插件。** 从代理中导入插件意味着，通过
定义，代理已安装它，因此 `agentsync import claude:plugin`
停止并提供延期：



```
ℹ INFO   claude already installs the plugin "feature-dev".
         agentsync can leave those components to claude, or project its own copy alongside them.
  Let claude keep serving this plugin? [Y]es / [n]o, project it there too:
```



接受记录 `native_agents = ["claude"]`。克劳德继续提供插件服务
本身；每个其他启用的代理仍然得到扇出 - 这就是整体
导入它的点。

拒绝是一个真正的选择，agentsync 说明了它的成本：



```
⚠ WARN   this will DUPLICATE the plugin "feature-dev"'s content in your claude
         harness — every skill, subagent and command it ships will appear twice,
         and its hooks will fire twice. Disable or uninstall "feature-dev" in
         claude now that agentsync is managing it.
```



**仅**如果您随后在 Claude 内关闭该插件，这才是一个有效的设置
代码。拒绝不会写入任何密钥，因此稍后的导入会再次询问 - 并且仅询问是否
副本仍然存在。一旦您本机禁用该插件，问题
不再被询问。要使其静音，同时保持插件在两者中启用，请编写
`native_agents = []` 手动：明确的空列表意味着“不服从任何人”
并被保留。

只会询问您答案何时生效 - 不会询问其插件
`native_agents` 您已经设置，因为重新导入会保留它。与
`--no-input`，或者当 stdin 不是终端时，延迟会被记录而无需
询问并且信息行这样说：脚本化导入不得默默地产生两个
的一切。

要将插件完全移交给agentsync，请在agent中卸载它
（克劳德代码中的`/plugin uninstall`）并从`native_agents`中删除该代理。
接下来的应用将在那里投影组件。与代理本人无关
无论哪种方式都会影响配置 - 延迟完全存在于您的规范中
源代码，这使得 `apply` 可以单独从 dotfiles 存储库中重现
而不是依赖于机器碰巧安装的任何内容。

两个密钥都可以在重新安装和重新导入时保持不变（问题#140），因此
缩小的允许名单或采用的插件永远不会默默重置。

因为 apply 的计划从不读取目的地，所以 agentsync 无法注意到
声明后本地安装的插件。 `status` 和 `doctor` 确实阅读
它，并在代理中安装插件*并*投射到那里时发出警告。下
`--agents`，该检查遵循您选择的代理 - 因此缩小了运行名称
具有本机插件管理器的已启用代理，它没有检查，而是
而不是让沉默被视为干净的健康证明。

关于 `--lossless`（`plugin upgrade <id> --lossless` 和
`plugin upgrade --all --lossless`； `plugin outdated` 不采用该标志）：
决定升级是否会在转换过程中丢失某些内容的检查
渲染每个*启用的*代理并且**不**尊重插件的`agents` /
`native_agents`。因此，它可以因代理的损失而拒绝升级
这个插件从未被预测到。它错误地走向衰落，而不是走向
升级；运行 `agentsync plugin explain <id>` 查看实际有哪些代理
接收插件，如果受影响的插件不存在，则在没有 `--lossless` 的情况下重新运行
其中。检查根本无法评估的碰撞也被排除在外，但是
单独报道——拒绝并不是针对目标和放弃旗帜
不是它的答案。

（指定了每个组件的 `[plugin.overrides.<agent>]` 表，但 **不是
有线连接 v1** — 投影机不参考它；使用上面的按键。）

### 秘密

切勿将凭据放入配置文件中。参考一下：



```toml
# in mcp/github.toml
[server.env]
GITHUB_TOKEN = "${secret:github.token}"
```



首先，创建一个年龄密钥对。保险库对**收件人**进行了加密
（公钥——安全提交）；解密需要**身份**（私钥-
每台机器）。 agentsync 嵌入年龄，但生成密钥使用 `age-keygen`
CLI (`brew install age`, `apt install age`, …):



```bash
mkdir -p ~/.config/agentsync
age-keygen -o ~/.config/agentsync/age.key   # prints "Public key: age1…" to stderr
chmod 600 ~/.config/agentsync/age.key        # agentsync refuses a group/other-readable identity
```



然后将 `agentsync.toml` 指向它 — `recipient` 是 `age1…` 公钥
`age-keygen` 打印（agentsync 加密到单个 X25519 收件人，因此使用
Age-keygen 密钥，而不是 SSH 密钥）：



```toml
[secrets]
backend       = "age"
recipient     = "age1…"
identity_file = "${env:HOME}/.config/agentsync/age.key"
```



将值存储在年龄加密的保管库中 - 三种方式：



```bash
agentsync secret set github.token --stdin    # from stdin (best for scripts / 1Password CLI)
agentsync secret set github.token            # interactive prompt, echo off
agentsync secret edit                        # open the whole vault in $EDITOR
agentsync secret get github.token            # read one back (to verify)
```



`secret set` 默认拒绝**空或仅空白**值（a
粗手指粘贴或空的 `pbpaste`/`1password` 管道否则会存储
在应用时解析为 `""` 的无声秘密）；通过 `--allow-empty`
故意存储一个空值。

`${secret:…}` 在应用时解析并写入本机配置； `${env:…}`
从环境中提取。解析的值**永远**不会被捕获回
你的源代码 - `agentsync diff` 甚至对其进行了编辑，这样管道差异就不会泄漏它。

> ### ⚠ 备份您的年龄密钥
> 秘密被加密到年龄**接收者**（公钥 - 可以安全提交）。
> 解密需要**身份**文件（私钥），该文件是每台机器的。
> **agentsync 不会为您备份。** 丢失它，您将无法访问所有内容
> 加密的秘密。将其存放在 1Password 安全注释或您的机器设置存储库中。

### 项目本地配置

存储库可以携带自己的**项目源代码树** - 位于以下位置的 `.agentsync/` 目录
它的根目录，与您的用户 `~/.agentsync/` 具有相同的布局。承诺分享
项目与合作者的代理配置。用以下方式搭建脚手架：



```bash
cd ~/code/myrepo
agentsync init --scope project        # creates ./.agentsync/
# or target another path explicitly (implies project scope):
agentsync init --project ~/code/myrepo
```



写为 `<root>/.agentsync/agentsync.toml` 加 `mcp/`, `lsp/`, `skills/`,
`subagents/`、`commands/`、`hooks/`、`memory/`、`plugins/` 和 `secrets/` —
与用户树相同的文件，减去 `.state/` （在下集中应用记录状态
`~/.agentsync/.state/`，由项目根键入）。在中编写项目的配置
这棵树：



```toml
# myrepo/.agentsync/agentsync.toml
[agents]
claude = { enabled = true }       # the agents THIS project renders to —
                                  # required; user-scope agents are never
                                  # inherited, so every collaborator gets the
                                  # same render
```



使用项目范围代理命令声明代理，而不是手动编辑：



```bash
agentsync agent add claude --scope project     # or --project <path>
agentsync agent list --scope project
agentsync agent disable claude --scope project
```





```toml
# myrepo/.agentsync/mcp/company-api.toml — same format as a user-scope mcp file
[server]
type    = "stdio"
command = "npx"
args    = ["-y", "@company/mcp"]
```





```toml
# myrepo/.agentsync/plugins/screenshot.toml — turn off a user-level plugin here
[plugin]
disabled = true
```



直接在 `<root>/.agentsync/memory/AGENTS.md` 中创作项目内存（撰写
它来自 `fragments/`，就像用户树一样）。

项目树**覆盖**到您的用户规范上：项目条目
替换具有相同 ID/名称的用户条目，附加新条目，并且
项目存储器附加在用户存储器之后。该项目的 `[agents]` 表是
**权威** - 项目范围仅呈现给项目本身的代理
声明，绝不是您的用户范围代理。一个没有声明任何内容的项目是一个困难的项目
每个范围感知渲染路径上的错误 - `apply`/`status`/`diff`/`reconcile`/
`plugin upgrade --all`/`check`（运行 `agentsync agent add <name> --scope project` 到
修复）； `import --scope project` 在声明任何代理之前仍然有效，因此
您可以首先从本机配置引导树。

在项目范围内应用（明确选择加入），覆盖层将合并到您的
用户配置：



```bash
cd ~/code/myrepo
agentsync apply --scope project   # walks up from cwd to the .agentsync/ tree
ls .mcp.json                      # project-scope MCP servers landed (repo root)
```



命令默认为**用户**范围。项目范围永远不会自动应用：通过
`--scope project`（从cwd向上走找到树）或`--project <path>`
（`--scope user` 与 `--project` 一起是一个错误）。如果您运行命令
项目树*内部*没有范围，agentsync **提示**您选择
项目与用户；在非交互式 shell 中 — 或者使用全局 `--no-input`
flag——它是错误而不是猜测。 `--scope project` 没有找到树（并且
`--project` 在没有 `.agentsync/` 树的路径上）是一个指向您的硬错误
在 `agentsync init --scope project` - 它永远不会默默地回退到用户范围。

> **从旧的单文件标记升级？** 已退役的 `.agentsync.toml`
> 不再读取存储库根目录处的标记 — agentsync 错误并告诉您运行
> `agentsync init --scope project` 并将设置移至 `.agentsync/`
> 树。

---

## 从网络更新

`plugin outdated` 是每日循环的轮询动词：它重新获取每个
将市场注册到本地缓存并重新计算版本引脚，无需
触摸任何代理配置。 `apply` 然后从该缓存渲染，所以它总是
快速、离线且可重复。



```bash
agentsync plugin outdated                      # refresh cache + show pending bumps
agentsync plugin upgrade --all                 # re-pin every pending bump, then re-apply
agentsync plugin upgrade --all --lossless      # same, skipping bumps that would lose translation
agentsync plugin upgrade atlassian             # re-fetch one plugin, then re-apply
```



两种 `upgrade` 表格均以重新申请结束，因此升级将一次性到达您的代理处
命令而不是让它们陈旧直到下一个 `apply`。

`plugin outdated` 不是纯粹的读取，尽管 `npm outdated` 先验：它使用
网络并写入状态（每个市场的获取时间戳和头部
沙）。它也不是*唯一的*网络命令 - `plugin add`、`plugin
upgrade`、`marketplace add`、`import <agent>:plugin` 和 `init <git-url>` 全部
获取。它只是每日循环运行的一个。

> **删除：**顶级 `update` 命令已消失，没有别名。移动你的
> cron 行结束：裸 `update` → `plugin outdated`、`--apply` → `plugin upgrade
> --all`、`--apply --auto-safe` → `plugin upgrade --all --lossless`。

想要每晚焕然一新吗？ agentsync 不提供守护进程 — 线
`agentsync plugin upgrade --all --lossless` 进入你自己的 cron/launchd/
systemd / 任务计划程序。

---

## 这个文件从哪里来？

`diff` 回答“发生了什么变化”。 `explain <path>` 回答了另一个问题 —
像 `~/.claude/settings.json` 这样的合并文件会使：



```bash
agentsync explain ~/.claude/agents/reviewer.md          # a whole file
agentsync explain ~/.claude.json#/mcpServers/github     # one merged key
agentsync explain ~/.claude.json --pointer /mcpServers/github   # same, unambiguous
agentsync explain ~/.claude.json --json                 # machine-readable
```



对于每个项目，它报告**记录来源**（`mcp/github.toml`，
`subagents/reviewer.md`、`memory/AGENTS.md` *加上其片段*)，**插件
origin** 如果组件来自投影，则 **适配器变换**
（途中减少或丢弃的内容），**所有权** - `managed`，
`untracked`（已渲染，尚未应用），或 `foreign`（您的；由
key merge) — 分类器自己的词汇表中的**漂移类**，并且
`${secret:…}`/`${env:…}` 引用已在那里解析。

值得了解的三件事：

- **它仅打印元数据。**没有键值，没有漂移片段，没有文件
  内容。这就是让它用**上锁的秘密金库**来回答的原因，其中
  `diff` 必须关闭失败，而不是打印它无法编辑的明文。想要
  内容？那是`agentsync diff <path>`。
- **它重新渲染实时**，如 `diff` — 因此它对于未应用的内容是诚实的
  源树而不是叙述最后的应用。
- **一条路径可以有多个所有者。** 代理共享目的地（宽度
  层共享 `~/.agents/skills/`），因此输出按拥有代理进行分组，并且
  两个代理将*不同的*内容渲染到一条路径被报告为其自己的内容
  回答。

未托管或有拼写错误的路径的报告与干净的路径不同（带有
建议最近的管理路径），与 `diff [<path>]` 完全相同。

对于每个插件的翻译覆盖范围——“每个代理可以用这个插件做什么”
— 使用 [`agentsync plugin explain`](#multi-agent-fan-out) 代替。

---

## 多代理扇出

并非每个代理都支持每个组件，agentsync 从不假装
否则。每个组件都标记为 **✓ 原生**、**◐ 投影**（有损，但
报告），或每个代理 **✗ 跳过**（没有诚实的翻译）。

Claude、OpenCode、Codex、Cursor、Gemini CLI、Continue、Windsurf、Roo Code 和 Cline 都是真正的适配器。

|组件|克劳德|开放代码 |法典|光标|双子座|继续 |风帆冲浪 |袋鼠 |克莱恩 |
|---|:--:|:--:|:--:|:--:|:--:|:--:|:--:|:--:|:--:|
| MCP服务器| ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
|内存| ✓ | ✓ | ✓ | ◐ | ✓ | ✓ | ✓ | ✓ | ◐ |
|技能| ✓ | ✓ | ✓ | ✓ | ✗ | ✗ | ✗ | ✗ | ✗ |
|子代理 | ✓ | ◐ | ◐ | ◐ | ◐ | ✗ | ✗ | ✗ | ✗ |
|斜线命令 | ✓ | ◐ | ◐ | ◐ | ◐ | ◐ | ◐ | ◐ | ◐ |
|钩| ◐ | ✗ | ◐ | ◐ | ◐ | ✗ | ✗ | ✗ | ✗ |
| LSP服务器| ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ |

（一些适配器是范围不对称的：Windsurf 和 Cline 的 MCP 仅限全局，并在用户范围内渲染 - Windsurf 内存 + 命令在两个范围内渲染，Cline 在项目范围内渲染；Roo 仅在项目范围内渲染 MCP - VS Code 代理将全局 MCP 保留在应用程序存储中。请参阅[功能矩阵](capability-matrix.md)。）

除了这 9 个深度适配器之外，还有 22 个代理（amp、goose、
qwen、warp、zed、kiro、junie、factory、copilot、crush 等）通过其中之一进行支持
数据驱动的通用适配器 - 所有人的内存，代理读取 JSON 的 MCP
代理本机扫描的服务器映射和代理技能（`SKILL.md` 目录）
技能目录。运行 `agentsync agent list --all` 查看全部；看到
每个代理的[能力矩阵 → 广度层](capability-matrix.md#breadth-tier)
覆盖范围。

完整的细节、本机路径以及每个◐/✗背后的推理都在
[能力矩阵](capability-matrix.md)。

---

## 跨机同步

agentsync 故意是单机的。携带 `~/.agentsync/` 穿越
机器，使用 [chezmoi](https://www.chezmoi.io/) （或任何点文件管理器）：



```bash
chezmoi add ~/.agentsync
```

加密的机密文件可以安全同步；年龄身份（私钥）是
不是——通过您现有的秘密共享流程分发它。

---

## 命令参考

贝塔面。 `agentsync <command> --help` 始终具有权威性。

| Command | Purpose | Key flags / args |
|---|---|---|
| `init [<git-url>]` | Create `~/.agentsync/` (user scope); optionally clone a bootstrap repo. `--scope project` scaffolds a project tree at `<cwd>/.agentsync/` instead; `--project <path>` targets `<path>/.agentsync/` (implies project scope). A git-URL clone is user-scope only. | `--scope --project` |
| `doctor` | Diagnose setup: PATH, home/state writability, config schema, secrets backend, destination-git-backup mode + per-dir repo status; flags natively-installed plugins missing from source. | |
| `check` | Validate the **config**: schema lint plus every `${secret:}`/`${env:}` reference resolved. `--scope project`/`--project <path>` lints the project tree against the inherited user secrets backend. Its sibling is `doctor`, which validates the **machine**. (Renamed from `verify` — see [Upgrading](#command-reference).) | `--scope --project` |
| `governance scan|init|check|capabilities` | Read or initialize the project Baseline/Profile contract used before Agent edits. `scan` is read-only; `init` writes only governance source files; `check` reports missing Profile/docs or capability loss; `capabilities` prints the explicit adapter matrix. | `--scope --project --profile --agents --dry-run --force --json` |
| `agent add\|remove\|list\|enable\|disable <name>` | Manage the agent registry — the user's, or with `--scope project`/`--project <path>` the project tree's own `[agents]` declaration (which project scope renders from; never inherited). At project scope `disable --purge` touches only that project's rendered files. | `disable --purge --scope --project` |
| `skill\|subagent\|command\|hook\|lsp list` | List that component in the canonical source. Read-only by design: unlike an MCP server, none of these is flag-authorable — a skill is a *directory*, a subagent is a markdown file — so you author them on disk or capture them with `import`. | `--scope --project` |
| `migrate subagents` | One-shot move of the retired canonical `agents/` directory to `subagents/`, rewriting that tree's recorded `source_id` values. Run once per tree (`--scope project` / `--project <path>` for a project tree). Refuses, listing the names, if a file exists under both directories. | `--scope --project` |
| `mcp add\|remove\|list\|enable\|disable <name>` | Manage MCP servers. `enable`/`disable` flip the server's `enabled` bit — keeping the definition but stopping the render (`remove` deletes it). `--header "Name: Value"` (repeatable, http/sse only) sets request headers — the usual remote-auth secret site, e.g. `--header "Authorization: Bearer ${secret:TOKEN}"`. | `--type --command --args --url --env --agents --header` |
| `marketplace add\|remove\|list <url-or-name>` | Manage marketplaces. | |
| `plugin add\|upgrade\|enable\|disable\|remove <id[@marketplace]>` / `list` / `outdated` / `explain` | Manage plugins (the lifecycle subcommands all accept the same `id[@marketplace]` ref `add` accepts; the bare id also works, and a qualifier naming a different marketplace than the one the plugin was installed from is refused). `outdated` **(network)** polls the marketplaces and reports pending bumps — it also writes each marketplace's fetch timestamp + head SHA to state. `upgrade` **(network)** re-fetches one plugin, or with `--all` every plugin with a pending bump, and **re-applies** in both cases; `--lossless` skips an upgrade that would introduce a new translation loss, reporting it. `explain` shows per-agent translation coverage. | `outdated` · `upgrade [<id>] --all --lossless --scope --project` · `explain [<id>...] --all --json` |
| `secret set\|get\|list\|remove <key>` / `secret edit` | Manage age-encrypted secrets (`list` prints KEYS only; `edit` opens the whole vault, no `<key>`; `set` refuses an empty value unless `--allow-empty`). | `set --stdin` |
| `apply` | Render source → write agent configs (offline). Git-versions each user-scope destination dir into a local-only repo (opt-out) so a bad apply is revertible. A delete-only run (a component removed from source) reports `removed: N key(s), M file(s)` — key-removals and file-deletes counted distinctly — and a mixed run `applied: X ops, removed: …`, rather than mislabeling itself `up to date`/`applied: 0 ops`; `--dry-run` previews the same removal counts. | `--agents --dry-run --scope --project --no-git-backup` |
| `revert <agent>` | Roll a destination dir back to a prior apply checkpoint (append-only). Default undoes the most recent apply; prints an out-of-sync notice. `--to` must name one of the dir's own checkpoints (the current one or an ancestor) — anything else is refused. A dir under which a foreign git repo has appeared (or that isn't an agentsync-managed backup) is an **error** when you name the agent, and a **skip with a warning** under `--all` — strictness follows the invocation; there is no `--strict` flag. | `--agents --to --all --dry-run` |
| `status` | Summarize drift/pending across agents; notes natively-installed plugins not yet in source. Skill directories collapse to one summary row by default (`--verbose` expands them). The formatted report shows `converged` items as `clean` (both mean apply has nothing to do); `--json` keeps the two distinct. `--legend` prints a standalone glossary of all nine drift classification statuses and exits (rejects combination with `--json`/`--exit-code`/`--agents` rather than silently ignoring them). `--exit-code` makes it a CI gate: exit `2` when any drift is detected, `0` when clean. | `--agents --verbose --legend --scope --project --json --exit-code` |
| `diff [<path>]` | Show pending/drift changes; secrets redacted. `<path>` is a filesystem path; an unmanaged/typo'd path is reported distinctly from a clean one. `--agents` narrows to an agent allowlist (like `status`); `--exit-code` exits `2` when any hunk exists, `0` when clean. | `--agents --scope --project --json --exit-code` |
| `reconcile` | Interactively merge drift back into source. | `--agents --auto-writeback --auto-override --auto-safe --scope --project` |
| `import <agent>[:<component>[:<name>]]` | Capture native config into source; drop parts to import a whole component or the agent's full config. Includes `plugin` (Claude), which re-fetches installed plugins + marketplaces **(network)**. `--scope project` reads the agent's *native project-scope* config (e.g. `<root>/.claude/`) and captures it into the project tree `<root>/.agentsync/`, seeding central state with the project scope + root. Plugin import is user-scope only. | `--dry-run --scope --project` |
| `explain <path>[#<pointer>]` | Show what produced a destination file (or one merged key): source of record, plugin origin, adapter transform, ownership, drift class, and any `${secret:…}` references. **Metadata only** — never destination content, so it answers even when the secrets vault is locked (where `diff` must fail closed). `--pointer` is the unambiguous alternative to an in-argument `#`. Project scope is inferred when the path lies inside a project tree. | `--pointer --scope --project --json` |
| `version` | Print version information (alias for `--version`). | |

全局：`--scope user|project` 和 `--project <path>` 是 **根标志**，已声明
一次并被在作用域源树上运行的每个命令接受（`init`，
`agent`、`mcp`、`apply`、`check`、`status`、`diff`、`reconcile`、`import`、
`explain`、`migrate`、`plugin upgrade` 和组件 `list`）。一个
命令*不能*
尊重他们 - `doctor`、`revert`、`version` 和 `plugin` / `marketplace` /
`secret` 组，所有这些组都作用于每台机器的状态 - **用
理性**而不是接受和忽视它们。

`-v/--verbose` 用于对任何命令进行详细记录（在 `status` 中，它还
将每个折叠的技能目录扩展回每个捆绑文件的一行）。
`--color=auto|always|never` 控制输出是否使用 ANSI 颜色样式
粗体（默认 `auto` — TTY 时打开，通过管道/重定向时关闭；荣誉
`NO_COLOR`）。颜色是“第二个”信号，而不是唯一的信号：每个诊断
还带有一个级别标签 - `✗ ERROR`、`⚠ WARN`、`ℹ INFO`（以及保留的
`• DEBUG`，今天没有任何发射） - 上
**stderr**，因此严重性可以通过管道传输到文件或 CI 日志中。命令
*结果*（`--json`有效负载，`status`表，`diff`，`list`）转到
没有标签的标准输出，成功结果以表情符号开头 -
`🎉 applied: 12 ops`、`✅ added agent: claude`、`🧹 removed mcp server: github`、
`📥 imported 4 items from claude`、`🔙 reverted…`、`✨ …initialized`。这种分裂就是为什么
警告永远不会出现在 `status --json` 的中间。 `--agents <list>` 是表示哪个代理命令的**一种**方式
作用于： `apply`、`status`、`diff`、`reconcile` 和 `revert` 都接受它，
相同的解析和相同的 `*` = 所有启用的约定（空或未知
值被所有五个人相同地拒绝）。在 `revert` 上，它是
位置形式，因此位置代理 `--agents` 和 `--all` 相互保持
独家。每个 `list` 接受 `ls`，每个 `remove` 接受 `rm`。 `status
--json` 和 `diff [<path>] --json` 发出
结构化报告而不是格式化报告，适用于 CI 门和
仪表板（`status --json` 永远不会折叠 - 它包含每个跟踪的文件；
`diff --json` 屏蔽了与格式化 diff 相同的解析秘密）。对于一个
在漂移上应该**失败构建**的门，添加 `--exit-code`: `status
--exit-code` / `diff --exit-code` 当存在漂移/帅哥时退出 `2` 和 `0`
clean (exit `2` 与通用错误退出 `1` 不同，并且不打印额外的内容
错误行）。交互式提示（例如范围菜单）始终转到 **stderr**，
因此从标准输出管道传输的 `--json` 有效负载永远不会损坏。

格式化的 `status` 报告将 `converged` 项目显示为 `clean` — 两者分别是
内部漂移分类器中不同（收敛意味着源*和*
目的地独立变化，但与干净相比，达到相同的值
两者都没有改变），但两者都意味着 apply 无关，所以
仪表板将它们折叠成一个单词和一个计数。 `status --json` 保留
真正的分类。 `status --legend` 打印所有内容的独立词汇表
九个分类状态（包括 `clean`/`converged` 拼写出来
分别）并退出而不运行漂移扫描；其摘要包含的运行
任何要解释的内容都以指向它的一行提示结束，并被抑制
每当没有任何跟踪来解释（没有启用代理，或启用
尚未呈现任何内容的代理）。

---

## 故障排除和环境覆盖

[README](../README.md#troubleshooting) 包含完整的故障排除列表
以及完整的环境变量表。您最常接触到的：

|环境变量 |目的|
|---|---|
| `AGENTSYNC_HOME` |覆盖 `~/.agentsync/` 位置。 |
| `AGENTSYNC_ALLOW_SYMLINK_DEST=1` |通过符号链接目标进行写入（例如 chezmoi 管理的文件）。 |
| `AGENTSYNC_ALLOW_INSECURE_URLS=1` |接受 `http://`/`git://` 插件/市场来源。 |
| `AGENTSYNC_ALLOW_OFFLINE_VERIFY=1` |让 `check` 仅验证参考 *shape*，跳过解析（没有年龄密钥的 CI）。 |
| `AGENTSYNC_NO_UPGRADE_NOTICE=1` |切勿显示升级后首次运行的一次性通知。 |

快速点击：

- **`${secret:foo}` 未解决？** `agentsync secret get foo` 确认
  密钥存在于解密的保管库中。
- **`plugin outdated` 无法获取市场？** 使用以下命令对 URL 进行完整性检查
  `git ls-remote`。
- **首先应用备份一堆文件？** 预计在已填充的计算机上 —
  他们在 `.state/backups/<ts>/` 中，没有丢失任何内容。

---

## 下一步去哪里

- **[概念和术语](concepts.md)** — 深入的心智模型。
- **[架构](architecture.md)** — 管道和安全不变量如何工作。
- **[能力矩阵](capability-matrix.md)** — 正是每个代理支持的内容。
- **[组件映射](components.md)** — 代码库，逐个包。
- **[SECURITY.md](../SECURITY.md)** — 威胁模型和报告。

在最初的 100 分钟内发现了困难？这正是测试版
我们想要的反馈 - [提出问题](https://github.com/spxrogers/agentsync/issues)。