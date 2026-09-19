# agentsync M7 — 完善 + 发布

> [`overview`](2026-05-04-agentsync-v1.0-overview.md) 中的约定。基于 M0–M6 构建。

**目标：** 交付就绪 v1.0。 `explain`（每个插件透明度）、`import`（将本机编辑捕获到源代码中）、`agent disable --purge`（删除代理时删除目标文件）、goreleaser 跨平台发布管道、Homebrew Tap、Scoop 清单、Chocolatey 包、本机 Linux 包发行版（通过 goreleaser 的 deb/rpm）以及包含安装/快速入门/故障排除/已知限制文档的综合自述文件。

**架构：** 没有新的基础设施；这一里程碑抛光了现有的表面。分发配置位于存储库根目录（`.goreleaser.yaml`、`.github/workflows/release.yml`、`README.md`）。 Tap 特定文件 (`Formula/agentsync.rb`) 位于 goreleaser 推送到的单独存储库 (`spxrogers/homebrew-tap`) 中。

---

## 任务 1：`agentsync explain <plugin> [--json]`

**文件：** `internal/cli/explain.go`、`internal/cli/explain_test.go`

对于一个插件 ID，从 M4 打印每个代理的翻译报告 — 每个适配器对每个组件执行的操作。



```go
func newExplainCmd() *cobra.Command {
    var jsonOut bool
    cmd := &cobra.Command{
        Use:   "explain <plugin-id>",
        Args:  cobra.ExactArgs(1),
        Short: "show per-agent translation for one plugin",
        RunE: func(cmd *cobra.Command, args []string) error {
            pluginID := args[0]
            // Load canonical, find plugin, ask each adapter to Render WITH ONLY that plugin's components,
            // then print the per-agent ✓/◐/✗ table.
            // Reuses internal/render/report.go from M4.
            return nil
        },
    }
    cmd.Flags().BoolVar(&jsonOut, "json", false, "structured JSON output")
    return cmd
}
```



测试，提交。

---

## 任务 2：`agentsync import <selector>`

**文件：** `internal/cli/import.go`、`internal/cli/import_test.go`

选择器语法：`<agent>:<component>:<name>`。通过适配器的 `Ingest()` 从磁盘读取本机配置，找到匹配的项目，通过 `internal/source.Writer` 将其写入规范源（来自 M3 Task 7）。



```bash
# Examples:
agentsync import claude:mcp:github
agentsync import opencode:agent:reviewer
agentsync import claude:plugin:atlassian-anthropic   # post-M4: import a plugin install record
```



实现：解析选择器，分派到正确的摄取路径，按名称查找匹配项，通过 Writer 编组回规范。测试每种组件类型。犯罪。

---

## 任务 3：`agentsync agent disable --purge`

**文件：**修改`internal/cli/agent.go`

今天 `agent disable` 仅翻转 `enabled` 位。使用 `--purge`，还会为该代理遍历 `state.Files` + `state.Keys` 并发出删除 FileOps（使用适配器的 `Apply`），以便清理目标。

- [ ] **测试**



```go
func TestAgentDisable_Purge_RemovesDestFiles(t *testing.T) {
    // setup: apply with an MCP, verify ~/.claude.json has it
    // run: agent disable claude --purge
    // verify: ~/.claude.json mcpServers.github gone (or whole file removed if all keys ours)
}
```



- [ ] **实现**：添加 `--purge` 标志。清除时：迭代代理的状态，构建删除操作（或合并操作，将我们的键设置为缺席），调用Apply，删除状态条目。犯罪。

---

## 任务 4：README 扩展

**文件：**修改`README.md`

要添加的部分：



