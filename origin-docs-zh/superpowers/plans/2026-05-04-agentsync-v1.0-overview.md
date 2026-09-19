# agentsync v1.0 — 实施计划概述

> **对于代理工作人员：** 所需的子技能：使用超级能力：子代理驱动开发（推荐）或超级能力：执行计划来逐个任务地实施这些计划。每个里程碑都有自己的计划文件，其中包含复选框 (`- [ ]`) 步骤。

**来源规范：** [`docs/superpowers/specs/2026-05-04-agentsync-design.md`](../specs/2026-05-04-agentsync-design.md)

**目标：** 发布 `agentsync` v1.0 — 一个 Go CLI，可集中管理跨 Claude Code 和 OpenCode 的 AI 编码代理配置，具有市场插件扇出、双向漂移检测、项目本地覆盖和年龄加密秘密。

**技术堆栈：** Go 1.22+、cobra (CLI)、`pelletier/go-toml/v2` (TOML AST)、`tailscale/hujson` (JSONC)、`gofrs/flock`（锁定）、`spf13/afero`（FS 测试假）、`filippo.io/age`（秘密）、`go-git/v5`（市场获取）、`log/slog`（日志记录）、 `golangci-lint`、`goreleaser`。

---

## 里程碑路线图

| ＃|里程碑 |计划文件|船舶 |
|---|---|---|---|
| **M0** |骷髅| [`m0-skeleton.md`](2026-05-04-agentsync-m0-骨骼.md) |模块引导、路径、原子 IO、锁、适配器接口、NoopAdapter、cobra CLI 公开 `init` / `agent` / `doctor` / `verify` / `apply --dry-run`。 CI 绿色。 |
| **M1** |克劳德适配器| [`m1-claude.md`](2026-05-04-agentsync-m1-claude.md) |第一个真正的适配器。所有 7 个组件（MCP、内存、技能、子代理、命令、挂钩、LSP）。按键合并到 `~/.claude/settings.json` 和 `~/.claude.json`。渲染+摄取往返。 |
| **M2** | OpenCode 适配器 | [`m2-opencode.md`](2026-05-04-agentsync-m2-opencode.md) |第二个适配器。 `opencode.json` 的 JSONC。写入共享 `.claude/skills/` 路径的技能。子代理 + 斜杠命令投影（markdown↔markdown w/ frontmatter munging）。挂钩/LSP `✗ skip(warn)`。 |
| **M3** |漂移/状态/差异/协调| [`m3-drift.md`](2026-05-04-agentsync-m3-drift.md) | 3 路分类器、文件 + 键级别、`status` / `diff` 命令、交互式协调循环、批量热键、`--auto-*` 标志、外键报告。 |
| **M4** |市场+插件| [`m4-marketplaces.md`](2026-05-04-agentsync-m4-marketplaces.md) |所有 5 个插件源（relative、github、url、git-subdir、npm）。 go-git fetch、npm tarball fetch、git-subdir 的稀疏克隆。 `strict` 模式。 `${CLAUDE_PLUGIN_ROOT}` 分辨率。每个组件的投影、翻译报告、sha pinning、更新模式。 |
| **M5** |项目本地 | [`m5-project.md`](2026-05-04-agentsync-m5-project.md) | `.agentsync.toml` 直接查看、叠加合并、`--project` 标志、项目范围状态。 |
| **M6** |秘密 | [`m6-secrets.md`](2026-05-04-agentsync-m6-secrets.md) |年龄库集成，`${secret:foo.bar}` 分辨率，`secrets {edit,get,set}`。 |
| **M7** |抛光+发布| [`m7-polish.md`](2026-05-04-agentsync-m7-polish.md) | `explain`、`import`、`agent disable --purge`、goreleaser 跨平台、Homebrew Tap、Scoop 清单、Chocolatey 包、README 文档（年龄密钥备份、首次应用警告、OpenCode 挂钩间隙、光标用户规则限制）。 |

**超越 v1.0：**
- **v1.1** — Codex 适配器（TOML 配置、markdown→TOML 子代理转换、挂钩事件映射）。 v1.0 发布后编写的计划。
- **v1.2** — 游标适配器（JSON `mcp.json`、AGENTS.md、带有事件映射的 hooks.json、项目范围规则）。 v1.1 发布后编写的计划。

## 所有里程碑计划中使用的约定

这些统一适用；每个计划都在这里引用而不是重新陈述。

### TDD 周期

每个引入行为的任务都遵循这个循环：

1. 编写失败的测试
2.运行它；验证它因预期原因而失败（测试发现缺少功能/符号）
3. 编写最小实现
4. 运行测试；验证是否通过
5. 提交（测试+实现一起）

仅引入类型（接口、结构定义、模式）的任务会跳过步骤 1-4，并具有单个“创建文件”步骤 + 提交，因为没有可以单独测试的行为。他们的测试存在于下一个行为任务中。

### 测试框架

- 仅 Stdlib `testing` 包。没有`testify`，没有`gomega`。
- 用于分支逻辑的表驱动测试。每个表行都有一个 `name` 字段；通过 `t.Run(tt.name, ...)` 进行子测试。
- 测试中的文件系统：**始终** `afero.NewMemMapFs()` 或 `t.TempDir()`。从来没有`os.UserHomeDir()`。 Lint 规则 (`forbidigo`) 强制执行此操作。
- `t.Helper()` 在任何调用 `t.Fatal`/`t.Errorf` 的辅助函数中。
- 命令级行为的集成测试位于 `internal/cli/*_test.go` 中，并运用完整的 cobra 命令路径。

