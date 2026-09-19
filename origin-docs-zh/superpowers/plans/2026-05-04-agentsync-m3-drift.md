# agentsync M3 — 漂移、状态、差异、协调

> [`overview`](2026-05-04-agentsync-v1.0-overview.md) 中的约定。基于 M0/M1/M2 构建。

**目标：** 3 路哈希分类器 + 命令 `status`、`diff`、`reconcile`。文件级 + JSON 指针键级。外键报告。批量热键 + `--auto-*` 标志。

**架构：** 新的 `internal/drift` 包包含分类器（纯函数）。 `internal/cli/{status,diff,reconcile}.go` 连接 CLI 表面。仅在成功应用或成功协调写回时更新状态；成功的协调覆盖将通过现有渲染管道重新应用。

**技术堆栈：** stdlib 仅用于分类器。 `charmbracelet/huh` 用于协调提示循环（轻量级；v1 仅使用确认+简单字符输入）。

---

## 文件



```
NEW:
internal/drift/{classifier.go, classifier_test.go}
internal/cli/{status.go, status_test.go, diff.go, diff_test.go, reconcile.go, reconcile_test.go}
internal/render/state_apply.go     # applies state updates after Apply

MODIFIED:
internal/cli/apply.go              # wire state updates post-apply
internal/cli/root.go               # AddCommand status/diff/reconcile
internal/render/pipeline.go        # populate FileOp.OwnedKeys from state
```



---

## 任务 1：漂移分类器

**文件：** `internal/drift/{classifier.go, classifier_test.go}`

纯函数：给定（H_src，H_applied，H_dest）→类。

- [ ] **测试（表驱动涵盖所有 9 种情况）**



```go
package drift_test

import (
    "testing"

    "github.com/spxrogers/agentsync/internal/drift"
)

func TestClassify(t *testing.T) {
    type tc struct {
        name                                string
        hsrc, happlied, hdest               string // "" = nil
        want                                drift.Class
    }
    cases := []tc{
        {name: "clean", hsrc: "a", happlied: "a", hdest: "a", want: drift.Clean},
        {name: "pending", hsrc: "b", happlied: "a", hdest: "a", want: drift.Pending},
        {name: "drift", hsrc: "a", happlied: "a", hdest: "b", want: drift.Drift},
        {name: "converged", hsrc: "b", happlied: "a", hdest: "b", want: drift.Converged},
        {name: "conflict", hsrc: "b", happlied: "a", hdest: "c", want: drift.Conflict},
        {name: "new", hsrc: "a", happlied: "", hdest: "", want: drift.New},
        {name: "foreign-collision", hsrc: "a", happlied: "", hdest: "x", want: drift.ForeignCollision},
        {name: "orphan", hsrc: "", happlied: "a", hdest: "a", want: drift.Orphan},
        {name: "orphan-drifted", hsrc: "", happlied: "a", hdest: "b", want: drift.OrphanDrifted},
    }
    for _, c := range cases {
        t.Run(c.name, func(t *testing.T) {
            got := drift.Classify(c.hsrc, c.happlied, c.hdest)
            if got != c.want {
                t.Fatalf("Classify(%q,%q,%q) = %v, want %v", c.hsrc, c.happlied, c.hdest, got, c.want)
            }
        })
    }
}
```



- [ ] **实施**

`internal/drift/classifier.go`：



