Error 500 (Server Error)!!1500.That’s an error.There was an error. Please try again later.That’s all we know.

**漂移** = `hash(destination) ≠ hash(applied)` 记录在 `.state/targets.json` 中。
**来源更改** = `hash(target) ≠ hash(applied)`。
两者 → 冲突（用户必须协调）。

### TOML 结构是规范模型

没有单独的“内部 IR”翻译层。解析 `~/.agentsync/` 中 TOML 文件的 Go 结构体是规范模型。每个适配器实现：



```go
type Adapter interface {
    Name() string
    Capabilities() Capability
    Detect(env Env) (bool, error)
    Paths(scope Scope, project string) Paths
    Render(canonical CanonicalModel, scope Scope, project string) ([]FileOp, []Skip, error)
    Ingest(scope Scope, project string) (CanonicalModel, error)   // for drift + import
    Apply(ops []FileOp) error
}
```



添加代理=添加实现此接口的`internal/adapter/<name>/`。规范模式保持不变。

---

## 源代码库布局

`~/.agentsync/`（可通过`AGENTSYNC_HOME`覆盖）：



```
~/.agentsync/
├── agentsync.toml              # global: agent registry, default update mode, secrets backend
├── mcp/<server>.toml          # one MCP server per file
├── marketplaces/<name>.toml   # one marketplace registry per file
├── plugins/<id>.toml          # one plugin enable + version pin per file
├── memory/
│   ├── AGENTS.md              # canonical memory; rendered per agent
│   └── fragments/*.md         # @-importable from above
├── skills/<name>/             # a skill is a DIRECTORY: SKILL.md + bundled
│                              #   scripts/, references/, assets/, nested files
├── subagents/<name>.md        # one subagent per file (NOT `agents/` — that word
│                              #   names the harness registry in agentsync.toml)
├── commands/<name>.md         # one slash command per file
├── hooks/<event>.toml         # one hook event per file
├── lsp/<server>.toml          # one LSP server per file
├── secrets/secrets.age        # age-encrypted; agentsync secrets {edit,get,set}
├── ignore.toml                # paths to suppress from drift reporting
└── .state/                    # gitignored
    ├── targets.json           # last-applied hashes
    ├── apply.lock             # gofrs/flock
    ├── staging/               # two-phase write tmpdir
    ├── backups/<ts>/          # first-apply backups
    └── cache/
        ├── marketplaces/<slug>/
        └── plugins/<id>/
```



### 规范模式（代表性 TOML）



```toml
# agentsync.toml
[agents]
claude   = { enabled = true,  scope = "user" }
opencode = { enabled = true,  scope = "user" }
codex    = { enabled = false }   # v1.1
cursor   = { enabled = false }   # v1.2

[updates]
default_mode     = "track"        # pinned | track | manual
default_interval = "24h"

[secrets]
backend       = "age"
file          = "secrets/secrets.age"
recipient     = "age1abc..."
identity_file = "${env:HOME}/.config/agentsync/age.key"
```





```toml
# mcp/github.toml
[server]
type    = "stdio"
command = "npx"
args    = ["-y", "@modelcontextprotocol/server-github"]
agents  = ["*"]                  # or ["claude","opencode"]; empty/"*" = all enabled

[server.env]
GITHUB_TOKEN = "${secret:github.token}"
```





```toml
# marketplaces/anthropic.toml
[marketplace]
url = "https://github.com/anthropics/claude-plugins-official"
ref = "main"
default_update_mode = "track"
```





```toml
# plugins/atlassian.toml
[plugin]
id           = "atlassian@anthropic"
version      = "1.2.3"
manifest_sha = "abc123..."           # detects re-uploads of same version
update       = "track"                # overrides marketplace default
agents       = ["claude","opencode"]  # default = all enabled

[plugin.overrides.cursor]
commands = "skip"                     # don't project /commands as Cursor rules
```





```markdown
<!-- memory/AGENTS.md -->
# Personal coding conventions

@import ./fragments/style.md
@import ./fragments/security-rules.md
```



### 项目标记



```toml
# <project-repo>/.agentsync.toml
agents = ["claude", "codex"]      # subset that applies in this project (default = all enabled)

[[mcp]]
id      = "company-api"
type    = "stdio"
command = "npx"
args    = ["-y", "@company/mcp"]
[mcp.env]
COMPANY_TOKEN = "${secret:company.api_token}"

[plugins]
disabled = ["screenshot"]

[memory]
import = ["./AGENTS.md"]          # project-relative, merged into rendered memory
```



