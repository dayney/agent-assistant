# 命名空间插件提供的组件由插件 — 设计规范

**日期：** 2026-07-28
**状态：** 在头脑风暴中获得批准；经过四镜头对抗后修正
审核 — 请参阅[修正案](#amendments-after-review)了解两个设计点
这份文件最初是错误的。
**问题：** [#211 — 跨插件子代理名称冲突无法解决 — Codex 错误命名了用户无法执行的补救措施](https://github.com/spxrogers/agentsync/issues/211)

---

## 总结

两个已安装的插件，每个插件都有一个同名的组件
`agentsync status`和`agentsync apply`退出1，并且每个都纠正错误
建议是用户在结构上无法执行的一项。复制品有两个
*官方库存*插件 - `feature-dev@claude-plugins-official` 和
`pr-review-toolkit@claude-plugins-official` — 每次运输
`agents/code-reviewer.md`。

修复：**插件提供的子代理、技能和命令始终是命名空间的
在投影时通过他们的插件**。 `feature-dev` + `code-reviewer` 呈现为
`feature-dev-code-reviewer`； `pr-review-toolkit` + `code-reviewer` 呈现为
`pr-review-toolkit-code-reviewer`。用户手动编写的组件
`~/.agentsync/` 从未被重命名。

---

## 目标/非目标

**目标**

- 库存的双插件安装可以干净地应用，两个组件都可以访问。
- 插件组件获得稳定、可预测的名称，不依赖于什么
  否则已安装。
- 碰撞检测器保留，对于命名空间无法到达的情况，其
  消息变得可操作。
- 现有用户的过时预重命名目标文件将被删除，而不是孤立。

**非目标**

- 规范侧覆盖/优先旋钮（让用户固定哪个插件获胜
  有争议的名称，或重命名插件提供的组件）。延期：自动
  命名空间使报告的案例无需任何命名空间即可工作。
- 修复 `import` 重新捕获插件投影组件（请参阅
  [已知相邻间隙](#known-adjacent-gap))。

---

## 为什么修复不能存在于适配器中

两个插件都项目到 `source.Subagent{Name: "code-reviewer"}` — *文件
Stem* 发生冲突，而不仅仅是 frontmatter `name`。插件来源已被删除
在 `internal/marketplace/loadprojected.go:91` 处，一个简单的
`c.Subagents = append(c.Subagents, proj.Subagents...)` 和 `source.Subagent` 有
无处可存。当任何适配器可以检测到冲突时，
解决该问题所需的信息已经消失。

它也不是特定于法典的。 Codex 适配器首先出错
(`internal/adapter/codex/subagent.go:65`)，但是 Claude 适配器发出两个写入
操作相同 `~/.claude/agents/code-reviewer.md`。 `seen` 地图位于
`internal/render/pipeline.go` 不会按代理重置，因此它是跨代理的
发散防护也会在单个适配器*内*触发，并发出消息中止
将冲突错误地归因于第二个代理：



```
agent "claude" renders different content than an earlier agent for the same path …
```



技能 (`skills/<name>/SKILL.md`) 和命令 (`commands/<name>.md`) 是
同一类：呈现到名称派生的目标路径的名称键控组件。

---

## 设计

### 1. 在投影时重命名，而不是在适配器中重命名

`projectOnePlugin` (`internal/marketplace/loadprojected.go`) 已持有
插件的文件系统 ID。 `projectWithFuncs` 返回后，每个投影
子代理、技能和命令均标有其出处，其 `Name` 是
重写为命名空间形式。

因为 `Name` 是每个适配器派生其目标路径和标识的内容
从此，所有渲染站点都变得正确，**没有适配器更改**。冲压件
*在*`resolveConflicts`之后运行，因此内部插件`dedupOrConflict`
`reflect.DeepEqual` 比较不受影响。

`ProjectInstalled` 共享 `projectOnePlugin`，因此 `agentsync plugin explain`
报告渲染生成的相同命名空间名称。

### 2.分隔符：`-`

是被迫的，不是选择的。 Claude Code 将子代理 `name` 记录为“唯一的
使用小写字母和连字符的标识符”，因此 `:` 不可用 —
熟悉的 `plugin:agent` 形式是一个*范围标识符* Claude Code 源自
插件目录，永远不会将值写入 `name` 字段。法典规定没有
字符集规则并将 `name` 视为代理的身份，因此连字符的名称是
在那里同样有效。

派生名称已使用 `source.ValidateComponentID` 进行验证，这已经
拒绝路径分隔符、`..`、`:` 和控制/双向符文 — 因此是病态的
插件 id 无法生成不可写或欺骗性的组件名称。

### 3. 架构

`source.Subagent`、`source.Skill` 和 `source.Command` 每个增益：

|领域|意义|
| --- | --- |
| `Plugin string` |提供插件的 id；对于手动编写的组件为空 |
| `BaseName string` |命名空间名称前的名称，用于报告和 `explain` |

两者都是 `toml:"-"`。这些是文件支持的组件，其规范形式是
磁盘上的文件，并且投影的组件永远不会写回
`~/.agentsync/`，因此这两个字段都不会被序列化。

安全对抗 `TestNewSecretFieldGuard`：该防护覆盖 `MCPServerSpec`，
`LSPServerSpec`、`Hook` 及其包装器。子代理、技能和命令是
秘密步行者故意不访问的文本组件，因此没有新字段
加入 `walkSecretFields`。

### 4. Frontmatter `name` 重命名后

仅重命名 `Name` 会使两个组件仍然发生冲突：

- Codex 适配器在派生时更喜欢 `Frontmatter["name"]` 而不是 `s.Name`
  Codex 代理身份 (`internal/adapter/codex/subagent.go:58`)。
- 克劳德的特工技能需要 frontmatter `name` 来匹配技能
  目录名称。

因此投影也会重写 `Frontmatter["name"]` **当密钥存在时**，
当它不存在时就让它不存在。这保留了 #144 合同
故意不同的 frontmatter 名称在渲染 → 摄取中保留下来，同时保留
身份与路径一致。

### 5. 硬错误仍然存​​在；消息已修复

命名空间无法达到每次冲突：

- 两个具有相同有效名称的手工编写的规范组件；
- 插件 `a` 运送 `b-c` 与插件 `a-b` 的病态情况
  运送 `c`。

`codex/subagent.go` 中的检测器仍然存在。当前正在打印其消息
`%q and %q` 来自两个文件茎，对于报告的情况呈现为
`"code-reviewer" and "code-reviewer"` — 相同的字符串两次，没有来源。
它获得出处 (`from plugin X` / `hand-authored`)，因此一半
旨在识别冲突的消息在冲突发生时就提供了信息。

### 6. 孤儿回收扩展到子代理和命令

`skillOrphanDeletes`（现在 `orphanDeletes`、`internal/render/state_apply.go`）仅回收
`skills/` SourceID。 `PruneStaleState` 删除目的地的状态条目
停止渲染但不会删除文件。

如果不扩展回收，始终命名空间将保留所有现有的
用户在两个新的旁边有一个过时的 `~/.claude/agents/code-reviewer.md`
命名空间文件——Claude Code 会加载它。该修复将*添加*一个
重复代理而不是删除代理。因此，填海工程延伸至
`subagents/` 和 `commands/` SourceID 前缀，匹配已有的技能
表现得好。

### 7.升级通知

始终命名空间会为每个用户重命名每个插件提供的句柄，因此它
获取 `upgradeNotices` 条目（#203 机制）和中的匹配部分
`website/src/content/docs/reference/upgrading.mdx`，根据该表的记录
两者并行维持的合同。

---

## 测试

根据 CLAUDE.md 保真度规则，锚定在磁盘上的工件 — 预言机是
渲染的文件树，而不是解析的模型。

Error 500 (Server Error)!!1500.That’s an error.There was an error. Please try again later.That’s all we know.

**§6（孤儿回收）现已不完整。** 回收涵盖退休人员
`agents/` SourceID 拼写以及 `subagents/` 和 `commands/`。的
规范目录重命名在同一版本中提供，因此升级用户的
state 仍然保留旧的拼写，并且对于其用户来说唯一的重写器无操作
子代理仅来自插件——这是报告的情况。没有它
他们的重命名前目的地永远不会被回收，这与
升级通知承诺。

**在审查期间也扩大了范围：** 捕获拒绝涵盖 MCP 服务器、LSP
服务器和钩子（每个 *handler* 的钩子，因为一个规范 `hooks/<event>.toml`
拥有来自许多来源的处理程序），而不仅仅是三种名称键控类型；和
下面的“已知相邻间隙”被关闭而不是推迟。用户的钩子处理程序
还声明永远不会被视为插件提供，即使插件发布了
相同的 - 如果没有该检查，`import` 会删除用户自己的钩子处理程序
和 `WriteHooks`' 整个文件替换将其从规范源中删除。的
重写是**仅钩子**，故意的：每个其他作家都是
每个组件一个文件，因此拒绝不会删除任何内容，而捕获组件
该插件还提供了使两者在以后的变异中分歧和楔入的功能
加载到`checkProjectedConflicts`。请参阅 Architecture.md § 5。