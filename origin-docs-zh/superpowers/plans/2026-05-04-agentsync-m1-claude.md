# agentsync M1 — 克劳德代码适配器

> **对于代理工人：** 所需的子技能：使用超级能力：子代理驱动的开发（推荐）或超级能力：执行计划。 [`overview`](2026-05-04-agentsync-v1.0-overview.md#conventions-used-across-all-milestone-plans) 中的约定。基于 [M0 骨架](2026-05-04-agentsync-m0-skeleton.md) 构建。

**目标：** 第一个真正的适配器。实现`internal/adapter/claude`，涵盖所有 7 个组件类型（MCP、内存、技能、子代理、斜杠命令、钩子、LSP）。渲染+摄取往返。写入 `~/.claude/settings.json` 和 `~/.claude.json` 的设置使用**每键合并**，因此外键（Claude 自己写入，用户手动编辑）不会受到影响。替换注册表中 `claude` 的 NoopAdapter。

**架构：** 一个包 (`internal/adapter/claude`) 每个关注点包含一个文件。适配器使用 `source.Canonical` 并发出 `[]adapter.FileOp` 准备进行原子写入。 Settings/`.claude.json` 合并使用基于 JSON 指针的 AST 遍历：agentsync 仅触及它之前写入的键（在 M0 的 `state.Keys` 中跟踪）；外键流经不变。

**技术堆栈：** Stdlib `encoding/json` 用于 Claude 的严格 JSON 文件； `pelletier/go-toml/v2` 已从 M0 出售； `os/exec` 检测 Claude 安装。

---

## 在此里程碑中创建的文件



```
internal/adapter/claude/
├── claude.go              # Adapter struct, Name(), Capabilities(), Detect()
├── paths.go               # Resolves ~/.claude/, settings.json, .claude.json paths
├── render.go              # Render(): canonical -> []FileOp
├── render_test.go
├── ingest.go              # Ingest(): disk -> source.Canonical
├── ingest_test.go
├── apply.go               # Apply([]FileOp) -> writes via iox.AtomicWrite
├── apply_test.go
├── settings.go            # JSON key-level merge for settings.json + .claude.json
├── settings_test.go
├── skill.go               # SKILL.md frontmatter + body passthrough
├── skill_test.go
├── memory.go              # CLAUDE.md rendering from canonical Memory
├── subagent.go            # agents/<name>.md from canonical
├── command.go             # commands/<name>.md from canonical
├── hook.go                # hooks JSON appended to settings.json
├── lsp.go                 # lspServers JSON appended to settings.json
├── frontmatter.go         # YAML frontmatter parser shared by skill/subagent
└── frontmatter_test.go
```



加上修改：
- `internal/cli/registry_internal.go` — 连接 `claude.New()` 作为“claude”名字
- `testdata/claude/<case>/{source,expected}` — 黄金赛程

---

## 任务 1：`claude.Adapter` 骨架 + `Detect()`

**文件：**
- 创建：`internal/adapter/claude/{claude.go,paths.go}`
- 创建：`internal/adapter/claude/claude_test.go`

`Adapter` 结构+路径帮助器。如果 `~/.claude/` 存在或 `claude` 在 PATH 上，则 `Detect()` 返回 true。

- [ ] **步骤 1.1：编写失败的测试**

`internal/adapter/claude/claude_test.go`：



```go
package claude_test

import (
    "os"
    "path/filepath"
    "testing"

    "github.com/spxrogers/agentsync/internal/adapter"
    "github.com/spxrogers/agentsync/internal/adapter/claude"
)

func TestAdapter_Identity(t *testing.T) {
    a := claude.New(claude.Options{TargetRoot: t.TempDir()})
    if a.Name() != "claude" {
        t.Fatalf("Name = %q", a.Name())
    }
    if a.Capabilities() == 0 {
        t.Fatalf("expected non-zero capabilities")
    }
    var _ adapter.Adapter = a
}

func TestAdapter_DetectsHomeDir(t *testing.T) {
    tmp := t.TempDir()
    _ = os.MkdirAll(filepath.Join(tmp, ".claude"), 0o755)

    a := claude.New(claude.Options{TargetRoot: tmp})
    ok, err := a.Detect()
    if err != nil {
        t.Fatal(err)
    }
    if !ok {
        t.Fatalf("expected Detect=true when ~/.claude/ exists")
    }
}

func TestAdapter_DetectsAbsentReturnsFalse(t *testing.T) {
    a := claude.New(claude.Options{TargetRoot: t.TempDir()})
    ok, _ := a.Detect()
    if ok {
        t.Fatalf("expected Detect=false on empty home")
    }
}
```



- [ ] **步骤1.2：运行；验证未定义**

`undefined: claude.New, claude.Options`

- [ ] **步骤 1.3：实施**

`internal/adapter/claude/paths.go`：



```go
package claude

import "path/filepath"

// Paths resolves the destination paths for a given (scope, project, target-root).
type Paths struct {
    Home            string // ~/.claude
    Settings        string // ~/.claude/settings.json
    DotClaude       string // ~/.claude.json (mcpServers + plugin enables live here)
    SkillsDir       string // ~/.claude/skills
    AgentsDir       string // ~/.claude/agents
    CommandsDir     string // ~/.claude/commands
    Memory          string // ~/.claude/CLAUDE.md (user scope) or <proj>/CLAUDE.md (project scope)
    PluginsCacheDir string // ~/.claude/plugins/cache
}

func ResolvePaths(targetRoot, project string, projectScope bool) Paths {
    home := filepath.Join(targetRoot, ".claude")
    p := Paths{
        Home:            home,
        Settings:        filepath.Join(home, "settings.json"),
        DotClaude:       filepath.Join(targetRoot, ".claude.json"),
        SkillsDir:       filepath.Join(home, "skills"),
        AgentsDir:       filepath.Join(home, "agents"),
        CommandsDir:     filepath.Join(home, "commands"),
        Memory:          filepath.Join(home, "CLAUDE.md"),
        PluginsCacheDir: filepath.Join(home, "plugins", "cache"),
    }
    if projectScope && project != "" {
        // project-scope settings live under <project>/.claude/
        projHome := filepath.Join(project, ".claude")
        p.Home = projHome
        p.Settings = filepath.Join(projHome, "settings.json")
        p.SkillsDir = filepath.Join(projHome, "skills")
        p.AgentsDir = filepath.Join(projHome, "agents")
        p.CommandsDir = filepath.Join(projHome, "commands")
        p.Memory = filepath.Join(project, "CLAUDE.md")
    }
    return p
}
```



`internal/adapter/claude/claude.go`：



```go
// Package claude implements the Claude Code adapter for agentsync.
package claude

import (
    "os"
    "os/exec"

    "github.com/spxrogers/agentsync/internal/adapter"
)

// Options configure the adapter at construction.
type Options struct {
    TargetRoot string // honors AGENTSYNC_TARGET_ROOT (real "/Users/x" in production)
}

// Adapter implements adapter.Adapter for Claude Code.
type Adapter struct {
    opts Options
}

// New constructs a Claude adapter.
func New(opts Options) *Adapter { return &Adapter{opts: opts} }

func (a *Adapter) Name() string { return "claude" }

func (a *Adapter) Capabilities() adapter.Capability {
    return adapter.CapMCP | adapter.CapMemory | adapter.CapSkill |
        adapter.CapSubagent | adapter.CapCommand | adapter.CapHook | adapter.CapLSP
}

func (a *Adapter) Detect() (bool, error) {
    p := ResolvePaths(a.opts.TargetRoot, "", false)
    if _, err := os.Stat(p.Home); err == nil {
        return true, nil
    }
    if _, err := exec.LookPath("claude"); err == nil {
        return true, nil
    }
    return false, nil
}
```



- [ ] **步骤1.4：运行；验证通过；提交**



```bash
go test -race ./internal/adapter/claude/...
```





```bash
git add internal/adapter/claude
git commit -m "$(cat <<'EOF'
feat(adapter/claude): adapter skeleton + Detect()

Capabilities cover all 7 component types (full-spectrum). Detect prefers
filesystem evidence (~/.claude exists) and falls back to PATH.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```



---

## 任务 2：`settings.json` 的每键 JSON 合并

**文件：**
- 创建：`internal/adapter/claude/settings.go`、`internal/adapter/claude/settings_test.go`

M1最硬的单块。 Claude 的 `settings.json` 在 Claude 代码本身、用户的手动编辑和 agentsync 之间共享。 agentsync 拥有特定的 JSON 指针（例如 `$.mcpServers.<id>`、`$.hooks.<event>[*]`、`$.lspServers.<id>`）；其他一切都必须保持不变。

**合同：**
- `MergeKeys(existing, ours map[string]any, ownedPointers []string) (merged map[string]any, kept, removed []string)`
- `existing` = 磁盘文件的原始解析
- `ours` = agentsync 想要写入的键（也作为映射）
- `ownedPointers` = agentsync *最后应用*写入的 JSON 指针 (`/mcpServers/github`, `/hooks/PreToolUse/0`) — 这些是可回收的
- 返回合并映射：保留外键；已拥有但现在不存在的指针已被删除；来自 `ours` 的新代理同步指针被覆盖。

- [ ] **步骤 2.1：测试**

`internal/adapter/claude/settings_test.go`：



```go
package claude_test

import (
    "encoding/json"
    "reflect"
    "testing"

    "github.com/spxrogers/agentsync/internal/adapter/claude"
)

func decode(s string) map[string]any {
    var m map[string]any
    _ = json.Unmarshal([]byte(s), &m)
    return m
}

func TestMergeKeys_NewKey(t *testing.T) {
    existing := decode(`{"foreign": {"keep": true}}`)
    ours := decode(`{"mcpServers": {"github": {"command": "npx"}}}`)

    merged, _, _ := claude.MergeKeys(existing, ours, nil)
    if _, ok := merged["foreign"]; !ok {
        t.Fatalf("foreign key dropped: %+v", merged)
    }
    if _, ok := merged["mcpServers"].(map[string]any)["github"]; !ok {
        t.Fatalf("ours not added: %+v", merged)
    }
}

func TestMergeKeys_ForeignKeyAtSameLevelPreserved(t *testing.T) {
    existing := decode(`{"mcpServers": {"foreign-server": {"command": "x"}}}`)
    ours := decode(`{"mcpServers": {"github": {"command": "npx"}}}`)

    merged, _, _ := claude.MergeKeys(existing, ours, nil)
    s := merged["mcpServers"].(map[string]any)
    if _, ok := s["foreign-server"]; !ok {
        t.Fatalf("sibling foreign mcpServers entry dropped: %+v", s)
    }
    if _, ok := s["github"]; !ok {
        t.Fatalf("our entry missing: %+v", s)
    }
}

func TestMergeKeys_OrphanRemoval(t *testing.T) {
    existing := decode(`{"mcpServers": {"github": {"command": "old"}, "stale": {"command": "x"}}}`)
    ours := decode(`{"mcpServers": {"github": {"command": "npx"}}}`)
    owned := []string{"/mcpServers/github", "/mcpServers/stale"} // both are ours per state

    merged, _, removed := claude.MergeKeys(existing, ours, owned)

    if len(removed) != 1 || removed[0] != "/mcpServers/stale" {
        t.Fatalf("expected /mcpServers/stale removed, got %v", removed)
    }
    s := merged["mcpServers"].(map[string]any)
    if _, ok := s["stale"]; ok {
        t.Fatalf("stale should be deleted: %+v", s)
    }
    if reflect.DeepEqual(s["github"].(map[string]any)["command"], "old") {
        t.Fatalf("github not updated to new value: %+v", s["github"])
    }
}

func TestMergeKeys_ForeignNotInOwnedListPreserved(t *testing.T) {
    existing := decode(`{"mcpServers": {"foreign": {"command": "x"}}}`)
    ours := decode(`{}`) // we removed everything
    owned := []string{"/mcpServers/old-mine"}

    merged, _, removed := claude.MergeKeys(existing, ours, owned)
    s := merged["mcpServers"].(map[string]any)
    if _, ok := s["foreign"]; !ok {
        t.Fatalf("foreign mcpServers entry must survive: %+v", s)
    }
    if len(removed) != 0 {
        t.Fatalf("we owned old-mine but it wasn't in existing; nothing to remove. Got %v", removed)
    }
}
```



- [ ] **步骤2.2：运行；验证未定义**

- [ ] **步骤 2.3：实施**

`internal/adapter/claude/settings.go`：



```go
package claude

import (
    "strings"
)

// MergeKeys merges ours into existing, removing ownedPointers that are no
// longer in ours. Returns the merged map plus diagnostic lists.
//
// kept: pointers from ownedPointers that are still present in ours.
// removed: pointers from ownedPointers that are absent from ours and were
// deleted from existing.
//
// JSON pointer syntax: leading "/", "/" separated path segments. RFC 6901
// escapes ("~0" for "~", "~1" for "/") are supported.
func MergeKeys(existing, ours map[string]any, ownedPointers []string) (map[string]any, []string, []string) {
    merged := deepCopyMap(existing)
    if merged == nil {
        merged = map[string]any{}
    }

    // Step 1: overlay ours onto merged
    for k, v := range ours {
        switch ev := merged[k].(type) {
        case map[string]any:
            if vv, ok := v.(map[string]any); ok {
                merged[k] = mergeMaps(ev, vv)
                continue
            }
        }
        merged[k] = v
    }

    // Step 2: walk ownedPointers; if a pointer is no longer present in `ours`,
    // delete it from merged. If still present, mark kept.
    var kept, removed []string
    for _, p := range ownedPointers {
        if pointerExists(ours, p) {
            kept = append(kept, p)
            continue
        }
        if pointerExists(merged, p) {
            deletePointer(merged, p)
            removed = append(removed, p)
        }
    }
    return merged, kept, removed
}

func mergeMaps(a, b map[string]any) map[string]any {
    out := deepCopyMap(a)
    for k, v := range b {
        switch existing := out[k].(type) {
        case map[string]any:
            if vv, ok := v.(map[string]any); ok {
                out[k] = mergeMaps(existing, vv)
                continue
            }
        }
        out[k] = v
    }
    return out
}

func deepCopyMap(m map[string]any) map[string]any {
    if m == nil {
        return nil
    }
    out := make(map[string]any, len(m))
    for k, v := range m {
        if mm, ok := v.(map[string]any); ok {
            out[k] = deepCopyMap(mm)
        } else {
            out[k] = v
        }
    }
    return out
}

func pointerExists(m map[string]any, ptr string) bool {
    parts := splitPointer(ptr)
    var cur any = m
    for _, p := range parts {
        mp, ok := cur.(map[string]any)
        if !ok {
            return false
        }
        cur, ok = mp[p]
        if !ok {
            return false
        }
    }
    return true
}

func deletePointer(m map[string]any, ptr string) {
    parts := splitPointer(ptr)
    if len(parts) == 0 {
        return
    }
    cur := m
    for i, p := range parts {
        if i == len(parts)-1 {
            delete(cur, p)
            return
        }
        next, ok := cur[p].(map[string]any)
        if !ok {
            return
        }
        cur = next
    }
}

func splitPointer(ptr string) []string {
    ptr = strings.TrimPrefix(ptr, "/")
    if ptr == "" {
        return nil
    }
    raw := strings.Split(ptr, "/")
    out := make([]string, len(raw))
    for i, s := range raw {
        s = strings.ReplaceAll(s, "~1", "/")
        s = strings.ReplaceAll(s, "~0", "~")
        out[i] = s
    }
    return out
}
```



（注意：此 v1 实现处理对象键，但不处理指针中的数组索引 - 对于对象 `mcpServers`/`lspServers` 来说足够了。Claude 下的 Hooks 结构是 `{"hooks":{"PreToolUse":[...]}}` 数组；M1 钩子所有权位于 *event-name* 级别 (`/hooks/PreToolUse`)，而不是数组索引粒度，因此更简单的实现可以工作。如果出现偏差，可以在以后的里程碑中添加数组索引支持跨数组变得很痛苦。）

- [ ] **步骤2.4：运行；验证通过；提交**



```bash
go test -race ./internal/adapter/claude/...
```





```bash
git add internal/adapter/claude
git commit -m "$(cat <<'EOF'
feat(adapter/claude): per-key JSON merge for settings.json + .claude.json

Foreign keys survive verbatim. ownedPointers list (read from
state.Keys at apply-time) controls reclamation: keys we wrote last
apply but no longer want are deleted; foreign keys at the same level
preserved. Object-pointer support only; array-index drift can be
deferred to a future milestone since hooks track at event-name level.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```



---

## 任务 3：YAML frontmatter 解析器

**文件：**
- 创建：`internal/adapter/claude/frontmatter.go`、`internal/adapter/claude/frontmatter_test.go`

技能、子代理和斜杠命令都使用约定 `---\n<YAML>\n---\n<body>`。需要一个小型解析器，将 frontmatter 作为 `map[string]any` 返回，将 body 作为字符串返回。

- [ ] **步骤 3.1：添加 YAML 依赖项**



```bash
go get sigs.k8s.io/yaml@latest
```



（`sigs.k8s.io/yaml` 首先通过转换为 JSON 来解析 YAML，因此它直接返回 `map[string]any` — 这正是我们需要往返而不丢失键的形状。）

- [ ] **步骤 3.2：测试**

`internal/adapter/claude/frontmatter_test.go`：



```go
package claude_test

import (
    "testing"

    "github.com/spxrogers/agentsync/internal/adapter/claude"
)

func TestParseFrontmatter_Standard(t *testing.T) {
    input := []byte(`---
name: my-skill
description: Does the thing
disable-model-invocation: true
---
This is the body.

Multiple lines.
`)
    fm, body, err := claude.ParseFrontmatter(input)
    if err != nil {
        t.Fatal(err)
    }
    if fm["name"] != "my-skill" {
        t.Fatalf("name = %v", fm["name"])
    }
    if fm["disable-model-invocation"] != true {
        t.Fatalf("disable-model-invocation = %v", fm["disable-model-invocation"])
    }
    if body != "This is the body.\n\nMultiple lines.\n" {
        t.Fatalf("body mismatch: %q", body)
    }
}

func TestParseFrontmatter_NoFrontmatter(t *testing.T) {
    fm, body, err := claude.ParseFrontmatter([]byte("plain markdown\n"))
    if err != nil {
        t.Fatal(err)
    }
    if len(fm) != 0 {
        t.Fatalf("fm should be empty: %+v", fm)
    }
    if body != "plain markdown\n" {
        t.Fatalf("body = %q", body)
    }
}

func TestEncodeFrontmatter_Roundtrip(t *testing.T) {
    fm := map[string]any{"name": "x", "description": "y"}
    out, err := claude.EncodeFrontmatter(fm, "body")
    if err != nil {
        t.Fatal(err)
    }
    fm2, body2, err := claude.ParseFrontmatter(out)
    if err != nil {
        t.Fatal(err)
    }
    if fm2["name"] != "x" || fm2["description"] != "y" {
        t.Fatalf("roundtrip lost data: %+v", fm2)
    }
    if body2 != "body" {
        t.Fatalf("body = %q", body2)
    }
}
```



- [ ] **步骤 3.3：实施**

`internal/adapter/claude/frontmatter.go`：



```go
package claude

import (
    "bytes"
    "fmt"
    "strings"

    "sigs.k8s.io/yaml"
)

// ParseFrontmatter extracts the YAML frontmatter and the markdown body. If
// the input doesn't begin with "---\n", returns an empty map and the entire
// input as body.
func ParseFrontmatter(data []byte) (map[string]any, string, error) {
    if !bytes.HasPrefix(data, []byte("---\n")) {
        return map[string]any{}, string(data), nil
    }
    rest := data[len("---\n"):]
    end := bytes.Index(rest, []byte("\n---\n"))
    if end < 0 {
        return nil, "", fmt.Errorf("unterminated frontmatter")
    }
    yml := rest[:end]
    body := rest[end+len("\n---\n"):]

    var fm map[string]any
    if err := yaml.Unmarshal(yml, &fm); err != nil {
        return nil, "", fmt.Errorf("parse yaml frontmatter: %w", err)
    }
    if fm == nil {
        fm = map[string]any{}
    }
    return fm, string(body), nil
}

// EncodeFrontmatter writes "---\n<yaml>\n---\n<body>" with the keys in fm.
// fm is empty: returns just the body.
func EncodeFrontmatter(fm map[string]any, body string) ([]byte, error) {
    if len(fm) == 0 {
        return []byte(body), nil
    }
    yml, err := yaml.Marshal(fm)
    if err != nil {
        return nil, fmt.Errorf("encode yaml: %w", err)
    }
    var buf strings.Builder
    buf.WriteString("---\n")
    buf.Write(yml)
    buf.WriteString("---\n")
    buf.WriteString(body)
    return []byte(buf.String()), nil
}
```



- [ ] **步骤3.4：运行；提交**



```bash
go test -race ./internal/adapter/claude/...
git add go.mod go.sum internal/adapter/claude
git commit -m "$(cat <<'EOF'
feat(adapter/claude): YAML frontmatter parse/encode round-trip

Used by skill/subagent/command. sigs.k8s.io/yaml goes via JSON so
parsed frontmatter is map[string]any (round-trip safe even for keys we
don't recognize, like disable-model-invocation).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```



---

## 任务 4：更新 `source.Skill` 加载器以解析 frontmatter

**文件：**
- 修改：`internal/source/loader.go`（扩展`loadSkills`）
- 修改：`internal/source/loader_test.go`（添加 frontmatter 断言）

在M0中我们将技能体保留为一根大弦。现在M1需要单独的frontmatter图。

- [ ] **步骤 4.1：测试添加**

附加到 `internal/source/loader_test.go`：



```go
func TestLoad_SkillFrontmatter(t *testing.T) {
    fs := afero.NewMemMapFs()
    _ = afero.WriteFile(fs, "/home/.agentsync/skills/foo/SKILL.md", []byte(`---
name: foo
description: Test skill
---
Body.
`), 0o644)

    c, err := source.Load(fs, "/home/.agentsync")
    if err != nil {
        t.Fatal(err)
    }
    if len(c.Skills) != 1 {
        t.Fatalf("skills = %d", len(c.Skills))
    }
    s := c.Skills[0]
    if s.Frontmatter["name"] != "foo" {
        t.Fatalf("frontmatter name = %v", s.Frontmatter["name"])
    }
    if s.Body != "Body.\n" {
        t.Fatalf("body = %q", s.Body)
    }
}
```



- [ ] **步骤 4.2：更新 `loadSkills`**

在 `internal/source/loader.go` 中，替换现有的 `loadSkills` 正文：



```go
func loadSkills(fs afero.Fs, home string) ([]Skill, error) {
    dir := filepath.Join(home, "skills")
    entries, err := afero.ReadDir(fs, dir)
    if err != nil {
        if errors.Is(err, os.ErrNotExist) {
            return nil, nil
        }
        return nil, fmt.Errorf("read %s: %w", dir, err)
    }
    var out []Skill
    for _, e := range entries {
        if !e.IsDir() {
            continue
        }
        raw, err := afero.ReadFile(fs, filepath.Join(dir, e.Name(), "SKILL.md"))
        if err != nil {
            if errors.Is(err, os.ErrNotExist) {
                continue
            }
            return nil, fmt.Errorf("read SKILL.md for %s: %w", e.Name(), err)
        }
        fm, body, err := parseFrontmatter(raw)
        if err != nil {
            return nil, fmt.Errorf("parse %s: %w", e.Name(), err)
        }
        out = append(out, Skill{Name: e.Name(), Frontmatter: fm, Body: body})
    }
    return out, nil
}
```



…并向 `source` 包添加一个小的 `parseFrontmatter` 帮助程序（从 M1 任务 3 镜像 - 保存在 `source` 中以避免源包导入 claude 适配器）。添加到 `loader.go`：



```go
import (
    // ...existing imports...
    "bytes"
    "sigs.k8s.io/yaml"
)

func parseFrontmatter(data []byte) (map[string]any, string, error) {
    if !bytes.HasPrefix(data, []byte("---\n")) {
        return map[string]any{}, string(data), nil
    }
    rest := data[len("---\n"):]
    end := bytes.Index(rest, []byte("\n---\n"))
    if end < 0 {
        return nil, "", fmt.Errorf("unterminated frontmatter")
    }
    yml := rest[:end]
    body := rest[end+len("\n---\n"):]
    var fm map[string]any
    if err := yaml.Unmarshal(yml, &fm); err != nil {
        return nil, "", fmt.Errorf("parse yaml frontmatter: %w", err)
    }
    if fm == nil {
        fm = map[string]any{}
    }
    return fm, string(body), nil
}
```



- [ ] **步骤4.3：运行；提交**



```bash
go test -race ./internal/source/...
git add internal/source
git commit -m "$(cat <<'EOF'
feat(source): parse skill frontmatter into Skill.Frontmatter

Frontmatter is map[string]any so unknown keys (disable-model-invocation,
custom vendor extensions) round-trip without loss.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```



---

## 任务 5：`Render()` MCP 服务器

**文件：**
- 创建：`internal/adapter/claude/render.go`、`internal/adapter/claude/render_test.go`

对于规范中的每个 `MCPServer`，在 `/mcpServers/<id>` 下发出一个 settings.json 键（对于用户范围 MCP，发出 `~/.claude.json` `/mcpServers/<id>` — Claude 会读取两者，但 `~/.claude.json` 是 MCP 的用户范围权限）。

- [ ] **步骤 5.1：测试**

`internal/adapter/claude/render_test.go`：



```go
package claude_test

import (
    "encoding/json"
    "strings"
    "testing"

    "github.com/spxrogers/agentsync/internal/adapter"
    "github.com/spxrogers/agentsync/internal/adapter/claude"
    "github.com/spxrogers/agentsync/internal/source"
)

func TestRender_MCP_UserScope(t *testing.T) {
    enabled := true
    c := source.Canonical{
        MCPServers: []source.MCPServer{{
            ID: "github",
            Server: source.MCPServerSpec{
                Type:    "stdio",
                Command: "npx",
                Args:    []string{"-y", "@modelcontextprotocol/server-github"},
                Env:     map[string]string{"GITHUB_TOKEN": "xyz"},
                Agents:  []string{"*"},
                Enabled: &enabled,
            },
        }},
    }
    a := claude.New(claude.Options{TargetRoot: t.TempDir()})
    ops, skips, err := a.Render(c, adapter.ScopeUser, "")
    if err != nil {
        t.Fatal(err)
    }
    if len(skips) != 0 {
        t.Fatalf("unexpected skips: %+v", skips)
    }
    // The MCP write goes into .claude.json under user scope.
    var found bool
    for _, op := range ops {
        if strings.HasSuffix(op.Path, ".claude.json") {
            found = true
            var got map[string]any
            if err := json.Unmarshal(op.Content, &got); err != nil {
                t.Fatalf("not valid json: %v", err)
            }
            srv := got["mcpServers"].(map[string]any)["github"].(map[string]any)
            if srv["command"] != "npx" {
                t.Fatalf("command = %v", srv["command"])
            }
        }
    }
    if !found {
        t.Fatalf(".claude.json op not produced: %+v", ops)
    }
}

func TestRender_MCP_AgentsAllowlist(t *testing.T) {
    enabled := true
    c := source.Canonical{
        MCPServers: []source.MCPServer{{
            ID: "private",
            Server: source.MCPServerSpec{
                Type:    "stdio",
                Command: "x",
                Agents:  []string{"opencode"}, // claude not in list
                Enabled: &enabled,
            },
        }},
    }
    a := claude.New(claude.Options{TargetRoot: t.TempDir()})
    ops, _, err := a.Render(c, adapter.ScopeUser, "")
    if err != nil {
        t.Fatal(err)
    }
    // No .claude.json write because the server isn't targeted at claude.
    for _, op := range ops {
        if strings.HasSuffix(op.Path, ".claude.json") {
            t.Fatalf("expected no .claude.json op when allowlist excludes claude")
        }
    }
}
```



- [ ] **步骤 5.2：渲染骨架**

`internal/adapter/claude/render.go`：



```go
package claude

import (
    "encoding/json"
    "fmt"

    "github.com/spxrogers/agentsync/internal/adapter"
    "github.com/spxrogers/agentsync/internal/source"
)

// Render produces the full set of FileOps for a given canonical model.
// Pure function: returns the same output for the same input.
func (a *Adapter) Render(c source.Canonical, scope adapter.Scope, project string) ([]adapter.FileOp, []adapter.Skip, error) {
    paths := ResolvePaths(a.opts.TargetRoot, project, scope == adapter.ScopeProject)

    var ops []adapter.FileOp
    var skips []adapter.Skip

    // 1. MCP -> .claude.json (user) or settings.json (project)
    if mcpOps, err := a.renderMCP(c, paths, scope); err != nil {
        return nil, nil, err
    } else {
        ops = append(ops, mcpOps...)
    }

    // 2-7: implemented in Tasks 6-11.

    return ops, skips, nil
}

func (a *Adapter) renderMCP(c source.Canonical, p Paths, scope adapter.Scope) ([]adapter.FileOp, error) {
    targeted := map[string]map[string]any{}
    for _, m := range c.MCPServers {
        if m.Server.Enabled != nil && !*m.Server.Enabled {
            continue
        }
        if !agentTargeted("claude", m.Server.Agents) {
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
        if len(m.Server.Headers) > 0 {
            spec["headers"] = m.Server.Headers
        }
        targeted[m.ID] = spec
    }
    if len(targeted) == 0 {
        return nil, nil
    }
    obj := map[string]any{"mcpServers": targeted}

    var dest, sourceID string
    if scope == adapter.ScopeProject {
        dest = p.Settings // project-scope settings.json holds mcpServers
    } else {
        dest = p.DotClaude
    }
    sourceID = "mcp/* (multiple)"

    body, err := json.MarshalIndent(obj, "", "  ")
    if err != nil {
        return nil, fmt.Errorf("marshal mcp: %w", err)
    }
    return []adapter.FileOp{{
        Action:   "write",
        Path:     dest,
        Content:  append(body, '\n'),
        Mode:     0o644,
        SourceID: sourceID,
    }}, nil
}

// agentTargeted reports whether agents allowlist includes us. Empty / nil
// list / "*" entry means everyone.
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



（注意：此初始渲染尚未“合并”到现有文件中。Apply（任务 12）从磁盘读取现有的 settings.json/.claude.json，使用渲染后的我们的值调用 MergeKeys，然后自动写入合并的结果。这就是使每个密钥所有权成为真实的原因。对于 Render 的合约（这是纯粹的），我们只需发出“我们的”有效负载；合并发生在 apply 中。）

…实际上该契约使 Render 变得不太干净：调用 Render 的下游消费者期望 FileOp.Content 是要写入的最终字节，而不是“你的一半”。更好的契约：渲染计算最终字节（内联合并磁盘状态）。这需要 Render 读取磁盘上的 settings.json。可以接受，因为无论如何渲染都需要 `targetRoot`。

更新合约：渲染读取磁盘中的共享文件（`settings.json`、`.claude.json`）并生成合并后内容。纯粹基于规范+磁盘输入。继续前行。

重构`renderMCP`：



```go
func (a *Adapter) renderMCP(c source.Canonical, p Paths, scope adapter.Scope, ownedKeys []string) ([]adapter.FileOp, error) {
    targeted := map[string]any{}
    // ...same loop building `targeted`...

    if len(targeted) == 0 && len(ownedKeys) == 0 {
        return nil, nil
    }

    var dest string
    if scope == adapter.ScopeProject {
        dest = p.Settings
    } else {
        dest = p.DotClaude
    }
    existing := readJSONFile(dest) // returns empty map if missing
    ours := map[string]any{"mcpServers": targeted}
    merged, _, _ := MergeKeys(existing, ours, ownedKeys)

    body, err := json.MarshalIndent(merged, "", "  ")
    if err != nil {
        return nil, fmt.Errorf("marshal mcp: %w", err)
    }
    return []adapter.FileOp{{
        Action:   "write",
        Path:     dest,
        Content:  append(body, '\n'),
        Mode:     0o644,
        SourceID: "mcp/* (multiple)",
    }}, nil
}

func readJSONFile(path string) map[string]any {
    data, err := os.ReadFile(path)
    if err != nil {
        return map[string]any{}
    }
    var m map[string]any
    _ = json.Unmarshal(data, &m)
    if m == nil {
        return map[string]any{}
    }
    return m
}
```



`ownedKeys` 通过有权访问状态存储的应用路径流入 Render — 因此 Render 的调用签名必须更改。更新 `internal/adapter/adapter.go` 中的 `Adapter.Render` 签名以接受拥有的密钥：

实际上一个更干净的解决方案：保持 `Render` 签名不变；渲染将“我们的”字节生成到一个单独的 FileOp 中，表示“合并目标”。应用管道（任务 12）使用状态进行合并。权衡：渲染不再纯粹（它不计算最终字节），但适配器接口保持干净。

决策：使用 `MergeStrategy` 字段扩展 `adapter.FileOp`。值：`"replace"`（整个文件写入）或`"merge-json-keys"`（应用并合并）。渲染设定策略；应用尊重它。

更新`internal/adapter/adapter.go`：



```go
type FileOp struct {
    Action        string
    Path          string
    Content       []byte
    Mode          uint32
    SourceID      string
    MergeStrategy string   // "replace" (default) | "merge-json-keys"
    OwnedKeys     []string // populated by Apply from state.Keys, not by Render
}
```



Render 发出带有 MergeStrategy 设置的 FileOp。 Apply 读取现有的磁盘文件、解析、调用 MergeKeys、原子写入。

这使 Render 保持纯粹，并将状态依赖性推向 Apply（这自然是状态依赖性的，因为它持续存在）。

申请签名：



```go
func (a *Adapter) Apply(ops []adapter.FileOp) error {
    for _, op := range ops {
        switch op.MergeStrategy {
        case "merge-json-keys":
            existing := readJSONFile(op.Path)
            var ours map[string]any
            _ = json.Unmarshal(op.Content, &ours)
            merged, _, _ := MergeKeys(existing, ours, op.OwnedKeys)
            body, _ := json.MarshalIndent(merged, "", "  ")
            return iox.AtomicWrite(op.Path, append(body, '\n'), os.FileMode(op.Mode))
        default:
            return iox.AtomicWrite(op.Path, op.Content, os.FileMode(op.Mode))
        }
    }
    return nil
}
```



`OwnedKeys` 由应用管道填充（在 M3/状态集成中）；对于 M1，Apply 实现可以与 `OwnedKeys=nil` 一起正常工作（没有孤立删除，但也没有错误）。

- [ ] **步骤 5.3：更新 render_test 以期望 MergeStrategy**

调整 `TestRender_MCP_UserScope` 以断言 `op.MergeStrategy == "merge-json-keys"`，并且呈现的内容是仅包含 `{"mcpServers": {...}}` 的有效 JSON。

- [ ] **步骤 5.4：将 `MergeStrategy` 和 `OwnedKeys` 添加到 `adapter.FileOp`**

- [ ] **步骤5.5：运行；提交**



```bash
go test -race ./internal/adapter/claude/... ./internal/adapter/...
git add internal/adapter
git commit -m "$(cat <<'EOF'
feat(adapter/claude): Render MCP servers with merge-json-keys strategy

FileOp gains MergeStrategy + OwnedKeys; Render emits "ours" payload, Apply
performs the merge against existing on-disk content. Owned-key tracking
wired through in Task 12.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```



---

## 任务 6：`Render()` 内存 (CLAUDE.md)

**文件：**
- 创建：`internal/adapter/claude/memory.go`
- 修改：`internal/adapter/claude/render.go`（调用renderMemory）
- 修改：`internal/adapter/claude/render_test.go`（测试内存渲染）

将 `memory/AGENTS.md` 正文与已解析的片段连接起来（将 `@import ./fragments/style.md` 替换为片段的内容）。写入 `~/.claude/CLAUDE.md`（用户）或 `<project>/CLAUDE.md`（项目）。

- [ ] **步骤 6.1：测试**



```go
func TestRender_Memory(t *testing.T) {
    c := source.Canonical{
        Memory: source.Memory{
            Body: "# Personal style\n\n@import ./fragments/style.md\n\nMore.\n",
            Fragments: map[string]string{
                "style.md": "Use semicolons.\n",
            },
        },
    }
    a := claude.New(claude.Options{TargetRoot: t.TempDir()})
    ops, _, err := a.Render(c, adapter.ScopeUser, "")
    if err != nil {
        t.Fatal(err)
    }
    var memOp *adapter.FileOp
    for i, op := range ops {
        if strings.HasSuffix(op.Path, "/CLAUDE.md") {
            memOp = &ops[i]
        }
    }
    if memOp == nil {
        t.Fatalf("no CLAUDE.md op")
    }
    if !strings.Contains(string(memOp.Content), "Use semicolons.") {
        t.Fatalf("fragment not inlined: %s", memOp.Content)
    }
    if strings.Contains(string(memOp.Content), "@import") {
        t.Fatalf("@import directive leaked: %s", memOp.Content)
    }
}
```



- [ ] **步骤 6.2：实施**

`internal/adapter/claude/memory.go`：



```go
package claude

import (
    "regexp"
    "strings"

    "github.com/spxrogers/agentsync/internal/adapter"
    "github.com/spxrogers/agentsync/internal/source"
)

var importRe = regexp.MustCompile(`(?m)^@import\s+\./fragments/(\S+)\s*$`)

func (a *Adapter) renderMemory(c source.Canonical, p Paths) ([]adapter.FileOp, error) {
    if c.Memory.Body == "" {
        return nil, nil
    }
    body := importRe.ReplaceAllStringFunc(c.Memory.Body, func(line string) string {
        m := importRe.FindStringSubmatch(line)
        if len(m) < 2 {
            return line
        }
        if frag, ok := c.Memory.Fragments[m[1]]; ok {
            return strings.TrimRight(frag, "\n")
        }
        // Unknown fragment; preserve line so the user notices.
        return line
    })
    return []adapter.FileOp{{
        Action:        "write",
        Path:          p.Memory,
        Content:       []byte(body),
        Mode:          0o644,
        SourceID:      "memory/AGENTS.md",
        MergeStrategy: "replace",
    }}, nil
}
```



连入 `Render()`：



```go
if memOps, err := a.renderMemory(c, paths); err != nil { return nil, nil, err } else { ops = append(ops, memOps...) }
```



- [ ] **步骤 6.3：提交**



```bash
git add internal/adapter/claude
git commit -m "feat(adapter/claude): render CLAUDE.md with @import resolution

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```



---

## 任务 7：`Render()` 技能（将 SKILL.md 写入 ~/.claude/skills/<name>/）

每个规范技能都会产生一个 FileOp，写入 `<SkillsDir>/<name>/SKILL.md`，并通过 `EncodeFrontmatter` 重新编码 frontmatter + body。

- [ ] **步骤 7.1：测试**



```go
func TestRender_Skills(t *testing.T) {
    c := source.Canonical{
        Skills: []source.Skill{{
            Name:        "review",
            Frontmatter: map[string]any{"name": "review", "description": "Review code"},
            Body:        "Do a code review.\n",
        }},
    }
    a := claude.New(claude.Options{TargetRoot: t.TempDir()})
    ops, _, _ := a.Render(c, adapter.ScopeUser, "")
    var found *adapter.FileOp
    for i, op := range ops {
        if strings.Contains(op.Path, "/skills/review/SKILL.md") {
            found = &ops[i]
        }
    }
    if found == nil {
        t.Fatalf("no SKILL.md op")
    }
    if !strings.HasPrefix(string(found.Content), "---\n") {
        t.Fatalf("missing frontmatter delimiter: %s", found.Content)
    }
}
```



- [ ] **步骤 7.2：实施**

`internal/adapter/claude/skill.go`：



```go
package claude

import (
    "fmt"
    "path/filepath"

    "github.com/spxrogers/agentsync/internal/adapter"
    "github.com/spxrogers/agentsync/internal/source"
)

func (a *Adapter) renderSkills(c source.Canonical, p Paths) ([]adapter.FileOp, error) {
    var ops []adapter.FileOp
    for _, s := range c.Skills {
        body, err := EncodeFrontmatter(s.Frontmatter, s.Body)
        if err != nil {
            return nil, fmt.Errorf("encode skill %s: %w", s.Name, err)
        }
        ops = append(ops, adapter.FileOp{
            Action:        "write",
            Path:          filepath.Join(p.SkillsDir, s.Name, "SKILL.md"),
            Content:       body,
            Mode:          0o644,
            SourceID:      filepath.Join("skills", s.Name, "SKILL.md"),
            MergeStrategy: "replace",
        })
    }
    return ops, nil
}
```



连入 `Render()`。承诺：



```bash
git commit -am "feat(adapter/claude): render skills (frontmatter passthrough)

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```



---

## 任务 8：`Render()` 子代理

Claude 的子代理住在 `~/.claude/agents/<name>.md`，其 frontmatter (`description`、`tools`、`model`、`color`)。与规范的类似技能类型相同，但单独存储。

将 `Subagents []Skill` 添加到 `source.Canonical` （因为它们共享 markdown+frontmatter 形状；Skill 是一个误导性的类型名称，但适配器可以消除歧义）。实际上 - 更好 - 为了清楚起见，向 `internal/source/schema.go` 添加一个不同的 `Subagent` 类型：



```go
type Subagent struct {
    Name        string
    Frontmatter map[string]any
    Body        string
}
```



…以及 `Canonical` 上的 `Subagents []Subagent`。装载机行走 `<home>/agents/<name>.md`。

- [ ] **步骤 8.1：更新源架构 + 加载程序**

将 `Subagent` 类型添加到 `internal/source/schema.go`。添加 `loadSubagents` 镜像 `loadSkills`，但读取 `<home>/agents/*.md`。在 `loader_test.go` 中进行测试。犯罪。

- [ ] **步骤 8.2：渲染**

`internal/adapter/claude/subagent.go`：



```go
package claude

import (
    "fmt"
    "path/filepath"

    "github.com/spxrogers/agentsync/internal/adapter"
    "github.com/spxrogers/agentsync/internal/source"
)

func (a *Adapter) renderSubagents(c source.Canonical, p Paths) ([]adapter.FileOp, error) {
    var ops []adapter.FileOp
    for _, s := range c.Subagents {
        body, err := EncodeFrontmatter(s.Frontmatter, s.Body)
        if err != nil {
            return nil, fmt.Errorf("encode subagent %s: %w", s.Name, err)
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
    return ops, nil
}
```



测试、连接到渲染、提交。

---

## 任务 9：`Render()` 斜线命令

在 `source.Canonical` 和 `<home>/commands/*.md` 上镜像 `Commands []Command` 的任务 8。路径：`<CommandsDir>/<name>.md`。 Frontmatter 传递。

测试、实施、连接、提交。

---

## 任务 10：`Render()` 挂钩 (settings.json `/hooks/<event>`)

钩子与 MCP 不同：它们进入 `settings.json` 而不是 `.claude.json`，并且结构是 `{"hooks": {"PreToolUse": [{matcher, hooks: [{type, command}]}]}}`。 agentsync 拥有每个事件的整个 `/hooks/<event>` 数组。

- [ ] **步骤 10.1：将 `Hooks` 添加到 `source.Canonical`**



```go
type Hook struct {
    Event   string  // e.g. "PreToolUse"
    Matcher string  // glob/regex string
    Type    string  // "command"
    Command string  // shell command
}
```



…以及 `Canonical` 上的 `Hooks []Hook`。加载器：如果您希望它们处于规范状态，则来自顶级 `hooks/<event>.toml`，或者来自单个插件条目（M4 领域）。对于 M1，支持 `hooks/<event>.toml` 和 `{matcher, type, command}` 条目数组：



```toml
# hooks/PreToolUse.toml
[[hook]]
matcher = "Write|Edit"
type    = "command"
command = "echo intercepting Write/Edit"
```



测试装载机。犯罪。

- [ ] **步骤 10.2：渲染挂钩**

`internal/adapter/claude/hook.go`：



```go
package claude

import (
    "encoding/json"
    "fmt"

    "github.com/spxrogers/agentsync/internal/adapter"
    "github.com/spxrogers/agentsync/internal/source"
)

// renderHooks writes a single op for settings.json containing /hooks/<event>
// entries. Per-event ownership: agentsync owns the entire array under its
// event key. Foreign event keys (e.g. PreToolUse if user has authored
// directly) are NOT touched if they're not in canonical.
func (a *Adapter) renderHooks(c source.Canonical, p Paths) ([]adapter.FileOp, error) {
    if len(c.Hooks) == 0 {
        return nil, nil
    }
    byEvent := map[string][]map[string]any{}
    for _, h := range c.Hooks {
        entry := map[string]any{
            "matcher": h.Matcher,
            "hooks": []map[string]any{{
                "type":    h.Type,
                "command": h.Command,
            }},
        }
        byEvent[h.Event] = append(byEvent[h.Event], entry)
    }
    obj := map[string]any{"hooks": byEvent}
    body, err := json.MarshalIndent(obj, "", "  ")
    if err != nil {
        return nil, fmt.Errorf("marshal hooks: %w", err)
    }
    return []adapter.FileOp{{
        Action:        "write",
        Path:          p.Settings,
        Content:       append(body, '\n'),
        Mode:          0o644,
        SourceID:      "hooks/* (multiple)",
        MergeStrategy: "merge-json-keys",
    }}, nil
}
```



测试（保留外部钩子事件）；犯罪。

---

## 任务 11：`Render()` LSP 服务器

在 `settings.json` 中的 `/lspServers/<id>` 处镜像 MCP。相同的结构（command/args/env/url/headers）。相同的合并策略。

- [ ] 将 `LSPServer` 类型添加到 `source.Canonical`。来自 `lsp/<id>.toml` 的加载程序。渲染到 `settings.json` 中的 `/lspServers/<id>`。测试、接线、提交。

---

## 任务 12：`Apply()` — 使用合并感知逻辑编写 FileOps

**文件：**
- 创建：`internal/adapter/claude/apply.go`、`internal/adapter/claude/apply_test.go`

合并感知写入循环。对于每个 FileOp：如果 `MergeStrategy == "merge-json-keys"`，读取现有 JSON，解析我们的内容，MergeKeys（传入 OwnedKeys），写入合并。否则直接原子写入内容。

- [ ] **步骤 12.1：测试**



```go
func TestApply_NewSettings_WritesContent(t *testing.T) {
    tmp := t.TempDir()
    a := claude.New(claude.Options{TargetRoot: tmp})

    op := adapter.FileOp{
        Action:        "write",
        Path:          filepath.Join(tmp, ".claude.json"),
        Content:       []byte(`{"mcpServers":{"github":{"command":"npx"}}}`),
        Mode:          0o644,
        MergeStrategy: "merge-json-keys",
    }
    if err := a.Apply([]adapter.FileOp{op}); err != nil {
        t.Fatal(err)
    }
    body, _ := os.ReadFile(op.Path)
    if !strings.Contains(string(body), `"github"`) {
        t.Fatalf("missing github key: %s", body)
    }
}

func TestApply_PreservesForeignKeys(t *testing.T) {
    tmp := t.TempDir()
    a := claude.New(claude.Options{TargetRoot: tmp})
    target := filepath.Join(tmp, ".claude.json")

    // pre-existing foreign content
    _ = os.WriteFile(target, []byte(`{"foreign":{"x":1},"mcpServers":{"old":{}}}`), 0o644)

    op := adapter.FileOp{
        Action:        "write",
        Path:          target,
        Content:       []byte(`{"mcpServers":{"new":{"command":"x"}}}`),
        Mode:          0o644,
        MergeStrategy: "merge-json-keys",
        OwnedKeys:     nil, // no owned keys -> no orphan removal
    }
    if err := a.Apply([]adapter.FileOp{op}); err != nil {
        t.Fatal(err)
    }
    var out map[string]any
    body, _ := os.ReadFile(target)
    _ = json.Unmarshal(body, &out)
    if _, ok := out["foreign"]; !ok {
        t.Fatalf("foreign key dropped: %s", body)
    }
    s := out["mcpServers"].(map[string]any)
    if _, ok := s["old"]; !ok {
        t.Fatalf("foreign mcpServers.old dropped: %s", body)
    }
    if _, ok := s["new"]; !ok {
        t.Fatalf("our mcpServers.new missing: %s", body)
    }
}

func TestApply_OrphanRemoval(t *testing.T) {
    tmp := t.TempDir()
    a := claude.New(claude.Options{TargetRoot: tmp})
    target := filepath.Join(tmp, ".claude.json")
    _ = os.WriteFile(target, []byte(`{"mcpServers":{"github":{"command":"old"},"stale":{}}}`), 0o644)

    op := adapter.FileOp{
        Action:        "write",
        Path:          target,
        Content:       []byte(`{"mcpServers":{"github":{"command":"new"}}}`),
        Mode:          0o644,
        MergeStrategy: "merge-json-keys",
        OwnedKeys:     []string{"/mcpServers/github", "/mcpServers/stale"},
    }
    if err := a.Apply([]adapter.FileOp{op}); err != nil {
        t.Fatal(err)
    }
    var out map[string]any
    body, _ := os.ReadFile(target)
    _ = json.Unmarshal(body, &out)
    s := out["mcpServers"].(map[string]any)
    if _, ok := s["stale"]; ok {
        t.Fatalf("stale should be deleted: %s", body)
    }
    if s["github"].(map[string]any)["command"] != "new" {
        t.Fatalf("github should be updated: %s", body)
    }
}
```



- [ ] **步骤 12.2：实施**

`internal/adapter/claude/apply.go`：



```go
package claude

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
    if op.MergeStrategy == "merge-json-keys" {
        existing := readJSONFile(op.Path)
        var ours map[string]any
        if err := json.Unmarshal(op.Content, &ours); err != nil {
            return fmt.Errorf("parse our payload for %s: %w", op.Path, err)
        }
        merged, _, _ := MergeKeys(existing, ours, op.OwnedKeys)
        body, err := json.MarshalIndent(merged, "", "  ")
        if err != nil {
            return fmt.Errorf("marshal merged for %s: %w", op.Path, err)
        }
        return iox.AtomicWrite(op.Path, append(body, '\n'), mode)
    }
    return iox.AtomicWrite(op.Path, op.Content, mode)
}

func readJSONFile(path string) map[string]any {
    data, err := os.ReadFile(path)
    if err != nil {
        return map[string]any{}
    }
    var m map[string]any
    _ = json.Unmarshal(data, &m)
    if m == nil {
        return map[string]any{}
    }
    return m
}
```



- [ ] **步骤12.3：运行；提交**



```bash
go test -race ./internal/adapter/claude/...
git add internal/adapter/claude
git commit -m "$(cat <<'EOF'
feat(adapter/claude): Apply writes via iox.AtomicWrite, merge-aware for JSON

merge-json-keys ops read existing JSON, MergeKeys with OwnedKeys (orphan
removal), then write merged content. replace ops write Content directly.
delete ops remove the file (no-op if absent).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```



---

## 任务 13：`Ingest()` — 将本机文件读回规范

**文件：**
- 创建：`internal/adapter/claude/ingest.go`、`internal/adapter/claude/ingest_test.go`

渲染的逆。读取 `.claude.json`、settings.json、agents/、commands/、skills/、CLAUDE.md → 生成部分 `source.Canonical`。由 `agentsync import <selector>` 和漂移检测 (M3) 使用。

对于每种组件类型，摄取原生形式。往返奇偶校验 (`Ingest(Render(c)) == c`) 是测试目标。

- [ ] **步骤 13.1：测试（往返）**



```go
func TestIngest_RoundTripsMCPAndSkills(t *testing.T) {
    tmp := t.TempDir()
    enabled := true
    in := source.Canonical{
        MCPServers: []source.MCPServer{{
            ID: "github",
            Server: source.MCPServerSpec{
                Type: "stdio", Command: "npx", Args: []string{"-y", "x"},
                Env: map[string]string{"K": "V"}, Agents: []string{"*"},
                Enabled: &enabled,
            },
        }},
        Skills: []source.Skill{{
            Name:        "review",
            Frontmatter: map[string]any{"name": "review", "description": "x"},
            Body:        "body\n",
        }},
    }
    a := claude.New(claude.Options{TargetRoot: tmp})
    ops, _, _ := a.Render(in, adapter.ScopeUser, "")
    if err := a.Apply(ops); err != nil {
        t.Fatal(err)
    }
    out, err := a.Ingest(adapter.ScopeUser, "")
    if err != nil {
        t.Fatal(err)
    }
    if len(out.MCPServers) != 1 || out.MCPServers[0].ID != "github" {
        t.Fatalf("MCP roundtrip lost: %+v", out.MCPServers)
    }
    if len(out.Skills) != 1 || out.Skills[0].Name != "review" {
        t.Fatalf("Skill roundtrip lost: %+v", out.Skills)
    }
}
```



- [ ] **步骤 13.2：实施**

`internal/adapter/claude/ingest.go`：



```go
package claude

import (
    "encoding/json"
    "fmt"
    "os"
    "path/filepath"
    "strings"

    "github.com/spxrogers/agentsync/internal/adapter"
    "github.com/spxrogers/agentsync/internal/source"
)

func (a *Adapter) Ingest(scope adapter.Scope, project string) (source.Canonical, error) {
    p := ResolvePaths(a.opts.TargetRoot, project, scope == adapter.ScopeProject)
    var c source.Canonical

    // MCP from .claude.json (user) or settings.json (project)
    var mcpFile string
    if scope == adapter.ScopeProject {
        mcpFile = p.Settings
    } else {
        mcpFile = p.DotClaude
    }
    if data, err := os.ReadFile(mcpFile); err == nil {
        var top map[string]any
        if err := json.Unmarshal(data, &top); err != nil {
            return c, fmt.Errorf("parse %s: %w", mcpFile, err)
        }
        if servers, ok := top["mcpServers"].(map[string]any); ok {
            for id, raw := range servers {
                spec, ok := raw.(map[string]any)
                if !ok {
                    continue
                }
                m := source.MCPServer{ID: id, Server: source.MCPServerSpec{
                    Type:    asStr(spec["type"]),
                    Command: asStr(spec["command"]),
                    Args:    asStrSlice(spec["args"]),
                    Env:     asStrMap(spec["env"]),
                    URL:     asStr(spec["url"]),
                    Headers: asStrMap(spec["headers"]),
                }}
                c.MCPServers = append(c.MCPServers, m)
            }
        }
    }

    // Skills
    if entries, err := os.ReadDir(p.SkillsDir); err == nil {
        for _, e := range entries {
            if !e.IsDir() {
                continue
            }
            data, err := os.ReadFile(filepath.Join(p.SkillsDir, e.Name(), "SKILL.md"))
            if err != nil {
                continue
            }
            fm, body, err := ParseFrontmatter(data)
            if err != nil {
                continue
            }
            c.Skills = append(c.Skills, source.Skill{Name: e.Name(), Frontmatter: fm, Body: body})
        }
    }

    // Subagents (agents/<name>.md), commands, hooks, lsp, memory: similar pattern
    // — see Task 13.3 for the per-component snippets.

    return c, nil
}

func asStr(v any) string { s, _ := v.(string); return s }
func asStrSlice(v any) []string {
    arr, ok := v.([]any)
    if !ok {
        return nil
    }
    out := make([]string, 0, len(arr))
    for _, x := range arr {
        if s, ok := x.(string); ok {
            out = append(out, s)
        }
    }
    return out
}
func asStrMap(v any) map[string]string {
    m, ok := v.(map[string]any)
    if !ok {
        return nil
    }
    out := make(map[string]string, len(m))
    for k, val := range m {
        if s, ok := val.(string); ok {
            out[k] = s
        }
    }
    // also accept JSON-numeric values cast to string
    _ = strings.Title // unused; placeholder so import lines stay clean
    return out
}
```



- [ ] **步骤 13.3：添加其余的摄取路径**

子代理 (`p.AgentsDir/*.md` → `Subagents`)、命令 (`p.CommandsDir/*.md` → `Commands`)、挂钩（读取 `settings.json` `/hooks/<event>` 数组 → `Hooks` 切片）、LSP (`/lspServers` → `LSPServers`），内存（`p.Memory` → `Memory.Body` 逐字，没有片段解析，因为片段不可恢复）。

每个都遵循与技能/MCP 相同的模式。在 `ingest_test.go` 中测试每个 - 往返应用→摄取。犯罪。

---

## 任务 14：将 claude 适配器连接到注册表

**文件：**
- 修改：`internal/cli/registry_internal.go`

将“claude”的 NoopAdapter 替换为真正的 `claude.New(...)`：



```go
package cli

import (
    "github.com/spxrogers/agentsync/internal/adapter"
    "github.com/spxrogers/agentsync/internal/adapter/claude"
    "github.com/spxrogers/agentsync/internal/adapter/noop"
    "github.com/spxrogers/agentsync/internal/paths"
)

var registryFactory = func() *adapter.Registry {
    r := adapter.NewRegistry()
    home := paths.HomeDir(paths.OSEnv{})
    _ = r.Register(claude.New(claude.Options{TargetRoot: home}))
    for _, name := range []string{"opencode", "codex", "cursor"} {
        _ = r.Register(noop.New(name))
    }
    return r
}
```



- [ ] **步骤 14.1：更新 `cli/apply.go` 以允许实际应用（删除 M0 错误）**

在 `apply.go` `RunE` 中，更改：



```go
if !dryRun {
    return fmt.Errorf("M0 only supports --dry-run; ...")
}
```



…至：



```go
if !dryRun {
    plan, err := render.Plan(c, reg, agents, sc, "")
    if err != nil {
        return err
    }
    if err := render.Apply(plan, reg); err != nil {
        return err
    }
    fmt.Fprintln(cmd.OutOrStdout(), "applied:", plan.Total(), "ops")
    return nil
}
// (existing dry-run code follows)
```



- [ ] **步骤 14.2：集成测试**

`internal/cli/m1_integration_test.go`：



```go
package cli_test

import (
    "encoding/json"
    "os"
    "path/filepath"
    "strings"
    "testing"
)

func TestIntegration_M1_ClaudeMCPApply(t *testing.T) {
    tmp := t.TempDir()
    env := map[string]string{"AGENTSYNC_TARGET_ROOT": tmp}

    if _, err := runCLI(t, env, "init"); err != nil {
        t.Fatal(err)
    }
    if _, err := runCLI(t, env, "agent", "add", "claude"); err != nil {
        t.Fatal(err)
    }

    // Author an MCP file directly (the `mcp add` CLI lands in M4; for M1 we
    // exercise via direct file creation, which is the supported "vim-able"
    // path anyway).
    mcpFile := filepath.Join(tmp, ".agentsync", "mcp", "github.toml")
    _ = os.MkdirAll(filepath.Dir(mcpFile), 0o755)
    _ = os.WriteFile(mcpFile, []byte(`
[server]
type    = "stdio"
command = "npx"
args    = ["-y", "@modelcontextprotocol/server-github"]
agents  = ["claude"]
`), 0o644)

    out, err := runCLI(t, env, "apply")
    if err != nil {
        t.Fatalf("apply: %v\n%s", err, out)
    }

    body, err := os.ReadFile(filepath.Join(tmp, ".claude.json"))
    if err != nil {
        t.Fatalf("read .claude.json: %v", err)
    }
    var top map[string]any
    _ = json.Unmarshal(body, &top)
    s := top["mcpServers"].(map[string]any)["github"].(map[string]any)
    if !strings.HasPrefix(s["command"].(string), "npx") {
        t.Fatalf("github command = %v", s["command"])
    }
}
```



- [ ] **步骤14.3：运行；提交**



```bash
go test -race ./...
git commit -am "$(cat <<'EOF'
feat(cli): register real claude adapter; apply writes destinations

Replaces NoopAdapter for "claude" with claude.New. apply (no flag) now
writes; --dry-run continues to compute plan only. Integration test
verifies end-to-end MCP fanout to ~/.claude.json.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```



---

## 完成时间



```bash
$ AGENTSYNC_TARGET_ROOT=/tmp/x agentsync init
$ AGENTSYNC_TARGET_ROOT=/tmp/x agentsync agent add claude
$ cat > /tmp/x/.agentsync/mcp/github.toml <<'TOML'
[server]
type    = "stdio"
command = "npx"
args    = ["-y", "@modelcontextprotocol/server-github"]
TOML
$ AGENTSYNC_TARGET_ROOT=/tmp/x agentsync apply
applied: 1 ops

$ jq '.mcpServers.github' /tmp/x/.claude.json
{
  "type": "stdio",
  "command": "npx",
  "args": ["-y", "@modelcontextprotocol/server-github"]
}
```



每键合并在外键后仍然存在。往返：`Render → Apply → Ingest` 对于每个组件都是奇偶校验干净的，除了规范中提到有损的地方（Claude 没有 - Claude 是真实形状的来源）。 Linux/macos/windows 上的 CI 绿色。绒毛干净。

钩子、技能、子代理、命令、LSP——全部根据规范渲染，以其原始形状写入磁盘，无损失地摄取回来。