发现：从 cwd 向上走寻找 `.agentsync.toml`。第一场比赛获胜。没有中央登记处。

---

## 插件模型

agentsync 使用 Claude 市场插件作为规范插件形式。架构来源：https://code.claude.com/docs/en/plugin-marketplaces。

### 插件源码（5种，均为v1.0）

|来源 |类型 |领域 |
|---|---|---|
|相对路径|字符串 `./...` |在市场回购内 |
| `github` |对象| `repo`、`ref?`、`sha?` |
| `url`（任何 git URL）|对象| `url`、`ref?`、`sha?` |
| `git-subdir`（稀疏克隆）|对象| `url`、`path`、`ref?`、`sha?` |
| `npm` |对象| `package`、`version?`、`registry?` |

- **`npm` 源**：agentsync 通过注册表 HTTP API 获取 tarball 并提取。用户计算机上不需要 `npm`/`Bun`。
- **`git-subdir`**：在支持的情况下使用 go-git 部分/稀疏克隆；如果 go-git 的稀疏支持在给定版本上不完整，则 shells 为 `git` 作为后备。

### 严格模式

`strict: true`（默认）- `<plugin>/.claude-plugin/plugin.json` 是权威的；市场准入补充（两者合并）。
`strict: false` — 市场进入是整个定义；与 `plugin.json` 冲突是一个错误。

agentsync 与 Claude Code 一样尊重此标志。

### `${CLAUDE_PLUGIN_ROOT}` 替换

该变量出现在挂钩命令和 MCP 服务器配置中（例如 `command = "${CLAUDE_PLUGIN_ROOT}/scripts/validate.sh"`）。

- 对于克劳德本人：逐字逐句地传递。克劳德在运行时解决它。
- 对于非 Claude 代理：agentsync 在应用时解析为 `~/.claude/plugins/cache/<plugin>/`。否则，该路径在这些代理的配置中将毫无意义。

### 版本解析

- `version` 设置（在市场入口或 `plugin.json`）→ 固定。
- `version` 省略 → 每个 git 提交都是一个新的“版本”。 `agentsync plugin outdated` 通过警告显示此内容，因此可以看到意外的漂移：`plugin foo: version unpinned; tracking commit a1b2c3 → d4e5f6`。

### `~/.claude/plugins/cache/` 是共同拥有的

agentsync 写入它安装的插件内容。 Claude本身也可以直接安装插件（`/plugin install foo`）。对于未安装agentsync 的插件，此目录中的漂移被报告为**外部管理**，而不是漂移。 agentsync 仅拥有它创建的 `.state/targets.json` 中的行。

### 保留的市场名称

`claude-plugins-official`、`anthropic-marketplace` 等会在 `marketplace add` 上触发警告，但不会阻止。

### 技能主题是开放式的

`name` 和 `description` 之外的字段（例如 `disable-model-invocation`）是真实的。 agentsync 的规范 `Skill` 类型带有 `Frontmatter map[string]any` 并逐字写入 — 永远不会丢失未知密钥。

---

## 每个组件的翻译表

插件是一包组件。每个组件都是根据目标代理独立翻译的。

- ✓ 原生——完全保真，代理直接有概念
- ◐ 预计 — 有损翻译，明确报告（例如 Claude `/command` → 游标规则丢失 `/`-调用）
- ✗ 跳过 — 没有可辩护的翻译，已登录应用输出