```go
// Package drift classifies (source, applied, destination) hash triples per the
// 9-case table in the agentsync design spec. Pure function, no IO.
package drift

type Class int

const (
    Clean Class = iota
    Pending
    Drift
    Converged
    Conflict
    New
    ForeignCollision
    Orphan
    OrphanDrifted
)

func (c Class) String() string {
    switch c {
    case Clean:
        return "clean"
    case Pending:
        return "pending"
    case Drift:
        return "drift"
    case Converged:
        return "converged"
    case Conflict:
        return "conflict"
    case New:
        return "new"
    case ForeignCollision:
        return "foreign-collision"
    case Orphan:
        return "orphan"
    case OrphanDrifted:
        return "orphan-drifted"
    }
    return "unknown"
}

// Classify returns the case for one tracked item. Empty string means "absent."
// hsrc=src hash now; happlied=last-applied hash; hdest=on-disk hash now.
func Classify(hsrc, happlied, hdest string) Class {
    switch {
    case happlied == "" && hdest == "" && hsrc != "":
        return New
    case happlied == "" && hdest != "" && hsrc != "":
        return ForeignCollision
    case happlied != "" && hsrc == "":
        if hdest == happlied {
            return Orphan
        }
        return OrphanDrifted
    case hsrc == happlied && hdest == happlied:
        return Clean
    case hsrc != happlied && hdest == happlied:
        return Pending
    case hsrc == happlied && hdest != happlied:
        return Drift
    case hsrc != happlied && hdest != happlied && hsrc == hdest:
        return Converged
    default:
        return Conflict
    }
}

// SafeForAutoApply returns true for cases apply can resolve without prompting.
func SafeForAutoApply(c Class) bool {
    return c == Clean || c == Pending || c == New || c == Converged
}
```



承诺：



```bash
git commit -am "feat(drift): 9-case 3-way hash classifier (pure function)

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```



---

## 任务 2：Apply 后写入线路状态

**文件：** `internal/render/state_apply.go`、修改`internal/render/pipeline.go`、`internal/cli/apply.go`

每次成功申请后，agentsync 必须：
1. 对于每个 `write` FileOp：哈希最终磁盘内容 + `state.Files[<key>]` 中的记录。
2. 对于每个 `merge-json-keys`/`merge-jsonc-keys` 操作：解析最终的磁盘上 JSON，对于来自 `ours` 的每个指针，哈希该子树 + `state.Keys[<key>]` 中的记录。

- [ ] **测试**



```go
package render_test

import (
    "encoding/json"
    "os"
    "path/filepath"
    "testing"

    "github.com/spxrogers/agentsync/internal/adapter"
    "github.com/spxrogers/agentsync/internal/adapter/noop"
    "github.com/spxrogers/agentsync/internal/render"
    "github.com/spxrogers/agentsync/internal/state"
)

func TestRecordState_FilesAndKeys(t *testing.T) {
    dir := t.TempDir()
    p := filepath.Join(dir, ".claude.json")
    _ = os.WriteFile(p, []byte(`{"mcpServers":{"github":{"command":"npx"}},"foreign":{}}`), 0o644)

    s := state.New()
    err := render.RecordOpsState(s, "claude", adapter.ScopeUser, "", []adapter.FileOp{{
        Action:        "write",
        Path:          p,
        MergeStrategy: "merge-json-keys",
        Content:       []byte(`{"mcpServers":{"github":{"command":"npx"}}}`),
        SourceID:      "mcp/github.toml",
    }})
    if err != nil {
        t.Fatal(err)
    }

    // Expect a key entry for /mcpServers/github
    var found bool
    for k := range s.Keys {
        if k == "claude:user::"+p+":/mcpServers/github" {
            found = true
        }
    }
    if !found {
        t.Fatalf("missing key entry; have: %+v", s.Keys)
    }
    _ = json.RawMessage{}
    _ = noop.New
}
```



- [ ] **实施**

`internal/render/state_apply.go`：