```markdown
# agentsync

Centrally manage AI coding-agent configurations across Claude Code, OpenCode, Codex CLI, and Cursor.

## Quickstart

    agentsync init
    agentsync agent add claude
    agentsync agent add opencode
    agentsync mcp add github --command npx --args "-y,@modelcontextprotocol/server-github"
    agentsync apply

## Install

### macOS — Homebrew

    brew tap spxrogers/tap
    brew install agentsync

### Windows — Scoop

    scoop bucket add spxrogers https://github.com/spxrogers/scoop-bucket
    scoop install agentsync

### Windows — Chocolatey

    choco install agentsync

### Linux

Debian/Ubuntu:

    curl -fsSL https://github.com/spxrogers/agentsync/releases/latest/download/agentsync.deb -o agentsync.deb
    sudo dpkg -i agentsync.deb

RPM:

    sudo rpm -i https://github.com/spxrogers/agentsync/releases/latest/download/agentsync.rpm

Arch (AUR):

    yay -S agentsync

## Cross-machine sync

agentsync is single-machine. To sync `~/.agentsync/` across machines, use chezmoi (or any dotfile manager):

    chezmoi add ~/.agentsync

## Secrets — age key backup

If you lose your age private key, you lose access to all encrypted secrets. Recommended: store the key in a 1Password Secure Note or your machine-setup repo. agentsync does not back up the key for you.

## Known limits in v1.x

- **OpenCode hooks**: OpenCode hooks are JS/TS plugins, not declarative shell commands. agentsync v1 does NOT auto-translate Claude hooks to OpenCode. Hand-author a small JS/TS plugin if you need a hook on OpenCode.
- **Cursor user-level rules**: Cursor stores user-level rules in app-local storage (not the filesystem). agentsync's Cursor adapter manages project-scope rules only.
- **LSP projection beyond Claude**: OpenCode/Codex/Cursor LSP support is deferred. Claude plugins that include LSP servers install correctly on Claude itself; on other agents you'll see `lsp server X skipped` in the apply translation report.
- **Continue, Gemini CLI, Aider**: not on the v1.x roadmap.

## Troubleshooting

- **First apply on a populated machine**: agentsync sees pre-existing native config files and triggers `foreign-collision`. The original is backed up to `~/.agentsync/.state/backups/<ts>/` before the new content lands. Recommend `agentsync apply --dry-run` first to preview the translation report.
- **`agentsync update` fails to fetch a marketplace**: verify the marketplace URL with `git ls-remote`. agentsync uses `go-git` and falls back to system `git` for sparse clones if needed.
- **`${secret:foo}` not resolving**: run `agentsync secrets get foo` to verify the key exists in the decrypted file. age library errors will surface here.

## License

MIT.
```



承诺。

---

## 任务 5：goreleaser 发布管道

**文件：**修改`.goreleaser.yaml`，添加`.github/workflows/release.yml`

使用分发配置扩展 `.goreleaser.yaml`：



```yaml
brews:
  - name: agentsync
    repository:
      owner: spxrogers
      name: homebrew-tap
    homepage: https://github.com/spxrogers/agentsync
    description: Centrally manage AI coding-agent configurations
    license: MIT
    install: bin.install "agentsync"
    test: system "#{bin}/agentsync --version"

scoops:
  - name: agentsync
    repository:
      owner: spxrogers
      name: scoop-bucket
    homepage: https://github.com/spxrogers/agentsync
    description: Centrally manage AI coding-agent configurations
    license: MIT

chocolateys:
  - name: agentsync
    api_key: '{{ .Env.CHOCOLATEY_API_KEY }}'
    title: agentsync
    project_url: https://github.com/spxrogers/agentsync
    license_url: https://github.com/spxrogers/agentsync/blob/main/LICENSE
    summary: Centrally manage AI coding-agent configurations

nfpms:
  - id: linux-pkgs
    package_name: agentsync
    homepage: https://github.com/spxrogers/agentsync
    license: MIT
    formats: [deb, rpm]
    section: utils
    description: Centrally manage AI coding-agent configurations.

aurs:
  - name: agentsync-bin
    homepage: https://github.com/spxrogers/agentsync
    description: Centrally manage AI coding-agent configurations.
    maintainers: ['Steven Rogers <noreply@example.com>']
    license: MIT
    private_key: '{{ .Env.AUR_KEY }}'
    git_url: 'ssh://aur@aur.archlinux.org/agentsync-bin.git'
```



`.github/workflows/release.yml`：



```yaml
name: release
on:
  push:
    tags: ['v*']

permissions:
  contents: write

jobs:
  release:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with: { fetch-depth: 0 }
      - uses: actions/setup-go@v5
        with: { go-version: '1.22.x' }
      - uses: goreleaser/goreleaser-action@v6
        with: { version: latest, args: release --clean }
        env:
          GITHUB_TOKEN:        ${{ secrets.GH_PAT }}     # for tap + bucket pushes
          CHOCOLATEY_API_KEY:  ${{ secrets.CHOCO_KEY }}
          AUR_KEY:             ${{ secrets.AUR_KEY }}
```



（在存储库上配置的必需机密：`GH_PAT` 范围为 `repo`，以便 goreleaser 可以推送到 `homebrew-tap` 和 `scoop-bucket`；来自 Chocolatey.org 的 `CHOCO_KEY`；`AUR_KEY` ed25519 SSH 密钥注册到维护者的 AUR 帐户。）

- [ ] **冒烟测试**：标记 `v1.0.0-rc.1`，推送，验证发布工作流程生成所有工件并推送到水龙头。如果出现任何问题，请回滚标记，然后再使用 `v1.0.0` 重试。

犯罪。

---

## 任务 6：配套存储库

**一次性设置**（不是对此存储库的代码更改）：