### 错误包装



```go
return fmt.Errorf("loading source from %s: %w", path, err)
```



仅标准库。没有 `pkg/errors`。 `errors.Is` 和 `errors.As` 用于匹配。

### 进口订单



```go
import (
    // stdlib
    "fmt"
    "os"

    // third-party
    "github.com/spf13/cobra"

    // internal
    "github.com/spxrogers/agentsync/internal/paths"
)
```



Goimports / gofumpt 格式化。

### 提交消息

具有明确范围的常规提交：

- `feat(paths): resolve AGENTSYNC_HOME with target-root override`
- `test(state): cover atomic write under simulated crash`
- `fix(adapter): preserve foreign keys on settings.json merge`
- `refactor(cli): extract path helper for tests`
- `docs(readme): document age key backup`
- `ci: add forbidigo rule for os.UserHomeDir in tests`
- `chore: bump go.mod to 1.22`

每次提交都以 AI 共同作者页脚结束（根据存储库约定）：



```
Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
```



当消息有多行时使用 HEREDOC 形式（以便换行符正确呈现）：



```bash
git commit -m "$(cat <<'EOF'
feat(paths): resolve AGENTSYNC_HOME with target-root override

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```



### Lint 规则 (`.golangci.yml`)

在M0任务0中配置；预计在各个里程碑中保持稳定：

- govet、staticcheck、errcheck、gocritic、ineffassign、gofumpt
- `forbidigo` 规则：
  - `*_test.go` 文件中的 `os.UserHomeDir`（必须使用 `paths.HomeDir(env)` 代替）
  - `internal/state/` 和 `internal/render/` 中的 `time.Now()`（必须接受时钟接口以实现可测试性）

### 文件修改礼仪

- 新文件 → 创建并显示完整内容。
- 现有文件修改 → 显示更改区域之前/之后的**准确**。更改区域之外的线路不受影响。
- 当任务涉及多个文件时，将它们全部列在任务顶部的 **文件** 下。

### 跨计划参考

M1 中的任务可以通过将其引用为 `(see M0 Task 4: iox.AtomicWrite)` 来依赖 M0 中引入的助手。帮手的契约在M0任务中给出；它被视为下游里程碑的承载 API。如果 M1 需要扩展助手，M1 有自己的任务来明确地执行此操作。

### 里程碑退出标准

每个里程碑计划最后都有一个“完成时间”部分，列出了工程师可以演示的用户可观察的行为。 CI必须是绿色的；绒毛干净；测试通过 `-race`。在这些检查通过之前，里程碑并未“完成”。

## 依赖关系图



```
M0 ──┬── M1 ──┬── M3 ── M4 ── M5 ── M7
     │        │
     └── M2 ──┘
              │
              └── M6   (M6 has no hard dependency on M3/M4 but ships after them
                        for ergonomic reasons; can be reordered if needed)
```



- M0是基础；一切都取决于它。
- M1和M2可以在M0之后以任意顺序实现；两者都提供 M3（漂移）、M4（插件）和 M5（项目）使用的适配器表面。
- M3至少需要一个适配器才能运行。
- M4 取决于 M3 的分类器（每个组件的漂移）。
- M5 依赖于 M4（项目覆盖与插件启用交互）。
- M6（秘密）大部分是独立的，但预计在 M3 之后，因此可以在实际应用路径下测试 `${secret:...}` 分辨率。
- M7为脱模抛光剂；取决于一切。

## 如何使用这些计划

- 每个计划都以**目标**、**架构**、**技术堆栈**、**在此里程碑中创建/修改的文件**开头，然后是编号的任务。
- 任务是小规模的（5 个子步骤；每个任务约 15-30 分钟）。工程师应该能够在一个集中的工作块中执行一项任务、提交并继续。
- 不要跳到前面。里程碑中的任务基于先前任务的代码构建。这些计划假设工程师按顺序读取每个计划文件。
- 如果任务引用同一计划中较早任务的代码，则为了清晰起见，会重复之前的代码，但工程师应该编写该代码。

## 完成时间（v1.0 整体）

用户可以端到端运行：



```bash
agentsync init
agentsync agent add claude
agentsync agent add opencode
agentsync mcp add github --command npx --args "-y,@modelcontextprotocol/server-github" --env GITHUB_TOKEN='${secret:github.token}'
agentsync secrets edit                     # paste token, save
agentsync marketplace add github:anthropics/claude-plugins-official
agentsync plugin install atlassian@anthropic
agentsync apply

# verify
jq '.mcpServers.github' ~/.claude/settings.json
jq '.mcp.github' ~/.config/opencode/opencode.json

# drift
claude /plugin install some-other-plugin    # outside agentsync
agentsync status                            # detects new plugin as foreign-managed
agentsync import claude:plugin:some-other-plugin
agentsync apply                             # propagates to opencode

# project-local
cd ~/code/myproject && touch .agentsync.toml
echo '[[mcp]]' >> .agentsync.toml
echo 'id = "company-api"' >> .agentsync.toml
echo 'type = "stdio"' >> .agentsync.toml
echo 'command = "npx"' >> .agentsync.toml
echo 'args = ["-y", "@company/mcp"]' >> .agentsync.toml
agentsync apply
ls .claude/settings.json                    # project-scope MCP landed
```



所有这些都可以在干净的 macOS / Linux / Windows 机器上运行，仅安装 `agentsync` 二进制文件（通过 Homebrew / Scoop / Chocolatey / 本机包）。运行时不需要 Go 工具链。