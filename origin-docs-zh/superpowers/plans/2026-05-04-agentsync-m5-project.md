# agentsync M5 — 项目本地

> [`overview`](2026-05-04-agentsync-v1.0-overview.md) 中的约定。基于 M0–M4 构建。

**目标：** `.agentsync.toml` 来自 cwd 的直接解析；项目 IR 叠加层合并到基础 IR 上； `--project <slug>` 用于显式项目选择的标志；项目范围状态跟踪（状态键包括项目 slug）。

**架构：** 新的 `internal/project` 包。 `Detect()` 从 cwd 向上查找 `.agentsync.toml`。架构镜像 `~/.agentsync/` 的顶层（代理白名单、MCP、插件启用/禁用、内存导入）。叠加合并：项目条目替换具有相同id的用户条目；添加具有新 ID 的项目条目。 `state.Targets.Files` 和 `state.Targets.Keys` 键包含项目根路径（或用户范围的空字符串），因此漂移跟踪每个（范围、项目）。

---

## 文件



```
NEW:
internal/project/{project.go, project_test.go}

MODIFIED:
internal/source/loader.go      # accepts project overlay path; merges
internal/cli/apply.go          # walks-up by default; --project explicit override
internal/cli/status.go, diff.go, reconcile.go  # all gain --project
```



---

## 任务 1：`.agentsync.toml` 架构 + 预演

`internal/project/project.go`：



```go
// Package project handles project-scope overlays: .agentsync.toml at a repo
// root, walk-up discovery from cwd, and merge against base canonical model.
package project

import (
    "errors"
    "os"
    "path/filepath"

    "github.com/pelletier/go-toml/v2"
    "github.com/spxrogers/agentsync/internal/source"
)

const MarkerFile = ".agentsync.toml"

// Marker is the parsed contents of a project's .agentsync.toml.
type Marker struct {
    Path     string                            // absolute path of the marker file
    Root     string                            // dirname of Path
    Agents   []string                          `toml:"agents,omitempty"`
    MCP      []source.MCPServerSpec            `toml:"mcp,omitempty"`
    Plugins  ProjectPluginsSection             `toml:"plugins,omitempty"`
    Memory   ProjectMemorySection              `toml:"memory,omitempty"`
}

type ProjectPluginsSection struct {
    Disabled []string `toml:"disabled,omitempty"`
    Enabled  []string `toml:"enabled,omitempty"`
}

type ProjectMemorySection struct {
    Import []string `toml:"import,omitempty"` // project-relative paths
}

// Discover walks up from cwd looking for MarkerFile. Returns (nil, nil) if
// not found. Returns error on read or parse failure.
func Discover(cwd string) (*Marker, error) {
    dir := cwd
    for {
        candidate := filepath.Join(dir, MarkerFile)
        if data, err := os.ReadFile(candidate); err == nil {
            var m Marker
            if err := toml.Unmarshal(data, &m); err != nil {
                return nil, err
            }
            m.Path = candidate
            m.Root = dir
            return &m, nil
        } else if !errors.Is(err, os.ErrNotExist) {
            return nil, err
        }
        parent := filepath.Dir(dir)
        if parent == dir {
            return nil, nil
        }
        dir = parent
    }
}
```



- [ ] **测试**



```go
func TestDiscover_FoundAtRoot(t *testing.T) {
    tmp := t.TempDir()
    deep := filepath.Join(tmp, "a", "b", "c")
    _ = os.MkdirAll(deep, 0o755)
    _ = os.WriteFile(filepath.Join(tmp, ".agentsync.toml"), []byte(`agents = ["claude"]`), 0o644)
    m, err := project.Discover(deep)
    if err != nil {
        t.Fatal(err)
    }
    if m == nil {
        t.Fatal("expected discovery")
    }
    if len(m.Agents) != 1 || m.Agents[0] != "claude" {
        t.Fatalf("agents = %v", m.Agents)
    }
}

func TestDiscover_NotFound(t *testing.T) {
    m, err := project.Discover(t.TempDir())
    if err != nil {
        t.Fatal(err)
    }
    if m != nil {
        t.Fatalf("expected nil marker")
    }
}
```



承诺。

---

## 任务 2：叠加合并

在`internal/project/project.go`中：