|组件|克劳德 (v1) |开放代码 (v1) |法典 (v1.1) |光标 (v1.2) |
|---|---|---|---|---|
| MCP服务器| ✓ 原生 | ✓ 原生（在 `opencode.json` 中）| ✓ `config.toml` 中的 `[mcp_servers.X]` | ✓ `mcp.json` |
|内存| ✓ `CLAUDE.md` | ✓ `AGENTS.md` | ✓ `~/.codex/AGENTS.md` | ✓ `AGENTS.md`（光标本机读取）|
|技能| ✓ `~/.claude/skills/X/SKILL.md` | ✓ 相同路径（OpenCode 读取 `.claude/skills/`）| ✓ `~/.agents/skills/X/SKILL.md` | ✗ 跳过 — 无技能概念 |
|子代理 | ✓ `~/.claude/agents/X.md` | ◐ 预计 — `.opencode/agents/X.md` w/ frontmatter munging (`tools` → `permission`, `color` 下降, `mode: subagent` 添加) | ◐ 投影 — `.codex/agents/X.toml` (markdown→TOML; body→`developer_instructions`) | ✗ 跳过 |
|斜线命令 | ✓ `~/.claude/commands/X.md` | ◐ 预计 — `.opencode/commands/X.md`;正文保留为模板，frontmatter 字段未全部映射（`argument-hint` 删除，未添加 OpenCode 特定字段）| ✗ 跳过 — 无自定义斜线命令 | ◐ 投影 — `.cursor/rules/X.mdc` 手动规则，body=template |
|钩| ✓ 设置中的 JSON | ✗skip(warn) - OpenCode hooks 是 JS/TS 插件（shim 生成延迟）| ◐ 预计 — `hooks.json`，5/9 事件重叠，需要 `[features] codex_hooks = true` | ◐ 预计 — `hooks.json`，~6/9 事件重叠（光标特有的 Tab 挂钩）|
| LSP服务器| ✗ 跳过 — 更正后设计：Claude Code 仅从插件清单中读取 LSP，而不是 `settings.json`（参见#66/#73）| ✗skip(warn) — LSP 投影推迟到 v1.x | ✗ 跳过（警告）— 无 LSP 概念 | ✗ Skip(warn) — 光标继承 VSCode LSP 但投影延迟 |

**技能写入策略**：当一项技能必须到达读取不同路径的多个代理时，agentsync 会直接将相同的 `SKILL.md` 内容写入每个路径。 **永远没有符号链接。** 每个技能有两个文件操作，均在状态中进行跟踪。相对于磁盘成本的稳健性。

**翻译报告** 在每个 `apply` 和 `verify` 末尾：



```
plugin: atlassian@anthropic
  claude    ✓ full   (1 mcp, 5 commands)
  opencode  ✓ full   (1 mcp, 5 commands)
  codex     ◐ partial (1 mcp; 5 commands skipped)
  cursor    ◐ partial (1 mcp; 5 commands → .cursor/rules/*.mdc)
```



通过 `agentsync plugin explain <plugin> --json` 构造相同的数据。

### 逃生舱口

- `agents = [...]` 插件条目上的白名单 → 仅向这些条目展开。 **已发货。**
- `[plugin.overrides.<agent>] component = "skip"` → 每个组件覆盖。
  **v1.0 中未接线** — 投影机不查阅表格；使用
  `agents` 允许名单。
- `apply` 上的 `--strict` 标志 → 将 ◐/✗ 变成硬错误，因此静默丢弃
  不可能的。 **v1.0 中未连接** — `apply` 未注册 `--strict`。
  最近的已发货登机口为 `status --exit-code` / `diff --exit-code`。

### 游标规则约束 (v1.2)

Cursor 用户级规则位于 Cursor 的应用程序本地存储中，而不是文件系统中。 **agentsync 的 Cursor 适配器仅管理项目范围规则。** 这是已记录的已知限制；通过 `~/.cursor/mcp.json` 的用户范围 MCP 可通过文件系统访问并正常工作。

---

## 漂移检测——3 路分类器

对于每个托管项目（文件或密钥），三个哈希值：

- `H_src` — 现在根据规范源计算。
- `H_applied` — 记录在 `.state/targets.json` 中。
- `H_dest` — 当前磁盘内容（或零）。

| H_applied 与 H_src | H_applied 与 H_dest |班级 | `apply` 行为 |
|---|---|---|---|
| = | = |干净|努普 |
| ≠ | = |待定 |写`H_src` |
| = | ≠ |漂移|堵塞;建议调和|
| ≠ | ≠ (`H_dest` = `H_src`) |收敛|静默刷新状态 |
| ≠ | ≠（三者均不同）|冲突|堵塞;要求协调|
| `H_applied` = 无，`H_dest` = 无 | — |新 |创建|
| `H_applied` = nil, `H_dest` ≠ nil | — |外国碰撞 |备份目的地，写|
| `H_src` = nil, `H_applied` ≠ nil | `H_dest` = `H_applied` |孤儿|删除 |
| `H_src` = nil, `H_applied` ≠ nil | `H_dest` ≠ `H_applied` |孤儿漂泊|警告;建议重新添加或显式删除 |