```go
package render

import (
    "crypto/sha256"
    "encoding/hex"
    "encoding/json"
    "fmt"
    "os"
    "time"

    "github.com/spxrogers/agentsync/internal/adapter"
    "github.com/spxrogers/agentsync/internal/state"
)

// RecordOpsState updates s with hashes for files and keys produced by ops.
// Caller is expected to call this AFTER a successful Apply.
func RecordOpsState(s *state.Targets, agent string, scope adapter.Scope, project string, ops []adapter.FileOp) error {
    now := time.Now().UTC()
    for _, op := range ops {
        if op.Action != "" && op.Action != "write" {
            continue
        }
        switch op.MergeStrategy {
        case "merge-json-keys", "merge-jsonc-keys":
            // Re-read final on-disk content and record per pointer
            data, err := os.ReadFile(op.Path)
            if err != nil {
                return fmt.Errorf("read post-apply %s: %w", op.Path, err)
            }
            var final map[string]any
            if err := json.Unmarshal(data, &final); err != nil {
                return fmt.Errorf("parse post-apply %s: %w", op.Path, err)
            }
            var ours map[string]any
            if err := json.Unmarshal(op.Content, &ours); err != nil {
                return fmt.Errorf("parse our payload for %s: %w", op.Path, err)
            }
            for _, ptr := range collectPointers(ours, "") {
                v := getPointer(final, ptr)
                hash := hashAny(v)
                key := fmt.Sprintf("%s:%s:%s:%s:%s", agent, scope.String(), project, op.Path, ptr)
                s.Keys[key] = state.KeyEntry{
                    SHA256:    hash,
                    AppliedAt: now,
                    SourceID:  op.SourceID,
                }
            }
        default:
            data, err := os.ReadFile(op.Path)
            if err != nil {
                return fmt.Errorf("read post-apply %s: %w", op.Path, err)
            }
            sum := sha256.Sum256(data)
            key := fmt.Sprintf("%s:%s:%s:%s", agent, scope.String(), project, op.Path)
            s.Files[key] = state.FileEntry{
                SHA256:    hex.EncodeToString(sum[:]),
                Mode:      op.Mode,
                AppliedAt: now,
                SourceID:  op.SourceID,
            }
        }
    }
    return nil
}

// collectPointers walks m and returns JSON pointers for every leaf-or-object
// at the second level (e.g. /mcpServers/github -> stop). agentsync owns at
// the second-level granularity; deeper edits fall under that key's value
// hash.
func collectPointers(m map[string]any, prefix string) []string {
    var out []string
    for k, v := range m {
        ptr := prefix + "/" + escapeJSONPointer(k)
        switch vv := v.(type) {
        case map[string]any:
            // Drill one level: each child key becomes a pointer.
            for kk := range vv {
                out = append(out, ptr+"/"+escapeJSONPointer(kk))
            }
        default:
            out = append(out, ptr)
        }
    }
    return out
}

func escapeJSONPointer(s string) string {
    s = replaceAll(s, "~", "~0")
    s = replaceAll(s, "/", "~1")
    return s
}

func replaceAll(s, from, to string) string {
    out := make([]byte, 0, len(s))
    for i := 0; i < len(s); {
        if i+len(from) <= len(s) && s[i:i+len(from)] == from {
            out = append(out, to...)
            i += len(from)
            continue
        }
        out = append(out, s[i])
        i++
    }
    return string(out)
}

func getPointer(m map[string]any, ptr string) any {
    parts := splitPtr(ptr)
    var cur any = m
    for _, p := range parts {
        mm, ok := cur.(map[string]any)
        if !ok {
            return nil
        }
        cur = mm[p]
    }
    return cur
}

func splitPtr(ptr string) []string {
    if len(ptr) > 0 && ptr[0] == '/' {
        ptr = ptr[1:]
    }
    if ptr == "" {
        return nil
    }
    parts := []string{}
    cur := []byte{}
    for i := 0; i < len(ptr); i++ {
        if ptr[i] == '/' {
            parts = append(parts, string(cur))
            cur = cur[:0]
            continue
        }
        cur = append(cur, ptr[i])
    }
    parts = append(parts, string(cur))
    for i, p := range parts {
        p = replaceAll(p, "~1", "/")
        p = replaceAll(p, "~0", "~")
        parts[i] = p
    }
    return parts
}

func hashAny(v any) string {
    data, _ := json.Marshal(v)
    sum := sha256.Sum256(data)
    return hex.EncodeToString(sum[:])
}
```



- [ ] **连接到应用**

在 `internal/cli/apply.go` 中，在 `render.Apply(plan, reg)` 之后：



```go
// Load + update state
home := paths.AgentsyncHome(paths.OSEnv{})
statePath := filepath.Join(home, ".state", "targets.json")
s, err := state.Load(statePath)
if err != nil {
    return err
}
for name, res := range plan.PerAgent {
    if err := render.RecordOpsState(s, name, sc, "", res.Ops); err != nil {
        return err
    }
}
if err := state.Save(statePath, s); err != nil {
    return err
}
```



