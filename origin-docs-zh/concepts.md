# 概念和术语表

agentsync 借鉴了 [chezmoi](https://www.chezmoi.io/) 的思维模型：你
保留一个真理的**规范来源**，你**应用**它来产生真实的
文件代理读取，当有人从你手下编辑这些文件时，
agentsync 检测**偏差**并帮助您**协调**它。

阅读本页一次以及其余文档 — [架构](architecture.md)，
[用户指南](user-guide.md) 和 `agentsync --help` — 将卡入到位。

---

## 三态模型

agentsync 所做的一切都是对同一对象的三种状态进行比较
逻辑事物（MCP 服务器、内存文件、插件的斜杠命令）：

|状态|它是什么 |它住在哪里|
|---|---|---|
| **来源** | *你*犯了什么——意图。 | `~/.agentsync/`（您拥有的 git 存储库）|
| **目标** |对于给定代理，源*呈现给*什么，在应用时在内存中新鲜计算。 |磁盘上无处可寻 — 它是暂时的 |
| **目的地** |现在每个代理的本机配置中*实际上在磁盘上*是什么。 | `~/.claude.json`、`~/.config/opencode/`、... |



```
   Source                Target                 Destination
  ~/.agentsync/   render   (in-memory)   write    ~/.claude/…
  hand-edited    ───────▶  per-agent    ───────▶  ~/.config/opencode/…
  TOML + .md               FileOps                native config files
       ▲                                                │
       │                      capture                   │
       └────────────────────────────────────────────────┘
              (write native edits back into source)
```



该模型的天才之处在于**漂移只是哈希比较**：

- **漂移** — 自 agentsync 上次写入以来目的地发生了变化
  (`hash(destination) ≠ hash(last-applied)`)。有些东西编辑了本机文件。
- **来源更改** — 您编辑了规范来源
  (`hash(target) ≠ hash(last-applied)`)。正常的待定更改。
- **冲突** — *两者都*发生了。 `apply` 无论如何都会覆盖目的地
  （没有备份，没有提示——见下文）； `reconcile` 是您首先捕获它的方式。

---

## 核心术语

### 规范来源 (`~/.agentsync/`)
您拥有并（可选）提交到 git 的单个目录。它容纳小，
可手动编辑的 TOML 文件（每个文件一个 MCP 服务器，每个文件一个插件）以及
记忆和技能的降价。 **技能**遵循[特工技能](https://agentskills.io)
spec — 它是一个*目录* `skills/<name>/`，其唯一必需的成员是
`SKILL.md` 以及任何捆绑的 `scripts/`、`references/`、`assets/` 或嵌套文件
逐字携带（包括二进制），而不仅仅是 `SKILL.md`。覆盖
带有 `AGENTSYNC_HOME` 的规范位置。
**没有隐藏的内部表示** - 解析这些的 Go 结构体
TOML 文件*是*规范模型。

### 适配器
每个代理的翻译。每个适配器都知道如何**渲染**规范模型
转换为一个代理的本机配置格式以及如何**摄取**该本机配置
回到规范模型。 Claude、OpenCode、Codex 和 Cursor 都是真实的
适配器（请参阅[能力矩阵](capability-matrix.md)）。添加代理
意味着添加适配器——规范模式永远不会改变。

### 应用/渲染
`agentsync apply` 运行 **渲染** 管道：加载源 → 解析
秘密 → 要求每个启用的适配器将模型投影到一组文件中
操作 → 以原子方式写入它们 → 在状态中记录哈希值。适用的是
**仅限本地且离线** - 它永远不会连接网络。渲染内存文件
还可以在前面添加一个简短的**托管横幅**（命名文件，指向编辑
返回 `.agentsync/memory/AGENTS.md` + `agentsync apply`)；它只生活在
渲染文件 - 在摄取/捕获时剥离，从未写入规范
源 - 默认情况下处于打开状态（`[memory] banner = false` 选择退出）。

### 捕获/摄取/回写
反方向。 **摄取** 将代理的本机配置读回到
规范模型； **capture** 将其保留回 `~/.agentsync/`。就是这样
`agentsync import`（将本机编辑拉入源）并协调
`[w]rite-back` 行动工作。所有回写渠道均通过一处
(`internal/capture`) 因此秘密会被重新引用并且永远不会以明文形式泄漏。

### 漂移和三向分类器
对于每个托管项目，agentsync 保存三个哈希值 — source (`H_src`)、
最后应用的（`H_applied`，来自州）和目的地（`H_dest`） - 和
将该项目准确地分为九种情况之一：

|班级 |意义| `apply` 的作用 |
|---|---|---|
| **干净** |三人都同意|什么都没有|
| **待定** |你改变了来源|编写新的源代码|
| **漂移** |目的地已编辑 |覆盖它——没有备份，因为agentsync已经拥有它； `reconcile` 是保留编辑的方式 |
| **收敛** | source 和 dest 更改为*相同*值 |静默刷新状态 |
| **冲突** |源和目标更改为*不同的*值 |覆盖它——没有备份，同样的原因； `reconcile` 是合并编辑的方式 |
| **新** |全新物品，磁盘上没有任何内容|创建 |
| **外来碰撞** |预先存在的文件agentsync 未写入|备份它，然后写|
| **孤儿** |从源中删除，仍在磁盘上 |删除，如果 `apply` 仍然回收那种文件 |
| **孤儿漂流** |从源中删除，但目标也被编辑 |备份它，然后删除，如果 `apply` 仍然回收那种文件 |

`apply` 从不阻止或提示其中任何一个 - 它总是完成运行。
只有 **foreign-collision** 和 **orphan-drifted** 在之前获得每个文件的备份
写入/删除：这是目的地保存内容的两种情况
agentsync 尚未拥有状态。 **漂移**和**冲突**是
根据定义已经属于国有，因此编写者的每个文件备份路径会跳过
它们并直接覆盖 - 手动编辑完全丢失，没有
保留每个文件的副本。 **孤儿**/**孤儿漂流**用“如果
`apply` 仍然回收那种文件": `apply` 只回收一个
当其源停止渲染它时的技能/子代理/命令目的地 - 对于
对于所有其他类型（内存、MCP/hook/LSP 条目），`status` 仍然报告
类，但下一个 `apply` 根本不触及目的地；它只是简单地
删除陈旧的簿记。 `status`/`diff`/`reconcile` 是捕获
在下一个 `apply` 对它起作用之前漂移/冲突/孤立漂移的项目；一个
用户范围应用的目标 git-versioning （选择退出，默认 `prompt`）是
启用后的事后恢复网络 — 请参阅“回滚坏的
[用户指南](user-guide.md)中的“应用”。它完全不存在于
项目范围，因此项目范围漂移/冲突覆盖不会自动
恢复超出您自己对目标的源控制。

`agentsync status` 的格式化仪表板将 **融合** 项目显示为
**干净** - 两者都意味着 `apply` 没有什么可做的，并且区别
上面是分类器和 `status --json` 需要的簿记，而不是
人类扫描报告受益匪浅。 `status --legend` 打印此表
（作为 CLI 参考）； `status --json` 始终报告真实的类别。

对于结构化文件（JSON/JSONC/TOML，由
JSON 指针）和 **每个文件** 对于其他一切。密钥agentsync从未写入
是 **外键** - 浮出水面以提高意识，但从未接触过。

### 调和
针对偏差和冲突的交互式合并用户体验。对于每个漂流项目，您
选择 `[w]rite-back`（将目标编辑采用到源中），`[o]verride`（重新强加
源）、`[s]kip`、`[i]gnore`（添加到 `ignore.toml`）、`[d]iff` 或 `[q]uit`。
批量热键 (`W`/`O`/`S`) 和非交互式标志 (`--auto-writeback`,
`--auto-override`、`--auto-safe`) 存在用于脚本编写。

### 范围和项目源代码树
配置适用于**用户**范围（整台机器）或**项目**范围（一台
回购）。存储库通过在其根部保存 `.agentsync/` **源树** 来选择加入 -
与 `~/.agentsync/` 相同的磁盘布局（`agentsync.toml` 加 `mcp/`，
`skills/`、`subagents/`、`commands/`、`hooks/`、`lsp/`、`memory/`）。将其搭成脚手架
`agentsync init --scope project`；提交以与以下人员共享项目代理配置
合作者。项目范围始终是**显式选择加入** — 通过 `--scope
project`（从树的 cwd 向上走）或 `--project <path>`。命令默认值
**用户**范围；唯一的例外是在 *inside* a 中没有作用域的情况下运行
项目树不明确，因此agentsync **提示**项目与用户（或者，当
非交互式/`--no-input`，错误而不是猜测）。它从不默默无闻
作用于它刚刚检测到的树。项目树**覆盖**到
用户规范：项目条目替换具有相同 ID/名称的用户条目，新的
添加条目并附加项目内存。该项目的 `[agents]`
表格具有**权威性**：项目范围仅呈现给该项目的代理
它本身声明，永远不会是用户启用的代理 - 因此提交的树会产生
在每个协作者的机器上进行相同的渲染。一个声明没有的项目
代理是每个范围感知渲染路径上的硬错误 -
`apply`/`status`/`diff`/`reconcile`/`plugin upgrade`/`check`（声明代理
与 `agentsync agent add <name> --scope project`); `import --scope project`
保持可用于首先从本机配置引导树。一个
项目 `plugins/<id>.toml` 与 `disabled = true`
抑制存储库中该插件的组件。 （退役的M5单列
`.agentsync.toml` 标记不再被读取； `agentsync init --scope project`
打印如何迁移。）

### 秘密：`${secret:…}` 和 `${env:…}`
规范源从不存储明文凭据。你写
`${secret:github.token}` 或 `${env:HOME}`； agentsync **解决**这些在应用
时间 — `${secret:…}` 来自年龄加密文件，`${env:…}` 来自
环境。公共**接收者**密钥可以安全提交；私人的
**身份**密钥是每台机器的并且永远不会为您备份。

> 中心不变量：*已解决的*明文秘密绝不能被写回
> 进入规范源。类型系统强制执行此操作 — 请参阅
> [架构](architecture.md#8-secrets--how-the-leak-is-prevented) 和 `CLAUDE.md`。

### 状态 (`~/.agentsync/.state/`)
Gitignored 簿记：`targets.json`（最后应用的导致漂移的哈希值
可能检测）、应用锁、两阶段写暂存目录、首次应用
备份以及市场/插件缓存。键相对于 `${HOME}` 存储，因此
状态可以跨机器移植。

### 目标git备份（回滚历史记录）
（可选）每个 **用户范围目标目录** (`~/.claude`, `~/.codex`, …) 获取
它的**自己的仅限本地 git 存储库**：`apply` 在每次之后记录一个检查点提交
run 会更改那里的托管文件，并且 `agentsync revert` 将目录回滚到
之前的检查站。这与 `.state/` 不同 — `.state/` 是 agentsync 的
操作内存（漂移散列、窄上限的外部冲突备份），
而这些存储库是持久的、可浏览的、可恢复的回滚历史记录
*渲染的目标文件*。他们受以下机构管辖
`[destination_directory_git_backup]` 表并且**从不推送** — 渲染的
文件以明文形式保存秘密，因此历史记录保留在本地（规范来源
你推送仍然只携带`${secret:…}`引用）。因为那段历史是成立的
明文，本地 `.git` 目录在 **POSIX** 上强化为 `0700` 以限制
静态暴露；在 **Windows** 上，此 chmod 是静默无操作，因此文件系统
ACL（不是模式位）是那里的边界。请参阅 `apply`/`revert`
[用户指南](user-guide.md#command-reference) 和
[架构§4](architecture.md)。

### 市场/插件/投影
**市场**是插件的注册表（克劳德的市场格式）。一个
**插件**是一个组件包 - MCP 服务器、技能、子代理、命令、
钩子、LSP 服务器。 **投影**将每个分量独立地转换为
每个目标代理，这是扇出发生的地方：安装一次，登陆每个
支持该组件的已启用代理。

**应用扇出组件，而不是插件本身。** agentsync 拥有
`~/.agentsync/plugins/` 中的插件并将每个插件的*组件*写入
代理的本机路径（技能到 `~/.claude/skills/<name>/`，MCP 到
`mcpServers`等）。它故意不写入启用元数据
（Claude 的 `enabledPlugins`、Codex 的 `[plugins."x@y"]`）返回特工的
配置 - 一旦组件到达本机路径，代理会将它们读取为
常规组件，并且写回支持会与
每次应用时，代理都有自己的 `/plugin disable` UI。 `PluginIngester`
接口**设计为只读**：`import` 捕获插件启用状态
对于发现，`apply` 永远不会重新发出它。参见
[architecture.md § PluginIngester（只读）](architecture.md#pluginingester-read-only)
完整的理由。

**安装插件本身的代理不会被投影到。**因为应用
永远不会写回插件启用，代理自己的管理器安装的插件
保持安装在那里 - 因此将该插件的组件投影到同一个
代理的独立路径将复制其中的每一个（并双重触发其
钩子）。因此，`plugins/<id>.toml` 带有两个定位键：`agents`，您的
扇出允许列表和 `native_agents`，为插件提供服务的代理
他们自己。仅当 `agents` 将其定位为组件并且
`native_agents` 没有声明它。 `import` 为每个插件提供 `native_agents`
它发现安装了插件的代理，因此导入→应用往返
默认情况下不制造副本；拒绝警告它会，并且
卸载本机副本并删除条目将插件交给
代理同步。两个门都在一个地方强制执行——渲染
腰部 (`source.FilterForAgent`, via `secrets.Resolved.ForAgent`) — 绝不在
适配器。请参阅[用户指南](user-guide.md)。

**插件组件由其插件命名。**因为 apply 会扁平化
每个启用的插件的组件放入一个目标目录，两个插件
传送同名组件会在一个路径上呈现两个文件 - 因此
插件提供的子代理、技能或命令呈现为 `<plugin>-<name>`：
`feature-dev` 的 `code-reviewer` 着陆为 `feature-dev-code-reviewer`。组件
您在 `~/.agentsync/` 中手工创作的内容永远不会被重命名。插件的派生名称可以
仍然落在你的一个上（映射不是单射的）； Agentsync 报告称
冲突点名双方，而不是让任何一方默默获胜。参见
[architecture.md § 插件组件命名空间](architecture.md#plugin-component-namespacing)。

### 翻译报告和报道
每个 `apply` 和 `plugin explain` 均以报告结尾，显示每个插件每个代理，
登陆的内容 (`check` 仅对源进行 schema-lint 并验证秘密 - 它
不投影插件或打印覆盖率报告）：

- **✓ 原生** — 完全保真度；代理直接有概念。
- **◐预计** - 有损但合理的翻译，明确报告。
- **✗ 跳过** — 不存在诚实的翻译；记录下来，所以它永远不会沉默。

一行还可以通过以下方式报告插件对代理没有任何贡献
*配置*而不是通过失败的翻译 - `disabled` 插件
在此范围内关闭，`not-targeted` 当其 `agents` 白名单排除
代理，以及 `native` 当其 `native_agents` 列表遵循该代理自己的列表时
插件管理器。这些是经过深思熟虑的结果，因此它们永远不会呈现在
✗ 失败词汇。

Error 500 (Server Error)!!1500.That’s an error.There was an error. Please try again later.That’s all we know.