**粒度**：结构化文件 (JSON/JSONC/TOML) 以 JSON 指针粒度运行，每个托管密钥路径均经过三重跟踪。 **外键**（agentsync 从未写入的路径）列在 `status` 中以供了解；他们从不进入案例表。当解析失败时，算法会降级到整个文件的文件级别。

Error 500 (Server Error)!!1500.That’s an error.There was an error. Please try again later.That’s all we know.

Error 500 (Server Error)!!1500.That’s an error.There was an error. Please try again later.That’s all we know.

- **每个包的单元测试**，在适当的情况下由表驱动。
- `testdata/<adapter>/<case>/{source,expected,state_in.json,state_out.json}` 和 `afero.MemMapFs` 下每个适配器的 **黄金测试**。 `make update-golden` 重新录制。
- **每个适配器、每个组件的往返奇偶校验**：`Ingest(Apply(canonical)) == canonical`。
- **漂移案例固定装置**：在文件和关键粒度上针对至少一种结构化格式执行 9 案例分类器。
- **通过 `teatest` 样式的工具将循环测试与脚本提示协调一致。验证每个热键路径（`w`、`o`、`s`、`i`、`d`、`q`、`W`、`O`、`S`）。
- **每个适配器的端到端 shell 测试**：完整生命周期 (`init → mcp add → apply → mutate → status → reconcile → apply → clean`)。每个适配器一个，加上一个交叉适配器“MCP 在全部启用后添加”。
- **并发应用锁测试** — 针对相同的 `AGENTSYNC_TARGET_ROOT` 生成两个 `apply` 调用，断言序列化。
- **通过 `file://` 假裸仓库获取**市场（CI 中没有网络）。
- TOML 和 JSONC 往返的**注释保留模糊**：使用注释 + 键顺序断言进行数千次“解析 → 改变一个键 → 写入 → 重新解析”迭代。
- **CI**：`go test -race ./...`； `golangci-lint`（gove、staticcheck、errcheck、gocritic、forbidigo 禁止 `_test.go` 中的 `os.UserHomeDir()`）； `goreleaser release --snapshot --skip publish` 在 linux/darwin/windows × amd64/arm64 上。

---

## 公开风险

1. **Claude 市场架构不断发展。** 架构已发布。缓解措施：将模型作为版本化的 Go 结构；每周 CI doc-diff 与提交的参考快照标记新字段，而不是默默地删除它们。
2. **每个适配器的表面都在演变**（这次头脑风暴发现了两个过时的计划）。缓解措施：翻译表是一个可 grep 的文件；每个适配器都有一个带版本标记的陈旧注释，用于测试断言。
3. **OpenCode hooks → Claude hooks deferred。** OpenCode hooks 是 JS/TS 插件事件订阅；翻译需要生成垫片。如果 v1 中需要，请手动编写 shim；记录的限制。
4. **光标用户级规则**不能由agentsync管理。仅限项目范围；记录的限制。
5. **年龄私钥丢失 = 全部秘密丢失。** 自述文件必须规定密钥备份规则； agentsync 不管理备份。
6. **首先在已填充的计算机上应用**会遇到许多 `foreign-collision` 情况。缓解措施：首先备份+推荐`apply --dry-run`。
7. **并发代理在协调中进行编辑** — 漂移分类器捕获下一个 `status`；没有特殊情况逻辑；记录在案。
8. **同一逻辑服务的两个相互竞争的“官方”实现**（例如 Slack Hosted 与 npm）。规范插件 ID 包括上游所有者：`slack@slackapi` 与 `slack@modelcontextprotocol`。工具强制 `(id, owner)` 的唯一性。

---

## 超出范围 (v1.x)

- 跨机器配置同步（委托给 chezmoi）。
- 光标用户规则文件系统管理（不存在路径）。
- OpenCode hook shim 自动生成（v1 的手工作者）。
- 克劳德之外的 LSP 投影。
- 权限规范投影（每个代理都有自己的模型）。
- 继续，Gemini CLI，Aider 适配器（不在用户堆栈上）。
- 任何守护进程。