测试、提交：



```bash
git commit -am "$(cat <<'EOF'
feat(render,cli): record state after successful apply

Files/keys hashed and stored to ~/.agentsync/.state/targets.json. Drift
detection in Task 4+ reads from this. State save is atomic
(iox.AtomicWrite via state.Save).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```



---

## 任务 3：管道拥有 OwnedKeys 流

**文件：** `internal/render/pipeline.go`

在调用每个适配器的渲染之前，管道会读取该适配器+范围+项目的 state.Keys，为适配器返回的每个操作将匹配的 `/file/pointer` 路径附加到 `op.OwnedKeys`。

实际上更干净：适配器返回没有 OwnedKeys 的操作；管道迭代由 `<agent>:<scope>:<project>:<file>:<pointer>` 键控的 state.Keys 并按文件路径分组→对于每个文件，收集指针→附加到具有该路径的操作。

- [ ] **在收集操作后，在 `Plan()` 中实现**指针注入步骤；测试孤儿删除是否可以通过完整路径进行。



```go
// in Plan():
for name, res := range out.PerAgent {
    for i, op := range res.Ops {
        if op.MergeStrategy == "merge-json-keys" || op.MergeStrategy == "merge-jsonc-keys" {
            res.Ops[i].OwnedKeys = ownedKeysFor(s, name, scope, project, op.Path)
        }
    }
    out.PerAgent[name] = res
}
```





```go
func ownedKeysFor(s *state.Targets, agent string, scope adapter.Scope, project, path string) []string {
    prefix := fmt.Sprintf("%s:%s:%s:%s:", agent, scope.String(), project, path)
    var out []string
    for k := range s.Keys {
        if strings.HasPrefix(k, prefix) {
            out = append(out, strings.TrimPrefix(k, prefix))
        }
    }
    return out
}
```



计划签名更改以接受 `*state.Targets`。更新来电者。测试：使用一个拥有的指针预填充状态，确保后续渲染使用该指针生成 FileOp.OwnedKeys。

犯罪。

---

## 任务 4：`agentsync status`

遍历 `state.Files` + `state.Keys`，对每个进行分类，打印每个（代理、文件）的细分。

- [ ] **实现** `internal/cli/status.go`：