```go
// Merge applies the project marker on top of base. Returns a new Canonical.
//   - Agents allowlist on Marker filters base.Config.Agents to only those
//     listed (intersect with enabled). Empty list = use all enabled.
//   - MCP entries on Marker are appended; collisions on .ID replace base.
//   - Plugins.Disabled removes plugins from base by ID.
//   - Plugins.Enabled is reserved for v1.x (currently a no-op since plugins
//     are enabled-by-default).
//   - Memory.Import paths are read relative to Marker.Root and appended to
//     the base memory body (separated by a single blank line).
func Merge(base source.Canonical, m *Marker) source.Canonical {
    if m == nil {
        return base
    }
    out := base // shallow copy is OK; we replace slices below

    // Agents filter
    if len(m.Agents) > 0 {
        allow := map[string]bool{}
        for _, a := range m.Agents {
            allow[a] = true
        }
        filtered := map[string]source.Agent{}
        for name, ag := range base.Config.Agents {
            if allow[name] {
                filtered[name] = ag
            }
        }
        out.Config.Agents = filtered
    }

    // MCP overlay
    byID := map[string]int{}
    for i, srv := range out.MCPServers {
        byID[srv.ID] = i
    }
    for _, spec := range m.MCP {
        // Marker.MCP entries are MCPServerSpec; need an ID. v1: treat the
        // first entry's command as the implicit id basis. Actually, our
        // MCP block in marker is `[[mcp]]` with explicit id field — adjust
        // schema to wrap MCPServer not MCPServerSpec. Update Marker:
    }

    // Plugins.Disabled
    if len(m.Plugins.Disabled) > 0 {
        block := map[string]bool{}
        for _, id := range m.Plugins.Disabled {
            block[id] = true
        }
        var kept []source.Plugin
        for _, p := range out.Plugins {
            if !block[p.ID] {
                kept = append(kept, p)
            }
        }
        out.Plugins = kept
    }

    // Memory imports
    if len(m.Memory.Import) > 0 {
        body := out.Memory.Body
        for _, rel := range m.Memory.Import {
            data, err := os.ReadFile(filepath.Join(m.Root, rel))
            if err != nil {
                continue
            }
            if body != "" && !strings.HasSuffix(body, "\n") {
                body += "\n"
            }
            body += "\n" + string(data)
        }
        out.Memory.Body = body
    }
    return out
}
```



（MCP 的标记架构需要携带 `id` 字段。更新 `Marker`:)



```go
type ProjectMCP struct {
    ID     string                  `toml:"id"`
    Server source.MCPServerSpec    `toml:"server"`
}

type Marker struct {
    Path    string
    Root    string
    Agents  []string                  `toml:"agents,omitempty"`
    MCP     []ProjectMCP              `toml:"mcp,omitempty"` // [[mcp]] tables
    Plugins ProjectPluginsSection     `toml:"plugins,omitempty"`
    Memory  ProjectMemorySection      `toml:"memory,omitempty"`
}
```



设计规范中的标记示例显示：



```toml
[[mcp]]
id      = "company-api"
type    = "stdio"
command = "npx"
args    = ["-y", "@company/mcp"]
[mcp.env]
COMPANY_TOKEN = "${secret:company.api_token}"
```



这是 `[[mcp]]` 表数组，其中规范字段内联在表的顶层（不在 `[server]` 下）。调整加载器：`ProjectMCP` 解析平面形状并映射到 `MCPServerSpec`。使用真实的 `.agentsync.toml` 测试叠加。犯罪。

---

## 任务 3：将项目连接到 CLI

- 将 `internal/cli/apply.go`、`status.go`、`diff.go`、`reconcile.go` 修改为：
  - 添加 `--project <path>` 标志（默认为直接）。
  - 通过 `project.Discover(cwd)` 发现项目标记（或使用 `--project` 覆盖）。
  - 当找到标记时，将 `adapter.ScopeProject` 和项目根传递给渲染/计划/应用。
  - 更新 `RecordOpsState` 调用以将项目根包含在状态键中。

- [ ] **集成测试**



```go
func TestIntegration_M5_ProjectOverlay(t *testing.T) {
    tmpHome := t.TempDir()
    project := t.TempDir()
    env := map[string]string{
        "AGENTSYNC_TARGET_ROOT": tmpHome,
        "PWD":                   project, // some shells; for us: chdir
    }

    _, _ = runCLI(t, env, "init")
    _, _ = runCLI(t, env, "agent", "add", "claude")

    _ = os.WriteFile(filepath.Join(project, ".agentsync.toml"), []byte(`
agents = ["claude"]

[[mcp]]
id      = "proj-mcp"
[mcp.server]
type    = "stdio"
command = "npx"
args    = ["-y", "@proj/mcp"]
`), 0o644)

    // chdir into project
    cwd, _ := os.Getwd()
    defer os.Chdir(cwd)
    _ = os.Chdir(project)

    if _, err := runCLI(t, env, "apply"); err != nil {
        t.Fatal(err)
    }
    body, _ := os.ReadFile(filepath.Join(project, ".claude", "settings.json"))
    if !strings.Contains(string(body), "proj-mcp") {
        t.Fatalf("project-scope MCP not landed: %s", body)
    }
}
```



承诺。

---

## 完成时间

- `cd ~/repo && agentsync apply` 发现 `.agentsync.toml`、合并覆盖、在 `<repo>/.claude/` 下写入项目范围目标等。
- `agentsync apply --project /abs/path` 无需 cwd 依赖即可工作。
- 状态键消除了用户与项目的歧义（`~/.claude/settings.json` 与 `<repo>/.claude/settings.json` 上的漂移是独立跟踪的）。
- 标记 `agents = ["claude"]` 过滤器：只希望 Claude 不会意外写入该范围内的 OpenCode 的项目。
- `[[mcp]]` 标记中的表数组正确覆盖到基本规范中。
- Memory.Import 解析项目相关文件并将它们连接到渲染的内存中。
- CI 绿色。