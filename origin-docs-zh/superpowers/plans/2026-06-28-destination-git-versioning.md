# 实施计划 — 目标自动 git 版本控制 + `agentsync revert`

**日期：** 2026-06-28
**问题：** [#118](https://github.com/spxrogers/agentsync/issues/118)
**规格：** [`docs/superpowers/specs/2026-06-28-destination-git-versioning-design.md`](../specs/2026-06-28-destination-git-versioning-design.md)
**公关：** [#119](https://github.com/spxrogers/agentsync/pull/119)

> **注意（实施后）：** 在审查过程中，设计以两种方式演变：
> 下面的计划早于 - 请参阅最终设计的规格。 (1) 可选适配器
> 扩展名是 **`VersionedDirs.VersionRoots() []string`** （一组*一组*目录），而不是
> `VersionedHome.HomeDir() (string,bool)`; **每个**适配器都实现它（深+
> 广度），并应用尾部联合/去嵌套/去重复根，以便**共享
> 像 `~/.agents/skills` 这样的跨代理目录是版本化的，重复数据删除到一个存储库**。
> (2) `revert` 获取全局锁定并快照之前未提交的跟踪编辑
> 恢复；从 #128 开始，恢复仅应用跟踪的 HEAD↔目标增量
> 逐个文件（无硬重置），因此用户的未跟踪/gitignored 文件会保留下来
>（无数据丢失）。

---

## 目标

指定每个 **用户范围** 目标代理目录 (`~/.claude`, `~/.codex`, `~/.cursor`,
...）它自己的 **仅限本地** git 存储库，因此每个更改托管文件的 `apply`
留下检查点提交，并发送 `agentsync revert` 将目录回滚到
先前的检查点（仅附加）。添加 `[destination_directory_git_backup]` 配置
表、`apply --no-git-backup` 旁路和只读 `agentsync doctor` 状态
线。切勿推送这些存储库。

## 架构（每个部分落地的地方）



```
internal/git/                    NEW package — the only go-git surface for this feature.
  git.go        Repo open/init/detect, Identity, marker, notice, perms.
  commit.go     Stage specific files + commit with the dedicated signature.
  restore.go    Append-only restore: materialize a checkpoint's tree, commit on top.
  log.go        List checkpoints (short hash, subject, time).
  (NO remote.go / push — the absence IS the never-push invariant.)

internal/adapter/adapter.go      NEW optional interface: VersionedHome.
internal/adapter/<each>/*.go     Implement HomeDir(scope, project) (string, bool).

internal/source/schema.go        NEW DestinationGitBackupConfig + Config field.
internal/project/project.go      Merge the new table in the project overlay.

internal/cli/gitbackup.go        NEW: apply-tail orchestration (group written → per
                                 agent → detect/prompt/init/commit) + mode persistence.
internal/cli/apply.go            Add --no-git-backup; call the orchestration after
                                 PruneBackups (line ~222).
internal/cli/revert.go           NEW `agentsync revert` command.
internal/cli/doctor.go           NEW "Destination git backup" section.
internal/cli/root.go             Register newRevertCmd().

docs/…, README.md, website/…, CHANGELOG.md, init.go initialAgentsyncTOML.
```



## 技术堆栈/实施者不得重新推导的事实

- **go-git 已经供应：** `github.com/go-git/go-git/v5 v5.18.0` （今天使用
  仅由 `internal/cli/init.go:244` 中的 `git.PlainClone` 执行。没有新的依赖项。
- **托管文件集**是由返回的 `written map[string]bool`
  `render.Apply(...)` — 请参阅 `internal/cli/apply.go:178`：
  `collisions, written, unchanged, applyErr := render.Apply(plan, reg, s, home, userHome, sc, projectRoot)`。
  键是本次运行实际写入的**绝对**目标路径。
- **应用尾钩点**是紧随其后的`internal/cli/apply.go`
  `_ = render.PruneBackups(home, render.DefaultBackupKeep)`（第 ~222 行）及之前
  成功消息（第 224 行）。状态已经保存； `written`、`plan`、`reg`、
  `agents`、`sc`、`projectRoot`、`userHome`、`home`、`p` 均在范围内。
- **注册表：** `reg.Lookup(name) adapter.Adapter` 和 `reg.Names() []string`
  (`internal/adapter/registry.go:25,28`)。适配器的构造是
  `Options{TargetRoot: home}` 其中 `home = paths.HomeDir(paths.OSEnv{})`
  (`internal/cli/registry_internal.go`)。注册：claude、opencode、codex、
  光标、gemini、continuedev、windsurf、roo、cline + 每个 `generic.New(spec,…)`
  `generic.Specs()`。
- **每个适配器通过 `ResolvePaths(targetRoot,
  project, projectScope)` 解析其自己的配置目录**，返回带有 `ConfigDir` 字段的结构（例如
  `cursor/paths.go:13` → `~/.cursor`）。用户范围内的 `ConfigDir` 是 git
  根。 （克劳德还在 `$HOME` 处写了 `~/.claude.json` — **超出范围**，
  决定＃10。）
- **TTY /非交互式助手已存在** `internal/cli/apply.go` 中：
  `stdinIsTerminal(cmd)`（第495行）和`noInputFlag(cmd)`（第481行）；提示
  习语是 `promptScopeChoice`（第 504 行，`bufio.NewReader(cmd.InOrStdin())`）。
- **就地 `agentsync.toml` 编辑** 保留编辑范围之外的所有内容
  通过线拼接 + `iox.AtomicWrite(p, …, 0o644)` 的部分 — 请参阅
  `writeAgents`/`readAgentsyncTOML` 在 `internal/cli/agent.go:144–240` 中。坚持
  `mode` 遵循相同的模式；不要 `toml.Marshal` 整个 `Config` （它
  核武器评论）。
- **原子写入：** `iox.AtomicWrite(dest, data, mode)` (`internal/iox/atomic.go`)
  首先写入 `0o600`，然后写入 chmod。渲染的文件已经是`0o600`。
- **`time.Now()` 警告仅适用于 `internal/render` / `internal/state`** —
  `internal/git` 和 `internal/cli` 可以直接调用 `time.Now().UTC()`（提交
  时间戳不提供哈希）。在 `CLAUDE.md` 中确认。

## 全局约束（适用于每个任务）

- **仅标准库测试**，带有 `name` 字段 + `t.Run` 的表驱动； `t.Helper()`
  在致命的帮手中。没有作证/gomega。
- **FS-touching测试**使用`t.TempDir()`（真正的FS - go-git需要一个真正的工作树，
  `afero.MemMapFs` 不会返回 go-git）并且 **必须** 调用
  `testenv.RequireContainer(t)` / `MustRunInContainer()` 因此他们拒绝在
  没有 `AGENTSYNC_TEST_IN_CONTAINER=1` 的主机。
- **错误**换行：`fmt.Errorf("doing X: %w", err)`；与 `errors.Is/As` 匹配。
- **永远不要**在 `internal/git` 下的任何位置添加 git 远程或调用任何推送 API。
- **提交**：常规，范围 - `feat(git):`，`feat(adapter):`，
  `feat(cli):`、`feat(source):`、`docs(...):`、`test(...):`。
- 在每个任务之后运行 `just build`，然后运行 ​​`just test-fast`；的 lint 怪癖
  容器是 `GOTOOLCHAIN=go1.26.2 go run github.com/golangci/golangci-lint/v2/cmd/golangci-lint@v2.12.2 run ./...`
  （请参阅 `CLAUDE.md`“容器/云会话”注释）。重新上演任何内容 `just lint`
  在提交之前重写。

## 如何实施这个计划

里程碑是有序的，因此每个里程碑都建立在最后一个里程碑的基础上并保持绿色。 **M0**（
`internal/git` 包）完全隔离并经过单元测试，零代理同步
耦合——首先建造并着陆。 **M1** 添加了规范/适配器管道。
**M2** 连接应用尾部。 **M3** 添加 `revert`。 **M4** 是医生 + 文档（
`CLAUDE.md` 文档同步规则意味着文档与行为处于相同的 PR 中）。检查
边走边关闭每个 `[ ]`。

---

# 里程碑 0 — `internal/git` 帮助程序包（隔离，经过单元测试）

这里没有代理同步类型——真实目录上的纯 git 机制。这是
该功能的整个 go-git 表面；将其保存在一个包装中使得
从不推送不变可审计（根本没有要调用的推送函数）。

## 任务 0.1 — 包骨架，`Identity`，工作树检测

**文件**
- 创建`internal/git/git.go`
- 创建`internal/git/git_test.go`

**此任务公开的接口**


```go
package git

// Identity is the author/committer for checkpoint commits.
type Identity struct{ Name, Email string }

// DefaultIdentity is used when the config overrides are empty.
var DefaultIdentity = Identity{Name: "agentsync", Email: "agentsync@localhost"}

// State classifies a destination dir for the apply tail.
type State int
const (
    StateUntracked      State = iota // no work tree anywhere at/above dir
    StateAgentsyncOwned              // a work tree WE created (carries the marker)
    StateForeign                     // a work tree the user owns (no marker) — leave alone
)

// Detect reports how dir is tracked. It uses DetectDotGit so a dir nested inside
// a user's existing repo (dotfiles) is StateForeign, not StateUntracked.
func Detect(dir string) (State, error)
```



**步骤**
- [ ] 使用导入 `gogit "github.com/go-git/go-git/v5"` 写入 `git.go` 并
  `"github.com/go-git/go-git/v5/config"`。
- [ ] 实施 `Detect`：


```go
  const markerSection = "agentsync"
  const markerOption  = "managed"

  func Detect(dir string) (State, error) {
      repo, err := gogit.PlainOpenWithOptions(dir, &gogit.PlainOpenOptions{DetectDotGit: true})
      if errors.Is(err, gogit.ErrRepositoryNotExists) {
          return StateUntracked, nil
      }
      if err != nil {
          return StateUntracked, fmt.Errorf("opening git repo at %s: %w", dir, err)
      }
      cfg, err := repo.Config()
      if err != nil {
          return StateForeign, fmt.Errorf("reading git config at %s: %w", dir, err)
      }
      if cfg.Raw.Section(markerSection).Option(markerOption) == "true" {
          return StateAgentsyncOwned, nil
      }
      return StateForeign, nil
  }
  ```


- [ ] 写入 `git_test.go`。每个 FS 测试体的第一行：
  `testenv.RequireContainer(t)`。案例（自然的表驱动）：
  - 空 `t.TempDir()` → `StateUntracked`。
  - `gogit.PlainInit(dir, false)`，标记已设置 → `StateAgentsyncOwned`。
  - `gogit.PlainInit(dir, false)` 没有标记 → `StateForeign`。
  - 在 `t.TempDir()` 处初始化一个存储库，创建一个子目录，`Detect(child)` →
    `StateForeign`（证明`DetectDotGit`）。

**测试命令+预期**


```
AGENTSYNC_TEST_IN_CONTAINER=1 go test ./internal/git/ -run TestDetect -count=1
# ok  github.com/spxrogers/agentsync/internal/git
```



**提交：** `feat(git): add internal/git package with work-tree state detection`

## 任务 0.2 — `Init`：创建一个 agentsync 拥有的存储库（标记、通知、权限）

**文件**
- 创建`internal/git/init.go`
- 修改`internal/git/git_test.go`（添加`TestInit`）

**界面**


```go
// Init creates a new agentsync-owned git repo at dir: PlainInit, stamp the
// [agentsync] managed=true marker into .git/config, write the local-only notice
// file, tighten .git to 0o700. Idempotent-safe: errors if dir already holds a repo
// (callers gate on Detect == StateUntracked first).
func Init(dir string) (*Repo, error)

// Repo wraps a go-git repository + its absolute work-tree root.
type Repo struct {
    dir  string
    repo *gogit.Repository
}

// Open opens an existing agentsync-owned repo (no marker check here; callers use
// Detect). Returns the wrapper used by Commit/Restore/Log.
func Open(dir string) (*Repo, error)

const NoticeFile = "AGENTSYNC_LOCAL_HISTORY.md"
```



**步骤**
- [ ] `Init`：


```go
  func Init(dir string) (*Repo, error) {
      repo, err := gogit.PlainInit(dir, false)
      if err != nil {
          return nil, fmt.Errorf("git init %s: %w", dir, err)
      }
      // Marker — distinguishes our repo from a user's own (Detect reads it back).
      cfg, err := repo.Config()
      if err != nil {
          return nil, fmt.Errorf("read fresh git config: %w", err)
      }
      cfg.Raw.Section(markerSection).SetOption(markerOption, "true")
      if err := repo.SetConfig(cfg); err != nil {
          return nil, fmt.Errorf("write marker to git config: %w", err)
      }
      // Privacy: history may carry cleartext secrets; keep .git user-only.
      if err := os.Chmod(filepath.Join(dir, ".git"), 0o700); err != nil {
          return nil, fmt.Errorf("chmod .git: %w", err)
      }
      if err := os.WriteFile(filepath.Join(dir, NoticeFile), []byte(noticeBody), 0o600); err != nil {
          return nil, fmt.Errorf("write %s: %w", NoticeFile, err)
      }
      return &Repo{dir: dir, repo: repo}, nil
  }
  ```


- [ ] 定义 `noticeBody` (a `const string`) 覆盖：仅本地代理同步
  回滚历史记录；可能包含明文秘密；不得推动；回滚
  与 `agentsync revert <agent>`。
- [ ] `Open`: `gogit.PlainOpen(dir)` → 换行；换行 `gogit.ErrRepositoryNotExists`
  在一个明显的错误中。
- [ ] `TestInit` （容器）：在 `Init` 之后，断言 `.git` 存在，`Detect` →
  `StateAgentsyncOwned`、`NoticeFile` 存在于模式 `0o600` 中，并且（尽力而为，
  在 Windows 上跳过）`.git` 模式为 `0o700`。

**测试：** `AGENTSYNC_TEST_IN_CONTAINER=1 go test ./internal/git/ -run TestInit -count=1` → `ok`

**提交：** `feat(git): Init creates agentsync-owned repo with marker + local-only notice`

## 任务 0.3 — `Commit`：暂存特定文件 + 使用专用签名提交

**文件**
- 创建`internal/git/commit.go`
- 修改`internal/git/git_test.go`（添加`TestCommit`）

**界面**


```go
// Commit stages exactly relPaths (slash-relative to the repo root) and records one
// commit authored by id. Returns the new commit hash. relPaths may include
// deletions (a path that no longer exists on disk is staged as a removal). Returns
// (zeroHash, nil) and commits nothing when there is nothing to stage.
func (r *Repo) Commit(relPaths []string, message string, id Identity) (string, error)
```



**步骤**
- [ ] 实施：


```go
  func (r *Repo) Commit(relPaths []string, message string, id Identity) (string, error) {
      wt, err := r.repo.Worktree()
      if err != nil {
          return "", fmt.Errorf("worktree: %w", err)
      }
      staged := 0
      for _, rel := range relPaths {
          // wt.Add stages both modifications and deletions for an existing index
          // entry; for a brand-new file it stages the addition.
          if _, err := wt.Add(rel); err != nil {
              return "", fmt.Errorf("git add %s: %w", rel, err)
          }
          staged++
      }
      if staged == 0 {
          return "", nil
      }
      if id.Name == "" { id = DefaultIdentity }
      sig := &object.Signature{Name: id.Name, Email: id.Email, When: time.Now().UTC()}
      h, err := wt.Commit(message, &gogit.CommitOptions{Author: sig, Committer: sig})
      if err != nil {
          return "", fmt.Errorf("git commit: %w", err)
      }
      return h.String(), nil
  }
  ```


- [ ] `TestCommit` (容器): `Init` 一个目录，写入 `a.txt`, `Commit(["a.txt"],
  "first", DefaultIdentity)` → 非空哈希； `Log`（下一个任务，或通过读取
  在此测试中直接使用 go-git）显示作者 `agentsync <agentsync@localhost>`。
  第二种情况：覆盖`a.txt`，再次提交→新的不同哈希，其父级是
  第一个。第三：空`relPaths`→返回`("", nil)`，没有新的提交。

**测试：** `AGENTSYNC_TEST_IN_CONTAINER=1 go test ./internal/git/ -run TestCommit -count=1` → `ok`

**提交：** `feat(git): Commit stages named files and records the agentsync checkpoint`

## 任务 0.4 — `Log`：列出检查点

**文件**
- 创建`internal/git/log.go`
- 修改`internal/git/git_test.go`（添加`TestLog`）

**界面**


```go
// Checkpoint is one commit in a destination repo's history.
type Checkpoint struct {
    Hash    string    // full hash
    Short   string    // first 7 chars
    Subject string    // first line of the message
    When    time.Time
}

// Log returns up to n checkpoints, newest first (n<=0 → all).
func (r *Repo) Log(n int) ([]Checkpoint, error)

// Resolve turns a revision (e.g. "HEAD", "HEAD~1", a short/long hash) into a full
// hash, erroring clearly if it doesn't resolve.
func (r *Repo) Resolve(rev string) (string, error)
```



**步骤**
- [ ] `Log`: `r.repo.Log(&gogit.LogOptions{})` → 迭代 `iter.ForEach`, 构建
  `Checkpoint` 秒，停在 `n`。主题 = `strings.SplitN(c.Message, "\n", 2)[0]`。
- [ ] `Resolve`: `r.repo.ResolveRevision(plumbing.Revision(rev))`;包裹一个
  在 `fmt.Errorf("no such checkpoint %q in %s: %w", rev, r.dir, err)` 中未找到。
- [ ] `TestLog` （容器）：三个提交 → `Log(0)` 返回 3 个最新优先；
  `Log(2)` 返回 2； `Resolve("HEAD~1")` 等于第二个最新的哈希值；
  `Resolve("deadbeef")` 错误。

**测试：** `AGENTSYNC_TEST_IN_CONTAINER=1 go test ./internal/git/ -run TestLog -count=1` → `ok`

**提交：** `feat(git): Log + Resolve for browsing destination checkpoints`

## 任务 0.5 — `Restore`：仅附加回滚到检查点

**文件**
- 创建`internal/git/restore.go`
- 修改`internal/git/git_test.go`（添加`TestRestore`）

**界面**


```go
// FileChange describes one file a restore would touch (for --dry-run preview).
type FileChange struct {
    Path string
    Kind string // "modify" | "create" | "delete"
}

// Plan computes what Restore(target) would change vs the current worktree HEAD,
// WITHOUT writing anything.
func (r *Repo) Plan(targetRev string) (targetHash string, changes []FileChange, err error)

// Restore makes the worktree match the targetRev checkpoint and records the result
// as a NEW commit on top of HEAD (append-only — HEAD advances, nothing is
// rewritten or lost). Returns the new commit hash. Commits nothing and returns
// (zeroHash, nil) when the worktree already matches target.
func (r *Repo) Restore(targetRev, message string, id Identity) (string, error)
```



**步骤**
- [ ] 助手 `treeOf(rev)`：`Resolve` → `r.repo.CommitObject(hash)` → `c.Tree()`。
- [ ] `Plan`：比较 HEAD 树与目标树：


```go
  headTree, _ := r.headTree()      // CommitObject(HEAD).Tree()
  tgtHash, _ := r.Resolve(targetRev)
  tgtTree, _ := r.treeOfHash(tgtHash)
  changes, err := headTree.Diff(tgtTree)   // object.Changes: HEAD -> target
  // For each *object.Change: action, _ := ch.Action()
  //   merkletrie.Insert -> file exists in target, not HEAD  => Kind "create"
  //   merkletrie.Delete -> file exists in HEAD,  not target => Kind "delete"
  //   merkletrie.Modify -> Kind "modify"
  ```


  构建 `[]FileChange`（使用 `ch.To.Name` 进行创建/修改，使用 `ch.From.Name` 进行
  删除）。
- [ ] `Restore`：调用`Plan`；如果没有更改，则返回 `("", nil)`。否则应用每个
  更改为 `r.dir` 下真实 FS 上的工作树：
  - 创建/修改 → 读取目标 blob (`tgtTree.File(name)` → `f.Contents()`),
    `os.MkdirAll(parent, 0o700)`、`os.WriteFile(abs, []byte(content), 0o600)`。
  - 删除 → `os.Remove(abs)`。
  收集触摸的相对路径，然后 `r.Commit(touched, message, id)` 进行记录
  仅附加检查点。
- [ ] `TestRestore`（容器）：提交 `a.txt` 的 `v1`；提交`v2`；添加`b.txt`
  在第三次提交中。 `Restore("HEAD~2", "revert", id)`：
  - `a.txt` 字节 == `v1`，`b.txt` 消失了。
  - 存在一个新的提交，其父级是先前的 HEAD（通过 `Log` 长度 + 断言
    父级），证明仅附加（HEAD~2 提交在 `Log` 中仍然可达）。
  - `Plan` 在已经匹配的目标上不返回任何更改； `Restore` 返回
    `("", nil)`。

**测试：** `AGENTSYNC_TEST_IN_CONTAINER=1 go test ./internal/git/ -run TestRestore -count=1` → `ok`

**提交：** `feat(git): append-only Restore to roll a worktree back to a checkpoint`

## 任务 0.6 — 永不推守测试

**文件**
- 创建`internal/git/nopush_test.go`

**步骤**
- [ ] 如果包增长为远程/推送，反射/源防护就会失败
  表面。最简单的健壮形式：静态扫描包自己的 `.go` 源
  （非测试）对于标记 `Push`、`CreateRemote`、`remote(`、`Remote(`：


```go
  func TestNoPushSurface(t *testing.T) {
      files, _ := filepath.Glob("*.go")
      for _, f := range files {
          if strings.HasSuffix(f, "_test.go") { continue }
          b, err := os.ReadFile(f); if err != nil { t.Fatal(err) }
          for _, bad := range []string{"Push", "CreateRemote", ".Remote(", "Remotes("} {
              if bytes.Contains(b, []byte(bad)) {
                  t.Errorf("%s references %q — internal/git must never push or add a remote (issue #118)", f, bad)
              }
          }
      }
  }
  ```


- [ ] 在包文档注释中记录规则，以便可以发现其意图。

**测试：** `AGENTSYNC_TEST_IN_CONTAINER=1 go test ./internal/git/ -count=1` → 全绿色。

**提交：** `test(git): guard that internal/git exposes no remote/push surface`

---

# 里程碑 1 — 规范配置 + `VersionedHome`

## 任务 1.1 — 规范模式中的 `DestinationGitBackupConfig`

**文件**
- 修改`internal/source/schema.go`
- 创建/扩展 `internal/source/schema_test.go` （或现有的加载器测试）
  一个解析案例。

**步骤**
- [ ] 添加到 `Config`（在 `Memory` 之后）：


```go
  DestinationGitBackup DestinationGitBackupConfig `toml:"destination_directory_git_backup"`
  ```


- [ ] 添加结构+模式常量：


```go
  // DestinationGitBackupConfig mirrors [destination_directory_git_backup] in
  // agentsync.toml. It controls whether `apply` git-versions the rendered
  // destination dirs (a local-only rollback history; never pushed — see issue
  // #118 and the secret-handling note in CLAUDE.md). Empty Mode == ModePrompt.
  type DestinationGitBackupConfig struct {
      Mode        string `toml:"mode,omitempty"`         // "prompt" | "on" | "off"
      AuthorName  string `toml:"author_name,omitempty"`
      AuthorEmail string `toml:"author_email,omitempty"`
  }

  const (
      GitBackupModePrompt = "prompt" // default: ask on first untracked write
      GitBackupModeOn     = "on"     // init + checkpoint silently
      GitBackupModeOff    = "off"    // never init/commit/prompt
  )

  // EffectiveMode returns Mode or GitBackupModePrompt when unset.
  func (g DestinationGitBackupConfig) EffectiveMode() string {
      if g.Mode == "" { return GitBackupModePrompt }
      return g.Mode
  }
  ```


- [ ] 测试：使用以下命令解组夹具
  `[destination_directory_git_backup]\nmode = "on"\nauthor_name = "x"` 并断言
  田野；断言不存在的表会产生 `EffectiveMode() == "prompt"`。

**测试：** `AGENTSYNC_TEST_IN_CONTAINER=1 go test ./internal/source/ -run GitBackup -count=1` → `ok`

**提交：** `feat(source): add [destination_directory_git_backup] to the canonical schema`

## 任务 1.2 — 新表的项目叠加合并

**文件**
- 修改`internal/project/project.go`（`Merge`函数——看看如何
  `UpdateDefaults`/`MemoryConfig` 今天合并）
- 修改项目合并测试。

**步骤**
- [ ] 在 `project.Merge` 中，在现有的 `Updates`/`Memory` 合并后，覆盖
  基 `DestinationGitBackup` 与项目的非零字段（项目获胜时
  设置；否则继承基址），匹配同级表使用的优先级。
  （从功能上讲，这是一个用户范围的问题，但保持合并对称
  避免了令人惊讶的“项目配置被默默忽略”的差距。）
- [ ] 测试：基本模式 `"on"`、项目模式 `""` → 合并 `"on"`；项目模式
  `"off"` → 合并 `"off"`。

**测试：** `AGENTSYNC_TEST_IN_CONTAINER=1 go test ./internal/project/ -count=1` → `ok`

**提交：** `feat(project): merge [destination_directory_git_backup] in the project overlay`

## 任务 1.3 — `VersionedHome` 可选适配器接口 + 实现

**文件**
- 修改`internal/adapter/adapter.go`（声明接口）
- 修改每个深度适配器：`internal/adapter/{claude,opencode,codex,cursor,gemini,continuedev,windsurf,roo,cline}/*.go`
- 修改`internal/adapter/generic/*.go`
- 创建`internal/adapter/versionedhome_test.go`（注册表范围的行为防护）

**界面（在 `adapter.go` 中，靠近 `PluginIngester`/`WarnEmitter`）**


```go
// VersionedHome is an OPTIONAL Adapter extension. An adapter that writes into a
// single coherent on-disk config directory declares it so the apply tail can
// git-init and checkpoint that directory (issue #118). Adapters with no single
// versionable root return ("", false).
//
// Read-only and user-scope-focused: HomeDir reports the dir to back up; it does
// NOT widen the render/Apply contract. At ScopeProject it SHOULD return ("",false)
// — project destinations live inside the user's own project repo and are left to
// that repo's source control.
type VersionedHome interface {
    HomeDir(scope Scope, project string) (string, bool)
}
```



**步骤**
- [ ] 使用上面的文档注释将接口添加到 `adapter.go`。
- [ ] 对于每个深度适配器，添加一个返回其用户范围 `ConfigDir` 的方法。
  每个适配器已存储其目标根 (`a.opts.TargetRoot`) 并具有
  `ResolvePaths`。示例（光标）：


```go
  func (a *Adapter) HomeDir(scope adapter.Scope, project string) (string, bool) {
      if scope != adapter.ScopeUser {
          return "", false
      }
      cd := ResolvePaths(a.opts.TargetRoot, "", false).ConfigDir
      if cd == "" {
          return "", false
      }
      return cd, true
  }
  ```


  使用适配器自己的路径访问器对每个适配器进行镜像 + `ConfigDir`/root
  字段名称。 **Claude:** 返回 `~/.claude` （其 `ConfigDir`），而不是 `$HOME` —
  有意排除杂散`~/.claude.json`（决定#10）。
- [ ] **通用适配器：** 通过读取目录来实现 `HomeDir`
  `generic.Spec`（规范已经对其使用的代理配置目录/根进行了编码
  渲染路径）。仅当规范确实没有单根时才返回 ("", false)。
- [ ] 防护测试 `versionedhome_test.go`：构建 `registryFactory()`，迭代
  `reg.Names()`，断言**每个**注册的适配器都实现`VersionedHome`并且
  返回位于测试目标下的 `ScopeUser` 处的非空绝对目录
  root，AND 在 `ScopeProject` 处返回 `("", false)`。 （这类似于
  `TestEveryAdapterClassifiesSkips` — 它将功能固定在代码中，因此一个新的
  适配器不能默默地跳过 git-backup 支持。）通过使用 `AGENTSYNC_TARGET_ROOT`
  现有的测试环境助手，以便目录在临时根目录下解析。

**测试：** `AGENTSYNC_TEST_IN_CONTAINER=1 go test ./internal/adapter/... -run VersionedHome -count=1` → `ok`

**提交：** `feat(adapter): VersionedHome extension exposing each agent's versionable dir`

## 任务 1.4 — 将 `mode` 保留回 `agentsync.toml`（保留注释）

**文件**
- 创建`internal/cli/gitbackup_config.go`
- 创建`internal/cli/gitbackup_config_test.go`

**界面**


```go
// setDestinationGitBackupMode writes mode into the [destination_directory_git_backup]
// table of ~/.agentsync/agentsync.toml, preserving everything outside that table
// (comments, key order, other sections) via a line-splice — never a full marshal.
// Creates the table if absent.
func setDestinationGitBackupMode(home, mode string) error
```



**步骤**
- [ ] 使用与 `writeAgents` 相同的线路拼接策略来实现
  (`agent.go:185`)：读取原始数据，删除现有的
  `[destination_directory_git_backup]` 部分的行，拼接在重新生成的
  块（`mode = "<mode>"`加上任何保留的`author_name`/`author_email`），写入
  通过 `iox.AtomicWrite(p, …, 0o644)`。仅重新发出 `mode` 行以及作者
  已经存在的行（在删除之前解析它们），所以编辑
  不会静默删除用户的身份覆盖。
- [ ] 测试（容器）：从带有注释和注释的装置 `agentsync.toml` 开始
  `[updates]` 表； `setDestinationGitBackupMode(home, "on")` 然后通过重新加载
  `source.Load` → 模式 `"on"`、`[updates]` 完好无损，前导注释完好无损。运行它
  两次 → 没有重复的表，间距稳定（镜像 `writeAgents` 幂等性
  断言）。

**测试：** `AGENTSYNC_TEST_IN_CONTAINER=1 go test ./internal/cli/ -run GitBackupConfig -count=1` → `ok`

**提交：** `feat(cli): persist destination-git-backup mode without clobbering agentsync.toml`

---

# 里程碑 2 — apply-tail 检查点挂钩

## 任务 2.1 — `apply --no-git-backup` 标志

**文件**
- 修改`internal/cli/apply.go`

**步骤**
- [ ] 在`newApplyCmd`中添加`var noGitBackup bool`；注册
  `cmd.Flags().BoolVar(&noGitBackup, "no-git-backup", false, "skip destination git
  versioning/checkpoint for this run (CI/scripting); does not modify
  agentsync.toml")`。
- [ ] 将其插入 `applyRun` （添加 `noGitBackup bool` 参数；同时传递
  调用站点位于第 42 和 45 行）。
- [ ] 除了管道之外还没有任何行为；仅构建。

**测试：** `go build ./... && AGENTSYNC_TEST_IN_CONTAINER=1 go test ./internal/cli/ -run Apply -count=1` → `ok`（现有应用测试仍然通过）。

**提交：** `feat(cli): add apply --no-git-backup flag (plumbing)`

## 任务 2.2 — 检查点编排（组 `written` → 每个代理 → 初始化/提交）

**文件**
- 创建`internal/cli/gitbackup.go`
- 创建`internal/cli/gitbackup_test.go`

**界面**


```go
// runDestinationGitBackup checkpoints each user-scope agent dir that got managed
// writes this apply. It is a no-op (returns nil) for: project scope, --no-git-backup,
// mode "off", an empty `written` set, or an agent whose dir is foreign-tracked.
// Best-effort by contract: a git failure for one agent is reported to p.Err and
// does NOT fail the apply (the files are already written + state saved).
func runDestinationGitBackup(
    cmd *cobra.Command, p *ui.Printer, reg *adapter.Registry, agents []string,
    sc adapter.Scope, projectRoot, home, userHome string,
    cfg source.DestinationGitBackupConfig, written map[string]bool, noGitBackup bool,
) error
```



**步骤**
- [ ] 提前返回：`sc != adapter.ScopeUser`、`noGitBackup` 或
  `cfg.EffectiveMode() == GitBackupModeOff` 或 `len(written) == 0` → 返回 nil。
- [ ] 构建专用身份：`id := agitgit.Identity{Name: cfg.AuthorName,
  Email: cfg.AuthorEmail}`（空字段回退到 `DefaultIdentity` 内
  `Commit`）。 （给包起别名，例如 `import agit "…/internal/git"`，以避免
  如果两者都被导入的话，会与 go-git 发生冲突。）
- [ ] 对于 `agents` 中每个启用的代理名称：
  - `ad := reg.Lookup(name)`; `vh, ok := ad.(adapter.VersionedHome)`；如果 `!ok`
    继续。
  - `dir, ok := vh.HomeDir(sc, projectRoot)`;如果`!ok`继续。
  - 在`dir`下选择该代理的书面文件：

```go
    var rels []string
    for abs := range written {
        if underDir(dir, abs) {            // filepath.Rel + no ".." prefix
            rels = append(rels, relSlash(dir, abs))
        }
    }
    if len(rels) == 0 { continue }         // nothing managed under this dir changed
    sort.Strings(rels)
    ```


  - `st, err := agit.Detect(dir)`;出现错误 → 警告 `p.Err`，继续。
  - `switch st`：
    - `StateForeign` → 可选的一行提示（每个目录仅一次），继续（决定＃5）。
    - `StateUntracked` → 请参阅`mode`：
      - `GitBackupModeOn` → `repo, err := agit.Init(dir)`。
      - `GitBackupModePrompt` → 调用 `promptInitGitBackup(cmd, p, dir)`（任务 2.3）。
        关于“是”→ `Init` + 坚持 `setDestinationGitBackupMode(home, "on")`。开
        “不”→继续。关于“不，不要再问”→坚持
        `setDestinationGitBackupMode(home, "off")` 并继续。无 TTY/无输入时
        → 打印一行提示并继续（不能询问）。
    - `StateAgentsyncOwned` → `repo, err := agit.Open(dir)`。
  - 手头有`repo`：如果通知文件是新的，也暂存通知文件（Init 已经写入
    它；在第一次提交时将 `agit.NoticeFile` 包含在 `rels` 中，以便对其进行跟踪）。
  - `msg := fmt.Sprintf("agentsync apply: %s (%s) — %d file(s)\n\n%s", name, sc, len(rels), strings.Join(rels, "\n"))`。
  - `h, err := repo.Commit(rels, msg, id)`;出现错误→警告，继续。论成功
    非空 `h` → 可选的微弱确认行到 `p.Err`。
- [ ] 助手 `underDir(dir, abs) bool` 和 `relSlash(dir, abs) string`
  （`filepath.Rel` + `filepath.ToSlash`，拒绝 `..`）。
- [ ] `gitbackup_test.go`（容器，真正的FS）：直接驱动编排
  （不是通过完全应用）使用伪造的注册表，其适配器的 `HomeDir` 返回一个
  临时目录及其下的 `written` 文件映射。断言：
  - 模式 `"on"`，未跟踪的目录 → 恰好一次提交，包含写入的文件 +
    请注意，不要将不相关的文件放置在该目录中。
  - 空 `written` → 未创建存储库。
  - 外部目录（`PlainInit` 之前，无标记）→ 未添加代理同步提交。
  - `--no-git-backup` / 模式 `"off"` → 无操作。
  - 项目范围→无操作。

**测试：** `AGENTSYNC_TEST_IN_CONTAINER=1 go test ./internal/cli/ -run GitBackup -count=1` → `ok`

**提交：** `feat(cli): checkpoint each user-scope agent dir after a writing apply`

## 任务 2.3 — 选择退出初始化提示

**文件**
- 修改`internal/cli/gitbackup.go`
- 修改`internal/cli/gitbackup_test.go`

**界面**


```go
type promptResult int
const (
    promptYes promptResult = iota
    promptNo
    promptNever      // "no, and don't ask again"
    promptUnavailable // no TTY / --no-input — caller skips silently with a hint
)

func promptInitGitBackup(cmd *cobra.Command, p *ui.Printer, dir string) promptResult
```



**步骤**
- [ ] 如果 `noInputFlag(cmd) || !stdinIsTerminal(cmd)` → `promptUnavailable`。
- [ ] 否则使用 `bufio.NewReader(cmd.InOrStdin())` 习惯用法进行提示
  `promptScopeChoice`（apply.go：504）。提供`[y] yes  [n] not now  [d] don't ask
  again`，无效输入重新提示最多5次； EOF → `promptNo`。使
  明确复制这会创建一个 **仅限本地、从未推送** 历史记录，可能会
  包含秘密。
- [ ] 测试： feed `cmd.SetIn(strings.NewReader("y\n"))` 等 - 但请注意这些运行
  通过 `stdinIsTerminal` 上的编排；用于单元测试
  提示解析，使用注入的阅读器和强制的测试 `promptInitGitBackup`
  TTY shim，或者（更简单）通过以下方式测试三个持续的结果
  通过存根终端检查取消设置 `--no-input` 的编排。保持在
  最少：“y”→ 存储库创建 + 模式持续 `"on"`； "d" → 无存储库 + 模式 `"off"`;
  no-TTY → 无存储库，模式不变。

**测试：** `AGENTSYNC_TEST_IN_CONTAINER=1 go test ./internal/cli/ -run GitBackupPrompt -count=1` → `ok`

**提交：** `feat(cli): opt-out prompt to git-init a destination dir on first apply`

## 任务 2.4 — 将编排连接到应用尾部

**文件**
- 修改`internal/cli/apply.go`

**步骤**
- [ ] 在 `_ = render.PruneBackups(home, render.DefaultBackupKeep)` 之后（第 222 行）
  在成功打印之前（第~224行），插入：


```go
  // Destination git backup (issue #118): checkpoint the user-scope agent dirs we
  // just wrote, so a bad apply is a `git revert` / `agentsync revert` away. Local-
  // only, never pushed. Best-effort — never fails an otherwise-successful apply.
  if err := runDestinationGitBackup(cmd, p, reg, agents, sc, projectRoot, home, userHome,
      c.Config.DestinationGitBackup, written, noGitBackup); err != nil {
      fmt.Fprintf(p.Err, "%s destination git backup: %v\n", p.Yellow("agentsync:"), err)
  }
  ```


  (`c` 是范围内加载的规范；`c.Config.DestinationGitBackup` 携带
  合并的配置。）
- [ ] `internal/cli` 中的完整集成测试（真实 FS，容器）：种子最小化
  规范地使用一个启用的代理 `mode = "on"`，运行真正的 `apply` 命令
  端到端，断言代理目录现在是 agentsync 拥有的存储库，其中包含一个
  检查站；再次运行 `apply` 且不更改源 → 无需第二次提交（
  “应用未更改任何内容”路径使 `written` 为空/不变）。使用现有的
  应用集成测试工具/夹具作为模板。

**测试：** `AGENTSYNC_TEST_IN_CONTAINER=1 go test ./internal/cli/ -run Apply -count=1` → `ok`

**提交：** `feat(cli): run destination git backup in the apply tail`

---

# 里程碑 3 — `agentsync revert`

## 任务 3.1 — 命令框架 + 标志

**文件**
- 创建`internal/cli/revert.go`
- 修改`internal/cli/root.go`（寄存器`newRevertCmd()`）

**步骤**
- [ ] `newVerifyCmd`/`newDoctorCmd` 上的模型。表面：


```
  agentsync revert <agent> [--to <ref>] [--all] [--dry-run]
  ```

标志：`--to`（字符串）、`--all`（布尔）、`--dry-run`（布尔）。 `Args:
  cobra.MaximumNArgs(1)`。拒绝 `--all` 和位置 `<agent>`
  (`return fmt.Errorf("--all reverts every managed dir; do not also pass an agent")`)。
  恰好需要{位置代理，`--all`}之一。
- [ ] 将 `newRevertCmd()` 添加到 `root.go:42` 中的 `cmd.AddCommand(...)` 块。
- [ ] 连接 `home`、`reg := registryFactory()`、`userHome`，并解析目标目录
  通过 `VersionedHome.HomeDir(adapter.ScopeUser, "")` 指定代理（或所有
  `--all` 的注册代理）。

**测试：** `go build ./... && agentsync revert --help` 显示表面；一个未知的
代理名称错误。添加 `internal/cli` 测试以进行 arg 验证。

**提交：** `feat(cli): scaffold agentsync revert command`

## 任务 3.2 — 恢复行为（解析目标、恢复、不同步通知）

**文件**
- 修改`internal/cli/revert.go`
- 创建`internal/cli/revert_test.go`

**步骤**
- [ ] 对于目标代理：`dir, ok := vh.HomeDir(...)`； `st, _ := agit.Detect(dir)`。
  - `st != StateAgentsyncOwned` → 清除错误：不是 agentsync 管理的存储库；
    建议首先启用备份+运行`apply`。
- [ ] `repo, _ := agit.Open(dir)`。解决目标转速：
  - `--to` 设置 → 该版本。
  - 默认 → `"HEAD~1"`（撤消最近的应用 — 决定 #17）。如果回购
    只有一次提交，错误：没有更早的内容可以恢复。
- [ ] `--dry-run` → `repo.Plan(target)` → 打印目标短哈希+主题（通过
  `Log`）和`[]FileChange`；退出而不写入。
- [ ] 否则 `repo.Restore(target, fmt.Sprintf("agentsync revert: %s → %s", name, short), id)`。
  在无操作（`("", nil)`）上告诉用户目录已经与该检查点匹配。
- [ ] 始终在实际恢复时打印不同步通知（决定#4）：
  > `agentsync revert` 已完成。目标目录 `<dir>` 现已退出
  > 与agentsync 配置同步。请根据需要进行协调（例如
  > `agentsync reconcile` / `agentsync import`) 在下一个 `agentsync apply` 之前
  > 避免重新丢失这些更改。
- [ ] 测试（容器）：构建一个具有 2 个检查点的存储库（通过以下方式模拟两个应用）
  直接写入+`Commit`，或运行 apply 两次）。 `revert <agent>`：
  - 文件与较早的检查点匹配；存在新提交（仅追加）；
  - 标准输出携带不同步通知。
  - `--to <hash>` 针对特定检查点。
  - `--dry-run` 不写入任何内容（工作树不变）并列出更改。
  - 未初始化的目录/未知 `--to` → 清除错误，无写入。

**测试：** `AGENTSYNC_TEST_IN_CONTAINER=1 go test ./internal/cli/ -run Revert -count=1` → `ok`

**提交：** `feat(cli): agentsync revert restores a destination dir to a checkpoint`

## 任务 3.3 — `--all`

**文件**
- 修改`internal/cli/revert.go`
- 修改`internal/cli/revert_test.go`

**步骤**
- [ ] `--all`：迭代每个注册代理；对于每个 `StateAgentsyncOwned` 目录，
  运行相同的default-`HEAD~1`恢复；跳过（带有微弱的注释）目录
  未追踪/外国或只有一个检查点。汇总总结；打印
  一次不同步通知，列出每个恢复的目录。 `--to` 被拒绝
  `--all`（引用是针对每个存储库的，并且在不同的目录中没有意义）- 明显错误。
- [ ] `--dry-run --all` 预览每个。
- [ ] 测试：两个托管代理目录 + 一个外部 → `--all` 恢复这两个目录，跳过
  外国的，通知列出了两者。

**测试：** `AGENTSYNC_TEST_IN_CONTAINER=1 go test ./internal/cli/ -run RevertAll -count=1` → `ok`

**提交：** `feat(cli): agentsync revert --all across every managed destination dir`

---

# 里程碑 4 — 医生身份 + 文档（相同 PR — `CLAUDE.md` 文档同步规则）

## 任务 4.1 — `agentsync doctor`“目标 git 备份”部分

**文件**
- 修改`internal/cli/doctor.go`
- 修改医生测试（现有的`internal/cli`医生测试引脚子字符串）。

**步骤**
- [ ] 在“插件”部分后添加：


```go
  fmt.Fprintln(p.Out, "")
  p.Section("Destination git backup")
  checkDestinationGitBackup(p, home)
  ```


- [ ] `checkDestinationGitBackup(p, home)`：加载配置（`source.Load`）；打印
  有效模式（`okCheck`/`warnCheck`，标签为`mode       `。然后对于每个
  使用磁盘上存在的 `VersionedHome` 目录注册代理，打印一行：
  `agentsync-versioned` (✓)、`foreign source control` (ℹ) 或 `untracked` (ℹ)、
  使用`agit.Detect`。仅供参考 — 从不增加 `fails`。
- [ ] 更新医生测试的预期子字符串（它断言连续
  `<label><status>`）；添加一个断言新节标题+模式行的案例。

**测试：** `AGENTSYNC_TEST_IN_CONTAINER=1 go test ./internal/cli/ -run Doctor -count=1` → `ok`

**提交：** `feat(cli): doctor surfaces destination-git-backup status (read-only)`

## 任务 4.2 — 文档 + 变更日志 + 脚手架

**文件**
- 修改 `docs/user-guide.md` — 命令参考：添加 `revert`、`apply
  --no-git-backup` 标志、新的 `doctor` 行、
  `agentsync.toml` 布局块中的 `[destination_directory_git_backup]` 表，
  以及一个简短的“目标版本控制”概念简介。
- 修改`README.md`——快速引用表：添加`revert`；一行注释
  仅本地自动版本控制。
- 修改 `website/src/content/docs/reference/cli.mdx` —“概览”表格行 +
  `### revert` 部分 + `apply --no-git-backup` 标志。
- 修改 `docs/concepts.md` — 注意目标目录可能携带仅限本地的 git
  历史，与 `.state/` 不同。
- 修改 `docs/architecture.md` — 检查点步骤位于应用尾部的位置；
  永不推的不变量； `.state/` 未受影响。
- 修改 `internal/cli/init.go` — 添加注释
  `[destination_directory_git_backup]` 块到 `initialAgentsyncTOML`（镜像
  评论 `[secrets]` 块样式）。
- 修改 `CHANGELOG.md` — 在 `[Unreleased] / Added` 下：
  - `agentsync revert` — 将目标代理目录回滚到之前的应用检查点。
  - 目标目录使用仅限本地的 git 存储库进行自动版本控制（选择退出；
    `[destination_directory_git_backup]`）； `apply --no-git-backup` 绕过它。

**步骤**
- [ ] 进行每次编辑；保持矩阵/合约页面不变（这不是
  每个代理组件的能力 - 能力矩阵不会改变）。
- [ ] 验证网站文档构建是否仍然通过（如果存储库构建了它）
  (`website/` — 仅上面创作的页面；生成的合同页面是
  未触及）。

**测试：** `go build ./... && AGENTSYNC_TEST_IN_CONTAINER=1 just test-fast`;那么
容器绒毛：
`GOTOOLCHAIN=go1.26.2 go run github.com/golangci/golangci-lint/v2/cmd/golangci-lint@v2.12.2 run ./...`
→ 干净。重新暂存 `just lint` 重写的所有文件。

**提交：** `docs: document destination git backup + agentsync revert`

---

## 规范覆盖图（每个要求 → 任务）

|规格要求|任务 |
|---|---|
|每个目标存储库，用户范围 | 1.3、2.2 |
|选择退出提示 + 粘性“不再询问”| 2.3、1.4 |
| `mode = prompt/on/off` 配置表 | 1.1, 1.2 |
| `--no-git-backup` CI 绕过 | 2.1、2.2 |
|承诺每项书面申请；同步时无操作 | 2.2、2.4 |
|整个文件提交范围；没有不相关的文件 | 2.2 | 2.2
|排除杂散 `$HOME` 文件（间隙）| 1.3 (克劳德 HomeDir=`~/.claude`) |
|尊重现有的源代码控制| 0.1（检测）、2.2 |
| `VersionedHome` git-root 机制 | 1.3 | 1.3
|专用提交身份（可覆盖）| 0.3、1.1、2.2 |
|自动创建标记+通知文件| 0.2 | 0.2
| `.git` 0o700 / 文件 0o600 | 0.2 | 0.2
|从不推送（无远程 API）| M0（无表面），0.6防护|
| `agentsync revert <agent>` 默认最后 | 3.1、3.2 |
|仅追加恢复 | 0.5, 3.2 |
| `--to`、`--all`、`--dry-run` | 3.1、3.2、3.3 |
|不同步通知 | 3.2, 3.3 |
| `doctor` 状态（无 `git` 命令）| 4.1 |
|同一 PR 中的文档 | 4.2 |
| `.state/` 未受影响 | （没有任务修改它——通过缺席验证）|

## 实施者的风险/观察项目

- **go-git `wt.Add` 删除：** 确认 v5.18.0 通过以下方式暂存已删除的文件
  `wt.Add(rel)`；如果没有，请使用 `wt.Remove(rel)` 作为 `Commit` 中的删除分支。
  任务 0.3 的测试必须包括删除以捕获此问题。
- **需要真正的 FS：** go-git 不会在 `afero.MemMapFs` 上运行；所有M0/M2/M3
  测试使用 `t.TempDir()` + `testenv.RequireContainer`。
- **通用适配器目录源：**验证 `generic.Spec` 确实公开了单个
  断言 `HomeDir` 之前的配置目录；如果规范缺少，则返回 `("",false)`
  并让登记守卫（1.3）记录弃权而不是发明一个
  路径（根据“交叉引用上游线束文档”规则）。
- **`apply` 集成装置：** 重用现有的 `internal/cli` 应用测试
  驾驭而不是手动滚动规范；匹配其范围/目标根环境
  设置以便新目录在临时根目录下解析。