```go
package cli

import (
    "crypto/sha256"
    "encoding/hex"
    "encoding/json"
    "fmt"
    "os"
    "path/filepath"
    "strings"

    "github.com/spf13/afero"
    "github.com/spf13/cobra"
    "github.com/spxrogers/agentsync/internal/adapter"
    "github.com/spxrogers/agentsync/internal/drift"
    "github.com/spxrogers/agentsync/internal/paths"
    "github.com/spxrogers/agentsync/internal/render"
    "github.com/spxrogers/agentsync/internal/source"
    "github.com/spxrogers/agentsync/internal/state"
)

func newStatusCmd() *cobra.Command {
    return &cobra.Command{
        Use:   "status",
        Short: "report drift across registered agents",
        Args:  cobra.NoArgs,
        RunE: func(cmd *cobra.Command, _ []string) error {
            home := paths.AgentsyncHome(paths.OSEnv{})
            c, err := source.Load(afero.NewOsFs(), home)
            if err != nil {
                return err
            }
            statePath := filepath.Join(home, ".state", "targets.json")
            s, err := state.Load(statePath)
            if err != nil {
                return err
            }
            reg := registryFactory()
            var agents []string
            for name, ag := range c.Config.Agents {
                if ag.Enabled {
                    agents = append(agents, name)
                }
            }
            plan, err := render.Plan(c, reg, agents, adapter.ScopeUser, "", s)
            if err != nil {
                return err
            }

            w := cmd.OutOrStdout()
            for _, name := range reg.Names() {
                res, ok := plan.PerAgent[name]
                if !ok {
                    continue
                }
                fmt.Fprintf(w, "[%s]\n", name)
                seen := map[string]bool{}
                // file-level: for each op, classify
                for _, op := range res.Ops {
                    if op.MergeStrategy != "" {
                        continue // covered key-by-key below
                    }
                    if seen[op.Path] {
                        continue
                    }
                    seen[op.Path] = true
                    hsrc := hashContent(op.Content)
                    happlied := s.Files[fmt.Sprintf("%s:user::%s", name, op.Path)].SHA256
                    hdest := hashFile(op.Path)
                    cls := drift.Classify(hsrc, happlied, hdest)
                    fmt.Fprintf(w, "  %-9s %s\n", cls, op.Path)
                }
                // key-level: for each merge op, walk owned pointers
                for _, op := range res.Ops {
                    if op.MergeStrategy != "merge-json-keys" && op.MergeStrategy != "merge-jsonc-keys" {
                        continue
                    }
                    var ours map[string]any
                    _ = json.Unmarshal(op.Content, &ours)
                    final := readJSON(op.Path)
                    for _, ptr := range render.PublicCollectPointers(ours, "") {
                        hsrc := hashAny(getPointer(ours, ptr))
                        happlied := s.Keys[fmt.Sprintf("%s:user::%s:%s", name, op.Path, ptr)].SHA256
                        hdest := hashAny(getPointer(final, ptr))
                        cls := drift.Classify(hsrc, happlied, hdest)
                        fmt.Fprintf(w, "  %-9s %s#%s\n", cls, op.Path, ptr)
                    }
                }
            }
            return nil
        },
    }
}

func hashContent(b []byte) string {
    sum := sha256.Sum256(b)
    return hex.EncodeToString(sum[:])
}

func hashFile(path string) string {
    data, err := os.ReadFile(path)
    if err != nil {
        return ""
    }
    return hashContent(data)
}

func hashAny(v any) string {
    if v == nil {
        return ""
    }
    data, _ := json.Marshal(v)
    return hashContent(data)
}

func readJSON(path string) map[string]any {
    data, err := os.ReadFile(path)
    if err != nil {
        return map[string]any{}
    }
    var m map[string]any
    _ = json.Unmarshal(data, &m)
    return m
}

func getPointer(m map[string]any, ptr string) any {
    if !strings.HasPrefix(ptr, "/") {
        return nil
    }
    parts := strings.Split(strings.TrimPrefix(ptr, "/"), "/")
    var cur any = m
    for _, p := range parts {
        mp, ok := cur.(map[string]any)
        if !ok {
            return nil
        }
        cur = mp[p]
    }
    return cur
}
```



（注：`render.PublicCollectPointers` 是 M3 Task 2 的 `collectPointers` 的导出版本 — 将其公开。）

连接到 `cli.Root.AddCommand(newStatusCmd())`。测试：



```go
func TestStatus_DriftAfterDirectEdit(t *testing.T) {
    tmp := t.TempDir()
    env := map[string]string{"AGENTSYNC_TARGET_ROOT": tmp}
    _, _ = runCLI(t, env, "init")
    _, _ = runCLI(t, env, "agent", "add", "claude")

    mcp := filepath.Join(tmp, ".agentsync", "mcp", "github.toml")
    _ = os.MkdirAll(filepath.Dir(mcp), 0o755)
    _ = os.WriteFile(mcp, []byte(`[server]
type="stdio"
command="npx"`), 0o644)
    _, _ = runCLI(t, env, "apply")

    // Modify destination directly
    dst := filepath.Join(tmp, ".claude.json")
    body, _ := os.ReadFile(dst)
    body = []byte(strings.Replace(string(body), `"npx"`, `"npm"`, 1))
    _ = os.WriteFile(dst, body, 0o644)

    out, err := runCLI(t, env, "status")
    if err != nil {
        t.Fatalf("status: %v\n%s", err, out)
    }
    if !strings.Contains(out, "drift") {
        t.Fatalf("status didn't report drift: %s", out)
    }
}
```



承诺。

---

## 任务 5：`agentsync diff [<path>]`

对于每个非干净项目，显示统一的差异（源与目标）。使用 `github.com/sergi/go-diff/diffmatchpatch` 作为差异渲染器。

- [ ] 添加依赖项、实施、测试（单文件差异和键级差异）。犯罪。

---

## 任务 6：`agentsync reconcile`

