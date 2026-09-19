# agentsync M2 — OpenCode 适配器

> **对于代理工人：** 所需的子技能：使用超级能力：子代理驱动的开发（推荐）或超级能力：执行计划。 [`overview`](2026-05-04-agentsync-v1.0-overview.md#conventions-used-across-all-milestone-plans) 中的约定。基于 [M0](2026-05-04-agentsync-m0-skeleton.md) + [M1](2026-05-04-agentsync-m1-claude.md) 构建。

**目标：** 第二个适配器。实现 `internal/adapter/opencode`，涵盖 MCP、内存 (`AGENTS.md`)、技能（写入共享 `.claude/skills/<name>/SKILL.md`，因为 OpenCode 可以本地读取它）、子代理（带有不同 frontmatter 的 markdown）、斜杠命令（带有 `template:` frontmatter 的 markdown）。挂钩 `✗ skip(warn)`（OpenCode 挂钩是 JS/TS 插件事件订阅；垫片生成延迟）。 LSP `✗ skip(warn)`。将注册表中的 `opencode` 替换为 NoopAdapter。

**架构：** 反映 M1 的封装布局。设置文件为 `~/.config/opencode/opencode.json` — **JSONC** （带注释的 JSON）。 `tailscale/hujson` 解析它； M1 的 `claude` 包中的 `MergeKeys` 被“通用化”并移至共享包（或重复；权衡如下所述）。

**决定：** 将 `MergeKeys`、`splitPointer`、`pointerExists`、`deletePointer`、`deepCopyMap`、`mergeMaps` 从 `internal/adapter/claude/settings.go` 移动到 `internal/jsonkeys/jsonkeys.go`，以便 OpenCode 和（最终）游标适配器共享。 M2 任务 1 进行此重构。

**技术堆栈：** `tailscale/hujson` 用于 JSONC 解析/保留，其他方面与 M1 相同。

---

## 创建/修改的文件



```
NEW:
internal/jsonkeys/                 # extracted from M1 claude settings.go
├── jsonkeys.go
└── jsonkeys_test.go

internal/adapter/opencode/
├── opencode.go                    # Adapter, Name, Capabilities, Detect
├── paths.go                       # ~/.config/opencode/* path resolution
├── render.go                      # Render(canonical) -> []FileOp
├── render_test.go
├── ingest.go                      # Disk -> source.Canonical
├── ingest_test.go
├── apply.go                       # FileOp writer, JSONC-aware
├── apply_test.go
├── settings.go                    # opencode.json (JSONC) merge logic
├── settings_test.go
├── skill.go                       # write SKILL.md to .claude/skills/ shared path
├── subagent.go                    # frontmatter munge: tools->permission, color drop, mode add
├── command.go                     # frontmatter munge for commands
└── memory.go                      # AGENTS.md (project root for project scope)

MODIFIED:
internal/adapter/claude/settings.go    # delegate to internal/jsonkeys
internal/cli/registry_internal.go      # register opencode.New(...)
```



---

## 任务 1：从 claude 设置中提取 `internal/jsonkeys`

**文件：**
- 创建：`internal/jsonkeys/jsonkeys.go`、`internal/jsonkeys/jsonkeys_test.go`
- 修改：`internal/adapter/claude/settings.go`

- [ ] **步骤 1.1：移动代码**

将 `internal/adapter/claude/settings.go` 的正文复制到 `internal/jsonkeys/jsonkeys.go`，将包名称更改为 `jsonkeys`，然后重新导出 `MergeKeys`。也移动测试文件（`internal/jsonkeys/jsonkeys_test.go` 通过包重命名镜像现有测试）。

- [ ] **步骤 1.2：存根 claude/settings.go 到委托**



```go
package claude

import "github.com/spxrogers/agentsync/internal/jsonkeys"

// MergeKeys is preserved as a re-export for backward compat with claude
// internal callers; new callers should import internal/jsonkeys directly.
func MergeKeys(existing, ours map[string]any, ownedPointers []string) (map[string]any, []string, []string) {
    return jsonkeys.MergeKeys(existing, ours, ownedPointers)
}
```



- [ ] **步骤1.3：运行；提交**



```bash
go test -race ./internal/jsonkeys/... ./internal/adapter/claude/...
git add internal/jsonkeys internal/adapter/claude
git commit -m "$(cat <<'EOF'
refactor(jsonkeys): extract per-key JSON merge from claude adapter

Identical behavior, shared package. Claude re-exports MergeKeys for
internal callers; opencode and future adapters import jsonkeys directly.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```



---

## 任务 2：添加 `tailscale/hujson` 依赖项 + JSONC 设置模块

**文件：**
- 修改：`go.mod`
- 创建：`internal/adapter/opencode/settings.go`、`internal/adapter/opencode/settings_test.go`

`opencode.json` 是 JSONC：它可以有 `// line comments` 和尾随逗号。我们必须往返它们。

- [ ] **步骤2.1：添加依赖**



```bash
go get github.com/tailscale/hujson@latest
```



- [ ] **步骤 2.2：测试解析 + 渲染往返**



```go
package opencode_test

import (
    "strings"
    "testing"

    "github.com/spxrogers/agentsync/internal/adapter/opencode"
)

func TestApplyJSONCMerge_PreservesComments(t *testing.T) {
    existing := []byte(`{
  // top comment, must survive
  "foreign": 1,
  "mcp": {
    "stale": {} // soon-to-be-removed
  }
}`)
    ours := map[string]any{
        "mcp": map[string]any{
            "github": map[string]any{"command": "npx"},
        },
    }
    owned := []string{"/mcp/stale", "/mcp/github"}

    out, err := opencode.MergeJSONC(existing, ours, owned)
    if err != nil {
        t.Fatal(err)
    }
    s := string(out)
    if !strings.Contains(s, "top comment, must survive") {
        t.Fatalf("comment lost:\n%s", s)
    }
    if strings.Contains(s, "stale") {
        t.Fatalf("stale should be removed:\n%s", s)
    }
    if !strings.Contains(s, `"github"`) {
        t.Fatalf("github not added:\n%s", s)
    }
}
```



- [ ] **步骤 2.3：实施**

`internal/adapter/opencode/settings.go`：



```go
// Package opencode-settings: JSONC-aware merge for opencode.json. Uses
// tailscale/hujson which preserves comments and trailing commas across
// parse->mutate->serialize cycles.
package opencode

import (
    "encoding/json"
    "fmt"

    "github.com/tailscale/hujson"
    "github.com/spxrogers/agentsync/internal/jsonkeys"
)

// MergeJSONC merges ours into existing JSONC content, removing ownedPointers
// no longer present in ours. Comments and trailing-comma formatting from
// existing are preserved as much as the hujson AST allows.
//
// Strategy: parse existing JSONC -> standardize to JSON (Pack), MergeKeys,
// then format result as plain JSON. v1 trade-off: trailing comma + comment
// preservation is partial — comments outside touched keys survive; comments
// adjacent to deleted keys are also removed. M2 ships this; comment-position
// fidelity can be tightened later if pain emerges.
func MergeJSONC(existing []byte, ours map[string]any, ownedPointers []string) ([]byte, error) {
    if len(existing) == 0 {
        existing = []byte("{}")
    }
    val, err := hujson.Parse(existing)
    if err != nil {
        return nil, fmt.Errorf("parse jsonc: %w", err)
    }
    val.Standardize()
    var existingMap map[string]any
    if err := json.Unmarshal(val.Pack(), &existingMap); err != nil {
        return nil, fmt.Errorf("standardize jsonc: %w", err)
    }
    if existingMap == nil {
        existingMap = map[string]any{}
    }
    merged, _, _ := jsonkeys.MergeKeys(existingMap, ours, ownedPointers)
    out, err := json.MarshalIndent(merged, "", "  ")
    if err != nil {
        return nil, fmt.Errorf("marshal merged: %w", err)
    }
    return append(out, '\n'), nil
}
```



（“与已删除的键相邻时注释丢失”的评论是诚实的——带有手术键编辑的完整 hujson AST 往返更加复杂；v1 提供了更简单的标准化+合并方法。权衡记录在案。）

- [ ] **步骤2.4：运行；提交**



```bash
go test -race ./internal/adapter/opencode/...
git add go.mod go.sum internal/adapter/opencode
git commit -m "$(cat <<'EOF'
feat(adapter/opencode): JSONC-aware merge via tailscale/hujson + jsonkeys

opencode.json is JSONC; hujson parses comments+trailing-commas. v1 ships
standardize+merge; trailing-comma fidelity may regress in edge cases —
documented trade-off. Comment fidelity around touched keys is partial;
fine for the typical "edit one MCP server" path.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```



---

## 任务 3：适配器骨架 + Detect()

**文件：**
- 创建：`internal/adapter/opencode/{opencode.go, paths.go, opencode_test.go}`

镜像 M1 任务 1。检测：路径上存在 `~/.config/opencode/` 或 `opencode`。

- [ ] **实施**

`internal/adapter/opencode/paths.go`：



```go
package opencode

import "path/filepath"

type Paths struct {
    ConfigDir       string // ~/.config/opencode
    Settings        string // ~/.config/opencode/opencode.json
    AgentsDir       string // ~/.config/opencode/agents (user); or .opencode/agents (project)
    CommandsDir     string
    ClaudeSkillsDir string // ~/.claude/skills (shared with Claude!)
    Memory          string // CLAUDE not used; AGENTS.md at project root
}

func ResolvePaths(targetRoot, project string, projectScope bool) Paths {
    if projectScope && project != "" {
        return Paths{
            ConfigDir:       filepath.Join(project, ".opencode"),
            Settings:        filepath.Join(project, ".opencode", "opencode.json"),
            AgentsDir:       filepath.Join(project, ".opencode", "agents"),
            CommandsDir:     filepath.Join(project, ".opencode", "commands"),
            ClaudeSkillsDir: filepath.Join(project, ".claude", "skills"),
            Memory:          filepath.Join(project, "AGENTS.md"),
        }
    }
    cfg := filepath.Join(targetRoot, ".config", "opencode")
    return Paths{
        ConfigDir:       cfg,
        Settings:        filepath.Join(cfg, "opencode.json"),
        AgentsDir:       filepath.Join(cfg, "agents"),
        CommandsDir:     filepath.Join(cfg, "commands"),
        ClaudeSkillsDir: filepath.Join(targetRoot, ".claude", "skills"),
        Memory:          filepath.Join(targetRoot, ".config", "opencode", "AGENTS.md"),
    }
}
```



`internal/adapter/opencode/opencode.go`：



```go
package opencode

import (
    "os"
    "os/exec"

    "github.com/spxrogers/agentsync/internal/adapter"
)

type Options struct{ TargetRoot string }

type Adapter struct{ opts Options }

func New(opts Options) *Adapter { return &Adapter{opts: opts} }

func (a *Adapter) Name() string { return "opencode" }

func (a *Adapter) Capabilities() adapter.Capability {
    return adapter.CapMCP | adapter.CapMemory | adapter.CapSkill |
        adapter.CapSubagent | adapter.CapCommand
    // Hook + LSP capabilities omitted: we ship them as ✗ skip in v1.
}

func (a *Adapter) Detect() (bool, error) {
    p := ResolvePaths(a.opts.TargetRoot, "", false)
    if _, err := os.Stat(p.ConfigDir); err == nil {
        return true, nil
    }
    if _, err := exec.LookPath("opencode"); err == nil {
        return true, nil
    }
    return false, nil
}
```



测试 M1 任务 1 的模拟，提交。

---

## 任务 4：渲染 — MCP

OpenCode `opencode.json` 形状：`{"mcp": {"<id>": {"command": "...", "args": [...], "env": {...}}}}` — 请注意，密钥是 `mcp` 而不是 `mcpServers`。

- [ ] **测试**



```go
func TestRender_MCP(t *testing.T) {
    enabled := true
    c := source.Canonical{MCPServers: []source.MCPServer{{
        ID: "github",
        Server: source.MCPServerSpec{
            Type: "stdio", Command: "npx", Args: []string{"-y", "x"},
            Agents: []string{"opencode"}, Enabled: &enabled,
        },
    }}}
    a := opencode.New(opencode.Options{TargetRoot: t.TempDir()})
    ops, _, _ := a.Render(c, adapter.ScopeUser, "")
    var found bool
    for _, op := range ops {
        if strings.HasSuffix(op.Path, "opencode.json") {
            found = true
            if op.MergeStrategy != "merge-jsonc-keys" {
                t.Fatalf("merge strategy = %q", op.MergeStrategy)
            }
            var ours map[string]any
            _ = json.Unmarshal(op.Content, &ours)
            mcp := ours["mcp"].(map[string]any)["github"].(map[string]any)
            if mcp["command"] != "npx" {
                t.Fatalf("command = %v", mcp["command"])
            }
        }
    }
    if !found {
        t.Fatal("opencode.json op missing")
    }
}
```



- [ ] **实施**

`internal/adapter/opencode/render.go`：



```go
package opencode

import (
    "encoding/json"
    "fmt"

    "github.com/spxrogers/agentsync/internal/adapter"
    "github.com/spxrogers/agentsync/internal/source"
)

func (a *Adapter) Render(c source.Canonical, scope adapter.Scope, project string) ([]adapter.FileOp, []adapter.Skip, error) {
    p := ResolvePaths(a.opts.TargetRoot, project, scope == adapter.ScopeProject)
    var ops []adapter.FileOp
    var skips []adapter.Skip

    if mcpOps, err := a.renderMCP(c, p); err != nil {
        return nil, nil, err
    } else {
        ops = append(ops, mcpOps...)
    }
    if memOps, err := a.renderMemory(c, p); err != nil {
        return nil, nil, err
    } else {
        ops = append(ops, memOps...)
    }
    if skOps, err := a.renderSkills(c, p); err != nil {
        return nil, nil, err
    } else {
        ops = append(ops, skOps...)
    }
    if saOps, saSkips, err := a.renderSubagents(c, p); err != nil {
        return nil, nil, err
    } else {
        ops = append(ops, saOps...)
        skips = append(skips, saSkips...)
    }
    if cmdOps, err := a.renderCommands(c, p); err != nil {
        return nil, nil, err
    } else {
        ops = append(ops, cmdOps...)
    }
    // Hooks
    for _, h := range c.Hooks {
        skips = append(skips, adapter.Skip{
            Component: "hook", Name: h.Event,
            Reason: "OpenCode hooks are JS/TS plugins; shim generation deferred to post-v1",
        })
    }
    // LSP
    for _, l := range c.LSPServers {
        skips = append(skips, adapter.Skip{
            Component: "lsp", Name: l.ID,
            Reason: "OpenCode LSP projection deferred to v1.x",
        })
    }
    return ops, skips, nil
}

func (a *Adapter) renderMCP(c source.Canonical, p Paths) ([]adapter.FileOp, error) {
    mcp := map[string]any{}
    for _, m := range c.MCPServers {
        if m.Server.Enabled != nil && !*m.Server.Enabled {
            continue
        }
        if !agentTargeted("opencode", m.Server.Agents) {
            continue
        }
        spec := map[string]any{}
        if m.Server.Type != "" {
            spec["type"] = m.Server.Type
        }
        if m.Server.Command != "" {
            spec["command"] = m.Server.Command
        }
        if len(m.Server.Args) > 0 {
            spec["args"] = m.Server.Args
        }
        if len(m.Server.Env) > 0 {
            spec["env"] = m.Server.Env
        }
        if m.Server.URL != "" {
            spec["url"] = m.Server.URL
        }
        mcp[m.ID] = spec
    }
    if len(mcp) == 0 {
        return nil, nil
    }
    ours := map[string]any{"mcp": mcp}
    body, err := json.MarshalIndent(ours, "", "  ")
    if err != nil {
        return nil, fmt.Errorf("marshal opencode mcp: %w", err)
    }
    return []adapter.FileOp{{
        Action:        "write",
        Path:          p.Settings,
        Content:       append(body, '\n'),
        Mode:          0o644,
        SourceID:      "mcp/* (multiple)",
        MergeStrategy: "merge-jsonc-keys",
    }}, nil
}

func agentTargeted(name string, agents []string) bool {
    if len(agents) == 0 {
        return true
    }
    for _, a := range agents {
        if a == "*" || a == name {
            return true
        }
    }
    return false
}
```



承诺：



```bash
git commit -am "feat(adapter/opencode): render MCP into opencode.json with merge-jsonc-keys

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```



---

## 任务 5：渲染 — 内存 (AGENTS.md)

`renderMemory` 将 `c.Memory.Body`（带有 @import 扩展）写入 `p.Memory`。与 Claude 的 renderMemory 形状相同，但路径不同。适用 Claude/M1 任务 6 的技能测试；复制并调整路径期望。犯罪。

---

## 任务 6：渲染 — 技能（写入共享 `.claude/skills/`）

OpenCode 原生读取 `.claude/skills/<name>/SKILL.md`。我们将 SKILL.md 写入 `p.ClaudeSkillsDir`（Claude 也在 M1 中写入）。

**与 M1 协调：**如果两个适配器渲染相同的技能文件，我们会得到两个具有相同路径和相同内容的 FileOp（因为 Skill.Frontmatter 和 Body 是确定性的）。应用管道必须对每个路径进行重复数据删除。将其添加到 render.Apply （在 `internal/render/pipeline.go` 中）：



```go
func Apply(p Plan, reg *adapter.Registry) error {
    seen := map[string]bool{}
    for name, res := range p.PerAgent {
        a := reg.Lookup(name)
        if a == nil {
            return fmt.Errorf("adapter %q not registered at apply", name)
        }
        var deduped []adapter.FileOp
        for _, op := range res.Ops {
            if op.Action == "write" {
                if seen[op.Path] {
                    continue
                }
                seen[op.Path] = true
            }
            deduped = append(deduped, op)
        }
        if err := a.Apply(deduped); err != nil {
            return fmt.Errorf("apply %s: %w", name, err)
        }
    }
    return nil
}
```



添加重复数据删除测试：



```go
func TestPipeline_DedupesIdenticalWritesAcrossAdapters(t *testing.T) {
    // ... build canonical with one Skill,
    // ... two adapters that both render the same path,
    // ... assert filesystem only sees one write.
}
```

opencode 中的渲染技能反映了 Claude 的 renderSkills。承诺：



```bash
git commit -am "feat(adapter/opencode): write skills to shared .claude/skills/ path

OpenCode reads .claude/skills/<name>/SKILL.md natively. Render emits
identical FileOp to Claude's; render.Apply dedupes per path so the file
is written once per apply even when multiple adapters target it.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```



---

## 任务 7：渲染 — 子代理（markdown w/ frontmatter munge）

将 Claude 型 subagent frontmatter 转换为 OpenCode 型。

|克劳德前线 | OpenCode 前言 |笔记|
|---|---|---|
| `description` | `description` |直接复制 |
| `model` | `model` |直接复制|
| `tools`（白名单）|下降/记录| OpenCode 使用 `permission` 模型；映射工具->权限并非易事 — 记录跳过注释 |
| `color` |下降| OpenCode没有颜色概念 |
| （无）| `mode: subagent` |总是添加 |
| （无）| `temperature` |未添加（克劳德没有携带）|
| （无）| `permission` | v1 中未添加（需要策略）|

- [ ] **测试**



```go
func TestRender_Subagent_FrontmatterMunge(t *testing.T) {
    c := source.Canonical{Subagents: []source.Subagent{{
        Name:        "review",
        Frontmatter: map[string]any{
            "description": "Code review",
            "model":       "claude-sonnet-4-7",
            "tools":       []string{"Read", "Grep"},
            "color":       "blue",
        },
        Body: "Review code.\n",
    }}}
    a := opencode.New(opencode.Options{TargetRoot: t.TempDir()})
    ops, skips, _ := a.Render(c, adapter.ScopeUser, "")
    // verify file content
    var op *adapter.FileOp
    for i, o := range ops {
        if strings.HasSuffix(o.Path, "/agents/review.md") {
            op = &ops[i]
        }
    }
    if op == nil {
        t.Fatal("no agent op")
    }
    if !strings.Contains(string(op.Content), "mode: subagent") {
        t.Fatalf("missing mode:subagent in: %s", op.Content)
    }
    if strings.Contains(string(op.Content), "color:") {
        t.Fatalf("color should be dropped: %s", op.Content)
    }
    if strings.Contains(string(op.Content), "tools:") {
        t.Fatalf("tools should be dropped: %s", op.Content)
    }
    // verify skip log
    var sawToolsSkip bool
    for _, s := range skips {
        if s.Component == "subagent-frontmatter" && s.Name == "review" {
            if strings.Contains(s.Reason, "tools") {
                sawToolsSkip = true
            }
        }
    }
    if !sawToolsSkip {
        t.Fatalf("no skip emitted for tools allowlist")
    }
}
```



- [ ] **实施**

`internal/adapter/opencode/subagent.go`：



```go
package opencode

import (
    "fmt"
    "path/filepath"

    "github.com/spxrogers/agentsync/internal/adapter"
    "github.com/spxrogers/agentsync/internal/adapter/claude"
    "github.com/spxrogers/agentsync/internal/source"
)

func (a *Adapter) renderSubagents(c source.Canonical, p Paths) ([]adapter.FileOp, []adapter.Skip, error) {
    var ops []adapter.FileOp
    var skips []adapter.Skip
    for _, s := range c.Subagents {
        out := map[string]any{}
        if v, ok := s.Frontmatter["description"]; ok {
            out["description"] = v
        }
        if v, ok := s.Frontmatter["model"]; ok {
            out["model"] = v
        }
        out["mode"] = "subagent"

        // Drop with skip notes for unmappable fields:
        if _, ok := s.Frontmatter["tools"]; ok {
            skips = append(skips, adapter.Skip{
                Component: "subagent-frontmatter", Name: s.Name,
                Reason: "Claude `tools` allowlist not projected to OpenCode `permission` (manual rule design needed)",
            })
        }
        if _, ok := s.Frontmatter["color"]; ok {
            skips = append(skips, adapter.Skip{
                Component: "subagent-frontmatter", Name: s.Name,
                Reason: "Claude `color` has no OpenCode equivalent",
            })
        }

        body, err := claude.EncodeFrontmatter(out, s.Body)
        if err != nil {
            return nil, nil, fmt.Errorf("encode opencode subagent %s: %w", s.Name, err)
        }
        ops = append(ops, adapter.FileOp{
            Action:        "write",
            Path:          filepath.Join(p.AgentsDir, s.Name+".md"),
            Content:       body,
            Mode:          0o644,
            SourceID:      filepath.Join("agents", s.Name+".md"),
            MergeStrategy: "replace",
        })
    }
    return ops, skips, nil
}
```



承诺。

---

## 任务 8：渲染 — 斜杠命令 (frontmatter munge)

Claude 命令 frontmatter (`description`、`argument-hint`、`model`) → OpenCode (`description`、`agent`、`subtask`、`model`、`template`)。

- 对于 Claude `description` → OpenCode `description`（直接）。
- `model` → `model`（直接）。
- `argument-hint` → 删除（跳过注释）。
- 降价正文成为命令正文（模板）。 OpenCode 可互换地对待 `template:` frontmatter 或 body；我们保留正文并且不添加 `template:` frontmatter 以避免重复。

- [ ] 镜像任务 7 模式。测试，实施（`command.go`），提交。

---

## 任务 9：`Apply()` — JSONC 感知编写器

类似于 Claude 的 Apply，但采用 `merge-jsonc-keys` 策略。

`internal/adapter/opencode/apply.go`：



```go
package opencode

import (
    "encoding/json"
    "fmt"
    "os"

    "github.com/spxrogers/agentsync/internal/adapter"
    "github.com/spxrogers/agentsync/internal/iox"
)

func (a *Adapter) Apply(ops []adapter.FileOp) error {
    for _, op := range ops {
        switch op.Action {
        case "delete":
            if err := os.Remove(op.Path); err != nil && !os.IsNotExist(err) {
                return fmt.Errorf("delete %s: %w", op.Path, err)
            }
        case "", "write":
            if err := a.applyWrite(op); err != nil {
                return err
            }
        default:
            return fmt.Errorf("unknown action %q", op.Action)
        }
    }
    return nil
}

func (a *Adapter) applyWrite(op adapter.FileOp) error {
    mode := os.FileMode(op.Mode)
    if mode == 0 {
        mode = 0o644
    }
    if op.MergeStrategy == "merge-jsonc-keys" {
        existing, _ := os.ReadFile(op.Path)
        var ours map[string]any
        if err := json.Unmarshal(op.Content, &ours); err != nil {
            return fmt.Errorf("parse our payload: %w", err)
        }
        out, err := MergeJSONC(existing, ours, op.OwnedKeys)
        if err != nil {
            return err
        }
        return iox.AtomicWrite(op.Path, out, mode)
    }
    return iox.AtomicWrite(op.Path, op.Content, mode)
}
```



测试（镜像 Claude 应用测试，断言 JSONC 注释有效）。犯罪。

---

## 任务 10：`Ingest()`

渲染的逆。读取 `opencode.json` → MCP； `<AgentsDir>/*.md` → 子代理（frontmatter 重回规范：删除 `mode`，保留 `description`+`model`）； `<CommandsDir>/*.md` → 命令； AGENTS.md → Memory.Body 逐字记录。

对于子代理摄取：我们丢失了在渲染时放置的工具/颜色字段 - 因此，往返对于子代理来说不是奇偶校验（不可能是奇偶校验，因为数据不在磁盘上）。 OpenCode 子代理的往返测试断言 `Render(Ingest(Apply(in))) ≈ in` 模丢弃字段。

测试、实施、提交。

---

## 任务 11：连接到注册表 + 集成测试

修改`internal/cli/registry_internal.go`：



```go
_ = r.Register(opencode.New(opencode.Options{TargetRoot: home}))
```



将 noop 条目替换为“opencode”。

集成测试：



```go
func TestIntegration_M2_OpenCodeMCPApply(t *testing.T) {
    tmp := t.TempDir()
    env := map[string]string{"AGENTSYNC_TARGET_ROOT": tmp}

    _, _ = runCLI(t, env, "init")
    _, _ = runCLI(t, env, "agent", "add", "claude")
    _, _ = runCLI(t, env, "agent", "add", "opencode")

    mcpFile := filepath.Join(tmp, ".agentsync", "mcp", "github.toml")
    _ = os.MkdirAll(filepath.Dir(mcpFile), 0o755)
    _ = os.WriteFile(mcpFile, []byte(`
[server]
type    = "stdio"
command = "npx"
args    = ["-y", "@modelcontextprotocol/server-github"]
`), 0o644)

    if _, err := runCLI(t, env, "apply"); err != nil {
        t.Fatal(err)
    }

    // Both Claude and OpenCode got it
    body, _ := os.ReadFile(filepath.Join(tmp, ".claude.json"))
    if !strings.Contains(string(body), "github") {
        t.Fatalf("claude missing github: %s", body)
    }
    body, _ = os.ReadFile(filepath.Join(tmp, ".config", "opencode", "opencode.json"))
    if !strings.Contains(string(body), "github") {
        t.Fatalf("opencode missing github: %s", body)
    }
}
```



承诺：



```bash
git commit -am "$(cat <<'EOF'
feat(cli): register real opencode adapter; cross-agent MCP fanout works

End-to-end: one mcp/<id>.toml -> ~/.claude.json AND
~/.config/opencode/opencode.json after a single apply. M2 done.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```



---

## 完成时间

- `agentsync apply` 从单个规范 MCP 文件写入 `~/.claude.json` 和 `~/.config/opencode/opencode.json`。
- 单个规范子代理渲染为 `~/.claude/agents/<n>.md`（Claude 形状）和 `~/.config/opencode/agents/<n>.md`（带有 frontmatter munge 的 OpenCode 形状）； `tools`+`color` 的跳过显示在应用翻译报告中。
- 一项规范技能在 `~/.claude/skills/<n>/SKILL.md` 处写入一个文件（已删除重复数据），由 Claude 和 OpenCode 消耗。
- 预先存在的 `opencode.json` 中的 JSONC 注释在我们不接触的键的代理同步突变中仍然存在。
- 规范中的挂钩和 LSP 在翻译报告中生成显式的 `✗ skip(warn)` 条目。
- Linux/macos/windows 上的 CI 绿色。