1. 创建 `spxrogers/homebrew-tap` 存储库（空；填充 goreleaser）。
2. 创建 `spxrogers/scoop-bucket` 存储库（空；填充 goreleaser）。
3. 使用与 `AUR_KEY` 匹配的 SSH 密钥在 AUR 上注册 `agentsync-bin` 包。
4. 在 Chocolatey (community.chocolatey.org) 上注册 `agentsync` 包。

这些步骤记录在自述文件的“发布”附录中；运行 v1.0 版本的工程师只执行一次。

---

## 任务 7：最终集成测试 — 完整的 v1.0 生命周期

**文件：** `test/e2e/v1_lifecycle_test.go`（新的顶级 e2e 线束）

顶级 shell 式集成测试，可测试每个 M0–M7 表面：



```go
//go:build e2e
package e2e

import (
    "os"
    "os/exec"
    "path/filepath"
    "strings"
    "testing"

    "filippo.io/age"
)

// Build agentsync into a tmp bin dir; tests then exec it like a user would.
func TestE2E_FullV1Lifecycle(t *testing.T) {
    bin := buildBinary(t)
    home := t.TempDir()
    env := append(os.Environ(),
        "AGENTSYNC_TARGET_ROOT="+home,
        "HOME="+home,
        "PATH="+filepath.Dir(bin)+string(filepath.ListSeparator)+os.Getenv("PATH"),
    )
    run := func(args ...string) (string, error) {
        cmd := exec.Command(bin, args...)
        cmd.Env = env
        out, err := cmd.CombinedOutput()
        return string(out), err
    }

    // 1. init
    if _, err := run("init"); err != nil { t.Fatal(err) }

    // 2. agents
    _, _ = run("agent", "add", "claude")
    _, _ = run("agent", "add", "opencode")

    // 3. age secrets
    id, _ := age.GenerateX25519Identity()
    _ = os.MkdirAll(filepath.Join(home, ".config", "agentsync"), 0o755)
    _ = os.WriteFile(filepath.Join(home, ".config", "agentsync", "age.key"), []byte(id.String()), 0o600)
    cfg := filepath.Join(home, ".agentsync", "agentsync.toml")
    body, _ := os.ReadFile(cfg)
    body = append(body, []byte("[secrets]\nbackend=\"age\"\nfile=\"secrets/secrets.age\"\nrecipient=\""+id.Recipient().String()+"\"\nidentity_file=\""+filepath.Join(home, ".config", "agentsync", "age.key")+"\"\n")...)
    _ = os.WriteFile(cfg, body, 0o644)
    // ... (encrypt secrets file, mcp w/ ${secret:...}, marketplace add, plugin install, apply, status, reconcile, etc.)
}

func buildBinary(t *testing.T) string {
    t.Helper()
    dir := t.TempDir()
    bin := filepath.Join(dir, "agentsync")
    cmd := exec.Command("go", "build", "-o", bin, "./cmd/agentsync")
    if out, err := cmd.CombinedOutput(); err != nil {
        t.Fatalf("build: %v\n%s", err, out)
    }
    return bin
}
```



CI 在推送之前在发布工作流上运行 `go test -tags=e2e ./test/e2e/...`。犯罪。

---

## 任务 8：最终提交 + 标记



```bash
go test -race ./...
golangci-lint run ./...
goreleaser release --snapshot --skip publish --clean    # local sanity

git tag v1.0.0
git push origin v1.0.0
```



GitHub Actions 发布作业会获取标签并发送所有内容。

---

## 完成时间

- `agentsync explain <plugin>` 显示任何已安装插件的每个代理翻译表（以人类或 `--json` 形式）。
- `agentsync import <agent>:<component>:<name>` 将任何本机配置捕获回规范。
- `agentsync agent disable claude --purge` 删除代理并清理我们拥有的目标文件。
- `git tag v1.0.0 && git push origin v1.0.0` 触发释放：
  - 构建 Linux/macOS/Windows × amd64/arm64 二进制文件。
  - 将 Homebrew 公式推至 `spxrogers/homebrew-tap`。
  - 将 Scoop 清单推送到 `spxrogers/scoop-bucket`。
  - 提交巧克力包。
  - 推动 AUR PKGBUILD。
  - 将 `.deb` + `.rpm` 上传到 GitHub 版本。
- 自述文件记录了快速入门、按平台安装、跨机器同步、年龄密钥备份、已知限制和故障排除。
- E2E测试在linux/macos/windows上通过。
- CI 所有阶段均呈绿色。

**v1.0 发布。** v1.1 (Codex) 和 v1.2 (Cursor) 是单独跟踪的计划，在 v1.0 落地并且模式得到验证后编写。