遍历所有漂移/冲突的项目，按项目提示用户。使用 `charmbracelet/huh`。

- [ ] **测试脚手架（通过 `teatest` 式线束进行 TUI 测试）**



```go
// integration test that pipes responses to stdin would be heavy; v1 ships
// reconcile with a non-interactive "--auto-*" path tested directly, plus
// a smoke test that reconcile exits 0 when input is empty (no drift).
func TestReconcile_NoDrift(t *testing.T) {
    tmp := t.TempDir()
    env := map[string]string{"AGENTSYNC_TARGET_ROOT": tmp}
    _, _ = runCLI(t, env, "init")
    _, _ = runCLI(t, env, "agent", "add", "claude")
    _, _ = runCLI(t, env, "apply")
    if _, err := runCLI(t, env, "reconcile", "--auto-safe"); err != nil {
        t.Fatal(err)
    }
}

func TestReconcile_AutoOverride(t *testing.T) {
    tmp := t.TempDir()
    env := map[string]string{"AGENTSYNC_TARGET_ROOT": tmp}
    _, _ = runCLI(t, env, "init")
    _, _ = runCLI(t, env, "agent", "add", "claude")

    mcp := filepath.Join(tmp, ".agentsync", "mcp", "github.toml")
    _ = os.MkdirAll(filepath.Dir(mcp), 0o755)
    _ = os.WriteFile(mcp, []byte(`[server]
type="stdio"
command="npx"`), 0o644)
    _, _ = runCLI(t, env, "apply")

    // Manually mutate destination to create drift
    dst := filepath.Join(tmp, ".claude.json")
    body, _ := os.ReadFile(dst)
    drifted := strings.Replace(string(body), `"npx"`, `"npm"`, 1)
    _ = os.WriteFile(dst, []byte(drifted), 0o644)

    // reconcile --auto-override should re-apply source value
    if _, err := runCLI(t, env, "reconcile", "--auto-override"); err != nil {
        t.Fatal(err)
    }
    final, _ := os.ReadFile(dst)
    if !strings.Contains(string(final), `"npx"`) {
        t.Fatalf("override didn't restore source value: %s", final)
    }
}
```



- [ ] **实现** `internal/cli/reconcile.go`：

三个标志：`--auto-writeback`、`--auto-override`、`--auto-safe`。按照与状态相同的顺序行走计划项目；对于每个：
- `Clean` / `Pending` / `New` / `Converged` → 无提示； `--auto-safe` 默默地解决它们。
- `Drift` / `Conflict` / `OrphanDrifted` → 提示除非 `--auto-*`。
- `Orphan` → 无提示（应用删除）。
- `ForeignCollision` → 无提示； `apply` 已备份原始文件。

对于每个提示项：



```
~/.claude/settings.json#/mcpServers/github   (drift)
  source:      {"command": "npx"}
  destination: {"command": "npm"}

  [w]rite-back  [o]verride  [s]kip  [i]gnore  [d]iff  [q]uit
```

- `w`（回写）：通过 `internal/source.Writer` 从目标值重写规范 TOML 键（作为一个小助手引入，使用 `pelletier/go-toml/v2` 重新编码单个 MCP/插件/等）。
- `o`（覆盖）：重新渲染并将源端值应用于目标（实际上下一个 `apply` 会执行此操作）。
- `s`（跳过）：暂时保留不一致；再次报告下一个状态。
- `i`（忽略）：将路径写入`~/.agentsync/ignore.toml`；分类器忽略它。
- `d` (diff)：打印统一diff，重新提示。
- `q`（退出）：停止；其余物品未触及。

批量：大写 `W` / `O` / `S` 将该操作应用于所有剩余项目。

实现使用 cobra + `os.Stdin` 上的简单读取字符循环（不需要完整的 huh 组件模型 - 保持较小）：



