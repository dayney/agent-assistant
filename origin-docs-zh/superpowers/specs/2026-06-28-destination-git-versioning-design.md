# 目标自动 git 版本控制 + `agentsync revert` — 设计规范

**日期：** 2026-06-28
**状态：** 在第一次审核中获得批准 ([PR #119](https://github.com/spxrogers/agentsync/pull/119))。实施计划：`docs/superpowers/plans/2026-06-28-destination-git-versioning.md`。
**问题：** [#118 - 使用 git 自动版本化目标代理目录，因此错误的 `apply` 是可恢复的](https://github.com/spxrogers/agentsync/issues/118)

---

## 总结

`agentsync apply` 将规范的 `~/.agentsync/` 配置渲染到每个代理的
本机配置目录（`~/.claude`、`~/.codex`、`~/.cursor`、...），覆盖真实的
这些工具读取的文件。如今，唯一的目的地安全网是狭窄的、
gitignored 下的临时、仅限外部冲突的备份
`~/.agentsync/.state/backups/<ts>/`（上限为最近 20 个），并且有
根本没有撤消路径。

此功能为每个目标代理目录提供了**自己的仅限本地的 git 存储库**，因此
每个 `apply` 都会留下一个干净的、可浏览的检查点，并添加一个一流的
`agentsync revert` 命令将目标回滚到先前的检查点。的
存储库**永远不会推送到远程** - 它们仅用于本地回滚 -
这就是为什么他们可以接受明文秘密
渲染的文件已经包含。

两个用户可见的表面一起运送：

1. **自动版本控制** — 第一次写入未跟踪的目标目录时，
   提示（选择退出）`git init`；每次成功申请后发生的变化
   那里的托管文件，`git add` +提交一个检查点。
2. **`agentsync revert <agent>`** — 将目标目录恢复到之前的状态
   检查点（默认为最后一个），仅附加，然后打印不同步通知
   告诉用户在下次应用之前进行协调。

`agentsync.toml` 中存在一个新的全局 `[destination_directory_git_backup]` 表
模式 (`prompt` / `on` / `off`) 和可选的提交身份覆盖；
`apply --no-git-backup` 绕过 CI/脚本的所有内容。

---

## 目标/非目标

**目标**

- Agentsync 写入的持久、可浏览、可恢复的**本地**历史记录
  每个目标代理目录，范围为每个代理。
- 干净的每应用检查点提交 - 仅当应用实际更改时
  托管文件（同步时应用是无操作，提交步骤也是如此）。
- 一流的 `agentsync revert` 不会让用户手动驱动 git，并且
  这是诚实的，让目的地与规范不同步。
- 零新的第三方依赖——重用已经供应的
  `github.com/go-git/go-git/v5`（`v5.18.0`，今天在 `internal/cli/init.go` 中使用）。
- 保留`~/.agentsync/.state/`（散列+外部冲突备份+其修剪）
  完全按原样；这是对它的补充，而不是取代它。

**非目标**

- 将任何目标存储库推送到远程（按设计仅限本地 - 请参阅秘密）。
- 对规范 `~/.agentsync/` 源目录进行版本控制（用户已将其保留在
  他们自己的 dotfiles 存储库，仅包含秘密*引用*）。
- 将杂散的 `$HOME` 级托管文件（例如 `~/.claude.json`）快照到
  git net — 有关确定的边界和间隙，请参阅“提交范围和 git root”。
- 共享文件的子文件/每个 JSON 指针暂存 — 共享文件已提交
  整个（KISS，根据问题）。

---

## 锁定决定

这些都已解决——来自本期的“决定”部分加上四个设计
该规范预先解决了问题。

| ＃|决定|来源 |
|---|---|---|
| 1 | **每个目的地存储库，而不是伞式存储库。** 每个代理目录都有自己独立的存储库（`~/.claude/.git`、`~/.codex/.git`、...）。 |问题 |
| 2 | **通过 CLI 绕过和粘性“不再询问”选择退出**。默认 = 首次未跟踪写入时提示； CI/脚本标记会跳过它；记住的拒绝会保留在 `agentsync.toml` 中。 |问题 |
| 3 | **提交范围 = agentsync 管理的更改； KISS 共享文件。** 对于完全拥有的文件，仅该文件；对于共享文件（例如 `settings.json`），其中 agentsync 仅拥有一些键， `git add` + 按写入方式提交整个文件 — 无每指针切片。不要扫入agentsync从未接触过的完全不相关的文件。 |问题 |
| 4 | **发送一流的 `agentsync revert`** ，它将目标回滚到之前的应用检查点，并在完成时打印不同步的协调通知。 |问题 |
| 5 | **尊重现有的源代码控制。** 如果目标目录已经是一个 git 工作树（或嵌套在其中 - 例如保存在点文件中的 `~/.claude`），则 **不** 初始化或自动提交；最多只是表面上的暗示。 |问题 |
| 6 | **仅限本地，从不推送。** 对于这些存储库，从不 `git remote add`、从不 `git push` 来自 agentsync。本地历史中的明文秘密是一种可接受的权衡。 |问题 |
| 7 | **保留 `.state/`（包括 `.state/backups/` 及其修剪）原样。** |问题 |
| 8 | **恢复仅追加。** `revert` 从所选检查点重新具体化文件并记录**新**提交；历史永远不会被重写，因此错误的应用保持可审核状态，并且恢复本身是可恢复的。 | Q（本规格）|
| 9 | **`agentsync revert <agent>` 默认为最后一个检查点**，包括 `--to <ref>`、`--all` 和 `--dry-run`。 | Q（本规格）|
| 10 | 10 **Git root = 每个目标目录，包括。共享的跨代理目录**（例如 `~/.agents/skills`），每个目录都被重复删除到一个存储库。杂散的 `$HOME` 级托管文件（例如 `~/.claude.json`）远离 git（我们从不在 `$HOME` 处初始化存储库）并继续依赖 `.state/backups`。 （根据维护者指令，从“仅代理自己的目录”修订而来。） Q+维护者|
| 11 | 11 **专用代理同步提交身份** (`agentsync <agentsync@localhost>`)，可通过 `[destination_directory_git_backup]` 覆盖。即使没有设置全局 git 身份也可以工作。 | Q（本规格）|
| 12 | 12 **通过可选的 `VersionedDirs` 适配器扩展进行 Git 根目录** — 每个适配器都声明它所写入的目录集（配置目录 + 共享目录）； apply tail unions/de-nests/de-dups 它们因此像 `~/.agents/skills` 这样的共享目录是一个存储库。 （从每个维护者指令的“仅配置目录”演变为版本共享目录。）评论 #119 + 维护者 |
| 13 | **CI/脚本绕过标志 = `apply --no-git-backup`.** |评论 #119 |
| 14 | 14 **配置表 = `[destination_directory_git_backup]`、`mode = "prompt" \| "on" \| "off"`**（默认 `prompt`）。 |评论 #119 |
| 15 | 15 **没有 `agentsync git` 命令 — 仅配置编辑； `agentsync doctor` 表面状态为只读。** |评论 #119 |
| 16 | 16 **自动创建的标记 = 存储库根目录中的 `AGENTSYNC_LOCAL_HISTORY.md` + 存储库的 `.git/config` 中的 `[agentsync] managed = true`。** |评论 #119 |
| 17 | 17 **`revert` 默认 = 撤消最近的应用**（恢复 `HEAD~1` 内容，仅提交追加）。 |评论 #119 |

---

## 架构

该功能几乎完全存在于 apply tail 和新的 `revert` 命令中，
加上一个瘦 git 助手。它**不**触及渲染/分类/写入核心，
漂移分类器、秘密不变量或 `.state/`。



```
agentsync apply  ──► render.Apply(...) ──► written map[string]bool (abs dest paths)
                                              │
                          (existing pipeline) │  state saved, backups pruned
                                              ▼
                                   ┌────────────────────────┐
                                   │  NEW: destination git   │   internal/git (new)
                                   │  checkpoint step        │   + apply.go hook
                                   └────────────────────────┘
                                              │
              per agent that wrote files this run:
              1. resolve agent's git root dir (user scope)
              2. is it already a work tree?  ── yes ─► skip (decision #5), hint
              3. untracked + mode=prompt + TTY ─► prompt to init (opt-out)
              4. mode=on / accepted ─► git init (first time) + add changed
                 managed files under root + commit checkpoint
```





```
agentsync revert <agent> [--to <ref>] [--all] [--dry-run]
   └─► open agent's git root repo ─► resolve target checkpoint (default: prev)
        └─► restore worktree files from that checkpoint ─► commit "revert to <ref>"
             └─► print: destination now OUT OF SYNC with canonical; reconcile first
```



### 钩住的地方（混凝土锚）

- **应用tail** — `internal/cli/apply.go`，状态保存成功后
  （`state.Save(statePath, s)`，~第 217 行）和备份修剪
  （`render.PruneBackups(home, render.DefaultBackupKeep)`，〜第 222 行），之前
  最后的成功消息。托管文件集是 `written map[string]bool`
  由 `render.Apply` 返回 (`internal/render/pipeline.go`, `applyPlan` 返回
  `reports, written, unchanged, err`）。 `written` 中的所有路径都是绝对路径。的
  检查点步骤在 `--dry-run` 中**完全跳过**（没有写入发生）并且是
  当 `written` 为空时无操作（应用不更改任何内容）。
- **新命令** — `internal/cli/revert.go`，注册于
  `internal/cli/root.go` 的 `NewRoot()` `cmd.AddCommand(...)` 列表，结构如下
  现有的简单命令（`newDoctorCmd` / `newVerifyCmd`）。
- **新的帮助程序包** - `internal/git/git.go`，go-git 上的薄包装：
  `IsWorkTree(dir) bool`、`Init(dir) error`、`Open(dir) (*Repo, error)`、
  `AddAndCommit(repo, paths, msg, author) (commitHash, error)`，
  `RestoreToCheckpoint(repo, ref) error`、`Log(repo, n) ([]Checkpoint, error)`、
  `IsClean(repo) (bool, error)`。今天 go-git 的用法是单内联
  `git.PlainClone` 在 `init.go` 中；这将新表面合并在一处
  具有一个作者签名路径。 （注意：go-git 的 `Worktree.Commit` 需要一个
  `object.Signature`；专用身份就在这里，决定＃11。）

### 提交范围和 git root

> **在实施过程中更新（根据维护者指令）：** 的单位
> 版本控制是 **目标目录**，以及 **共享的跨代理目录
> （例如 `~/.agents/skills`） ARE 版本** — 重复数据删除到单个存储库。这个
> 取代之前的“仅代理自己的配置目录/广度层弃权”
> 下面的框架。

版本控制的单位是 **用户范围内的目标目录** — 每个代理的
配置目录加上它写入的任何共享目录。

- **Git 根解析 — `VersionedDirs` 适配器扩展。** 每个适配器已
  通过每个适配器 `ResolvePaths` 解析其自己的目标路径。我们添加一个
  小**可选扩展接口**（镜像 `PluginIngester` /
  `WarnEmitter` idiom) 因此适配器声明它写入的目录的**集合**：



```go
  // VersionedDirs is an OPTIONAL Adapter extension: an adapter declares the dirs
  // it writes into (its config dir + any SHARED cross-agent dir) so the apply tail
  // can git-version them. nil at project scope.
  type VersionedDirs interface {
      VersionRoots(scope Scope, project string) []string
  }
  ```



  **每个适配器都实现它**（深度和广度）。深度适配器返回它们的
  配置目录加上他们的共享技能目录（Codex → `~/.codex` + `~/.agents/skills`;
  OpenCode → `~/.config/opencode` + `~/.claude/skills`）。广度层派生
  它的根源来自于它的 `generic.Spec` 目标。应用尾部然后**联合**所有
  跨启用的适配器的根，**去嵌套**（删除嵌套在另一个根下的根 -
  `~/.claude/skills` 折叠成 `~/.claude` — 所以在 a 中永远不会有 repo
  repo）和 **de-dups** （共享目录，如 `~/.agents/skills`，由 Codex 声明
  和几个广度代理，是一个存储库，对其所有文件进行一次检查点）。
  项目范围返回 nil （这些目录位于用户自己的项目存储库中）。
  由 `TestVersionedDirsContract` + `TestEnabledVersionRoots_DedupAndDenest` 固定。

  *考虑并拒绝的替代方案：*纯粹从`written`派生根源
  路径。 Skills-dir 根 (`~/.agents/skills`) 没有父级为
  正是那个目录（技能位于 `<root>/<name>/SKILL.md`），所以叶派生
  无法恢复；适配器声明其根源是干净的答案。

- **哪些文件被暂存。** 这次运行的每个根目录下写入的托管文件，加上
  任何跟踪的删除（仅删除仍应用检查点）。对于共享文件
  （例如 `~/.claude/settings.json`）**整个文件**按写入方式暂存
  （决定#3），包括位于同一位置的用户密钥。我们从来没有`git add -A`，所以未被追踪
  用户放入目录的文件不会被扫入。（已经跟踪的 `.git`
  more 是用户现有的回购案例 → 决策#5，我们不碰它。）

- **杂散`$HOME`级文件超出范围（确定的间隙）。** Claude 写道
  `~/.claude/` *和* `~/.claude.json` 直接位于 `$HOME` 中。我们不会**启动一个
  仓库位于 `$HOME`。因此 `~/.claude.json` （以及任何其他顶级托管文件）是
  **未**在 git 历史记录中捕获；它继续依赖现有的
  `.state/backups` 外部冲突备份与今天完全相同。这被记录为
  用户指南和 `revert` 帮助文本中的已知限制。

- **范围。** 这针对 **用户范围** 目的地。项目范围渲染
  写入项目树（例如`<repo>/.cursor/…`），这实际上是
  总是已经是一个 git 工作树 → 决策 #5 使其成为自然的无操作。的
  检查点步骤仅在用户范围内运行； `agentsync revert` 运行于
  用户范围代理目录。

### 检测现有工作树（决定#5）

使用 go-git `PlainOpenWithOptions(dir, &git.PlainOpenOptions{DetectDotGit: true})`
因此检测到**嵌套在**现有存储库（点文件用户）内的目标，
不仅仅是根部的 `.git`。如果检测到并且该存储库**不是**一个
agentsync 本身创建（参见下面的“标记”），agentsync 既不初始化也不初始化
自动提交；它可能会打印一行提示，表明自动版本控制已禁用
因为该目录已经在源代码控制之下。

**回购标记（决定#16）。** 区分“自动创建的回购代理同步”和“
用户自己的存储库”，自动初始化将 `[agentsync] managed = true` 键写入
存储库自己的 `.git/config`，以及生成的 `AGENTSYNC_LOCAL_HISTORY.md`
通知文件（如下）。自动提交和 `agentsync revert` 仅在存储库上运行
携带此标记。用户预先存在的存储库永远不会获得标记，因此
agentsync 不参与其中。

### 自动初始化存储库内容

第一次初始化时，agentsync 在存储库根目录中写入生成的通知文件（例如
`AGENTSYNC_LOCAL_HISTORY.md`）明确：

- 这是**仅限本地**代理同步回滚历史记录，
- 它**可能包含明文秘密**并且**绝不能推送**，
- 如何回滚（`agentsync revert <agent>`）。

该通知本身是在初始检查点中提交的。回购协议隐私：渲染
文件已经是 `0o600` (`internal/iox/atomic.go`)； `.git` 目录已创建
`0o700` 因此历史记录仅供用户使用。

---

## 配置模式 — `[destination_directory_git_backup]` 表

`~/.agentsync/agentsync.toml` 中的新全局表，解析为
`DestinationGitBackupConfig` 结构已添加到 `source.Config`
(`internal/source/schema.go`，与 `Agents`、`Updates`、`Secrets` 一起，
`Memory`）。表名故意明确——这些存储库是以下内容的备份
*目标目录*，而不是一般的“git”配置。项目叠加合并
像其他人一样（`internal/project`），尽管这是一个用户范围的问题
练习。



```toml
[destination_directory_git_backup]
# Behavior for git-backing the rendered destination dirs. Tri-state:
#   "prompt" (default) — on first write to an untracked agent dir, ask to init.
#   "on"               — init + checkpoint silently (set after the user says yes).
#   "off"              — never init/commit/prompt (set by "don't ask again").
mode = "prompt"

# Optional commit-identity overrides (defaults shown).
author_name  = "agentsync"
author_email = "agentsync@localhost"
```





```go
// internal/source/schema.go
type DestinationGitBackupConfig struct {
    Mode        string `toml:"mode,omitempty"`         // "prompt" (default) | "on" | "off"
    AuthorName  string `toml:"author_name,omitempty"`
    AuthorEmail string `toml:"author_email,omitempty"`
}
// added to Config as:
//   DestinationGitBackup DestinationGitBackupConfig `toml:"destination_directory_git_backup"`
```



状态转换：

|活动 |结果 |
|---|---|
|表不存在/`mode` 未设置 |视为 `"prompt"`。 |
|用户在 init 提示符下回答 **yes** |坚持`mode = "on"`。 |
|用户回答**否** |跳过此运行；离开 `mode = "prompt"`（将再次询问）。 |
|用户回答**否+不要再问** |坚持`mode = "off"`。 |
| `apply --no-git-backup`（每次运行）|强制跳过该调用的初始化/提交，无论 `mode` 如何。永远不会改变配置。 |
|稍后重新启用 |用户编辑 `agentsync.toml` (`mode = "prompt"`/`"on"`)。 |

将持久化的 `mode` 写回 `agentsync.toml` 会遍历现有的
规范的 TOML writer（保留注释和顺序），相同的路径
`agentsync mcp add` 等使用 - 它不得破坏用户的手动编辑。的
当前模式也通过 `agentsync doctor` 以只读方式显示（请参阅 CLI 表面）。

---

## CLI 界面

### `agentsync apply` — 一个新标志



```
--no-git-backup   Skip destination git init/checkpoint for this run (CI/scripting).
                  Does not modify agentsync.toml.
```



关于 apply 的其他所有内容均保持不变。在 `--no-git-backup` 或非交互式中
(`--no-input` / no TTY) 运行，agentsync 从不提示；与 `mode = "on"` 一起
仍然静默检查点（不需要提示），使用 `mode = "prompt"` 并且没有 TTY
它会跳过 init （无法询问）并打印一行提示。

### `agentsync doctor` — 表面状态（无新命令）

**没有**专用的 `agentsync git`/启用/禁用命令（仅保留配置
鉴于琐碎 — 决定，PR #119）。相反，`agentsync doctor`
(`internal/cli/doctor.go`) 获得一个小的只读检查，报告
目的地 git 备份模式，以及每个托管代理目录，是否是
Agentsync 版本控制，已受外部源代码控制，或未跟踪。改变
该行为是单行 `agentsync.toml` 编辑。

### `agentsync revert` — 新命令



```
agentsync revert <agent> [flags]

  Restore an agent's destination dir to a prior apply checkpoint. Append-only:
  records a new commit; never rewrites history.

Flags:
  --to <ref>     Checkpoint to restore (commit hash / relative like HEAD~2).
                 Default: the previous checkpoint (undo the most recent apply).
  --all          Revert every agentsync-managed destination dir to its last
                 checkpoint. Mutually exclusive with a positional <agent>.
  --dry-run      Show what would change (files + target commit) without writing.
  --scope        user (default). Project dirs are left to their own repo.
```



行为：

1.解析代理的git root（用户范围）。如果目录不是，则明显错误
   agentsync 管理的存储库（无标记/从未启动）。
2. 解析目标检查点（默认为`--to`或前一个检查点）。
3. 将工作树文件恢复到该检查点的内容并记录新的提交
   `agentsync revert: <agent> → <short-ref>`。仅附加（决定#8）。这个
   **仅逐个文件应用跟踪的 HEAD↔目标增量** - 不得使用
   go-git 的 `HardReset`，它（与 `git reset --hard` 不同）枚举并删除
   每个未跟踪/gitignored 的工作树文件。仅接触不同的路径叶子
   用户自己的未跟踪/gitignored 文件完好无损（问题＃128）； `--dry-run` 注释
   当存在未跟踪的文件时（gitignored 文件也会被保留，但是
   git status 无法枚举它们，因此它们没有列出）。
4. 打印 **不同步通知**（决定 #4）：

   > `agentsync revert` 已完成。目标目录 `<dir>` 现已退出
   > 与agentsync 配置同步。请根据需要进行协调（例如
   > `agentsync reconcile` / `agentsync import`) 在下一个 `agentsync
   > apply` 之前，以避免重新丢失这些更改。

`--dry-run` 打印目标提交和文件差异摘要并退出
承诺。

### 提交消息格式

每个应用检查点：



```
agentsync apply: <agent> (<scope>) — <n> file(s)

<rel/path/under/root/1>
<rel/path/under/root/2>
…
```



恢复检查点：



```
agentsync revert: <agent> → <short-target-ref>
```



两者均由专用身份撰写（决定#11）。

---

## Secrets — 仅限本地的护栏（不变的不变量）

渲染的目标文件包含在应用时解析为**明文**的秘密
时间（规范源保存 `${secret:…}`/`${env:…}` 引用；即
不变）。因此，目标 git 存储库在历史上保存着明文秘密。
这是从该问题中故意接受的权衡，通过保持
存储库**仅限本地**：

- **从不 `git remote add`，从不 `git push`** 来自这些存储库的 agentsync。的
  `internal/git` 帮助程序根本不公开推送/远程界面 - 没有代码
  可以添加遥控器的路径，因此这不会意外回归。
- 生成的回购通知警告历史记录仅限本地，并且可能包含
  秘密，绝不能强行推行。
- `.git` 目录是 `0o700`；渲染的文件保留 `0o600`。

这 **不会触及** agentsync 秘密不变量：没有 `secrets.Resolved` 是
写回规范，`walkSecretFields` 不变，`capture.Capture` 的
故障关闭逆止器未改变。 git 步骤仅提交已经存在的文件
存在于目的地的磁盘上——它永远不会通过规范进行往返
来源。

---

## 边缘情况

- **不应用任何更改** → `written` 该代理为空 → 没有提交（干净
  历史，没有空的检查点）。
- **Dir 已经是一个工作树/嵌套在一个工作树中** → 没有初始化，没有自动提交
  （决定＃5），一行提示。
- **没有 TTY 和 `mode = "prompt"`** → 无法询问 → 跳过 init、提示；永远不会阻止。
- **`mode = "off"` 或 `--no-git-backup`** → 完全跳过。
- **一次应用多个代理** → 每个代理存储库提交一个检查点（
  每个代理分组是自然的：每个代理都有自己的根+存储库）。
- **`revert` 在从未启动的代理上** → 清除错误，建议运行
  首先在启用自动版本控制的情况下应用。
- **`revert --to` 未知参考** → 清除错误，没有更改。
- **go-git 不可用功能/无身份提交** → 专用身份
  消除了常见的“未配置 git 身份”故障；显示任何 go-git 错误
  包裹（`fmt.Errorf("committing checkpoint for %s: %w", agent, err)`）。
- **杂散 `$HOME` 文件已更改，但代理目录未更改** → 无检查点（间隙为
  记录在案； `.state/backups` 仍然涵盖外部冲突情况）。

---

## 要更新的文档（相同的更改，根据 `CLAUDE.md`）

新命令 + 新 `agentsync.toml` 表是 CLI 表面 + 模式更改，因此
相同的 PR 必须更新：

- `docs/user-guide.md` — 命令参考（`apply --no-git-backup`，新 `revert`
  部分、新的 `doctor` 状态行）、 `agentsync.toml` 布局块
  （`[destination_directory_git_backup]` 表）和目标版本控制
  概念。
- `README.md` — 快速入门/快速参考表（添加 `revert`；注意
  自动版本控制）。
- `website/src/content/docs/reference/cli.mdx` —“概览”表格行 + a
  `### revert` 部分 + `apply --no-git-backup` 标志。
- `docs/concepts.md` — 三态模型：注意目标目录现在可以携带
  仅限本地的 git 历史记录（与 `.state/` 不同）。
- `docs/architecture.md` — 检查点步骤位于应用尾部的位置；的
  永不推不变； `.state/` 未受影响。
- `internal/cli/init.go` `initialAgentsyncTOML` 常量 — 添加注释
  `[destination_directory_git_backup]` 示例，因此 `agentsync init` 搭建它的脚手架。
- `CHANGELOG.md` — `[Unreleased] / Added`。

能力矩阵**不**受影响（这不是每个代理组件
能力）。

---

## 审核中已解决（PR #119）

所有先前公开的决定都在第一次规范审查中得到解决——并入
上面的正文和锁定决策表（第 12-17 行）：

1. **Git-root 机制** → 可选的 `VersionedDirs` 适配器扩展（SET
   每个适配器的目录数，通过应用尾部联合/解除嵌套/重复数据删除）。 ✅
2. **CI 绕过标志名称** → `apply --no-git-backup`。 ✅
3. **配置表+模式值** → `[destination_directory_git_backup]` with
   `mode = "prompt" | "on" | "off"`（`off` 的相反是 `on`）。 ✅
4. **重新启用人体工程学** → 仅配置编辑； **无** `agentsync git` 命令；
   `agentsync doctor` 以只读方式显示当前状态。 ✅
5. **通知文件 + 标记** → 仓库根目录下的 `AGENTSYNC_LOCAL_HISTORY.md` 加一个
   `[agentsync] managed = true` 输入存储库自己的 `.git/config` 作为
   “agentsync 创建”标记。 ✅
6. **`revert` default** → 撤消最近一次应用，即恢复检查点
   *在* `HEAD`（`HEAD~1` 内容）之前并仅以追加方式提交。 ✅

---

## 测试计划/成功标准

所有与 FS 相关的测试都在容器内 (`testenv.RequireContainer`) 上运行
`afero`/`t.TempDir`，stdlib `testing`，表驱动。

- **帮助单元测试** (`internal/git`): init 创建一个 `.git` (0o700)
  已提交通知； add+commit 仅暂存给定路径和作者
  专用身份； `IsWorkTree` 检测根处的 `.git` 和嵌套的；
  恢复到检查点再现先前的字节； `IsClean`正确。
- **Apply-tail 集成** (`internal/cli`)：首先应用于未跟踪的目录
  `mode = "on"` 恰好生成一个包含写入的托管检查点
  文件和**不是**不相关的文件；无操作应用不会产生新的提交；一个目录
  这已经是一个工作树保持不变（没有添加代理同步提交）；
  `--no-git-backup` 和 `mode = "off"` 跳过；杂散 `~/.claude.json` 不存在于
  克劳德·回购历史。
- **提示行为**：TTY + `mode="prompt"` → 提示； “不”离开`mode=prompt`；
  “不+不要再问”仍然存在`mode=off`；非交互式永远不会阻塞。
- **`revert`**：默认恢复最后一次应用（文件与先前的检查点匹配），
  记录新的仅追加提交，打印不同步通知； `--to`，
  `--all`、`--dry-run`表现良好；显然未知的引用/未初始化的目录错误。
- **从不推送不变**：测试（或中缺少任何远程/推送符号）
  `internal/git`) 断言未配置任何远程且不存在推送 API。
- **秘密不变量不变**：现有的 `internal/secrets` /
  `internal/capture` 警卫仍然通过；没有 `secrets.Resolved` 到达源
  writer（此功能不添加目标→源路径）。
- **配置往返**：`[destination_directory_git_backup]`表解析；
  持久化 `mode` 会保留 `agentsync.toml` 中的注释/键顺序。

---

## 超出范围（目前）

- 将任何目标存储库推送到远程。
- 对规范 `~/.agentsync/` 源目录进行版本控制。
- 共享文件的子文件/每指针暂存。
- 将杂散的 `$HOME` 级托管文件捕获到 git 中（已记录的差距）。
- 目标 git 历史记录的计划/自动修剪（git 本身 +
  仅本地作用域使其优先级较低；如果历史变得难以掌握，请重温）。