```go
package cli

import (
    "bufio"
    "fmt"
    "io"
    "os"
    "strings"

    "github.com/spf13/cobra"
    // ...
)

func newReconcileCmd() *cobra.Command {
    var autoWB, autoOR, autoSafe bool
    cmd := &cobra.Command{
        Use:   "reconcile",
        Short: "interactively resolve drift",
        RunE: func(cmd *cobra.Command, _ []string) error {
            // build plan, walk items, classify each
            // bulkAction tracks W/O/S overrides
            // for items needing prompt, read 1 char from os.Stdin or use --auto-* flags
            return reconcileRun(cmd, os.Stdin, autoWB, autoOR, autoSafe)
        },
    }
    cmd.Flags().BoolVar(&autoWB, "auto-writeback", false, "auto-resolve drift by writing dest back to source")
    cmd.Flags().BoolVar(&autoOR, "auto-override", false, "auto-resolve drift by re-applying source to dest")
    cmd.Flags().BoolVar(&autoSafe, "auto-safe", false, "auto-resolve only converged/pending/new")
    return cmd
}

func reconcileRun(cmd *cobra.Command, in io.Reader, autoWB, autoOR, autoSafe bool) error {
    // implementation: pseudocode
    //   for each item in plan:
    //     classify
    //     if SafeForAutoApply || (auto-safe && SafeForAutoApply): no-op or apply
    //     elif autoWB: writeBackItem(...)
    //     elif autoOR: overrideItem(...) (defer to next apply)
    //     else: prompt(in, item) -> {w,o,s,i,d,q,W,O,S}
    //   if any --auto-override happened, run render.Apply at end
    //   on quit: return early
    return nil
}
```



（实现很简单，但样板代码较多。工程师使用 M3 任务 7 中添加的 source.Writer 填充每个组件的 `writeBackItem` / `overrideItem`。）

承诺。

---

## 任务 7：`internal/source.Writer` 用于回写

**文件：** `internal/source/writer.go`、`internal/source/writer_test.go`

当 reconcile 选择回写时，agentsync 会更改规范文件。通过 `pelletier/go-toml/v2` AST 保留评论：我们今天没有完整的 AST 突变； v1策略：

对于 `mcp/<id>.toml`：将更新的 `MCPServer` 编组为 TOML 并原子写入 — 文件上方的注释在第一次写回时丢失。 **记录了 v1 权衡。** 作为 v1.x 的改进，更好的保存落地。



```go
package source

import (
    "fmt"
    "path/filepath"

    "github.com/pelletier/go-toml/v2"
    "github.com/spxrogers/agentsync/internal/iox"
)

// WriteMCP writes mcp/<id>.toml from m. Overwrites existing; comments lost.
func WriteMCP(home, id string, m MCPServer) error {
    body, err := toml.Marshal(m)
    if err != nil {
        return fmt.Errorf("marshal mcp %s: %w", id, err)
    }
    return iox.AtomicWrite(filepath.Join(home, "mcp", id+".toml"), body, 0o644)
}

// WritePlugin, WriteMarketplace, WriteSkill follow the same pattern.
```



测试，提交。

---

## 任务 8：`--auto-safe` 集成测试 + 最终提交



```go
func TestApplyThenReconcileAutoSafe(t *testing.T) {
    tmp := t.TempDir()
    env := map[string]string{"AGENTSYNC_TARGET_ROOT": tmp}
    _, _ = runCLI(t, env, "init")
    _, _ = runCLI(t, env, "agent", "add", "claude")
    // ... apply, then reconcile --auto-safe should be a no-op
    if _, err := runCLI(t, env, "reconcile", "--auto-safe"); err != nil {
        t.Fatal(err)
    }
}
```



承诺。

---

## 完成时间

- `agentsync status` 使用 9 类分类器报告每个智能体的漂移；干净地处理文件级+键级项目。
- `agentsync diff <path>` 显示统一的源与目标差异。
- `agentsync reconcile` 使用 `[w]/[o]/[s]/[i]/[d]/[q]` + 批量热键提示每个漂移的项目；非交互式标志（`--auto-writeback`、`--auto-override`、`--auto-safe`）在 CI 中工作。
- 外键（`state.Keys` 中没有条目的路径）列在 `status` 中，但从未进入 case 表。
- 往返 `apply → mutate dest → status (drift) → reconcile --auto-writeback → apply` 产生干净状态。
- CI 绿色。