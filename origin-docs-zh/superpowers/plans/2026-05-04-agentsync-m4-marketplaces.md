# agentsync M4 — 市场 + 插件

> [`overview`](2026-05-04-agentsync-v1.0-overview.md) 中的约定。基于 M0–M3 构建。

**目标：** 实现 Claude 市场摄取（5 种源类型：相对、github、url、git-subdir、npm）、插件安装/升级/启用/禁用/删除 CLI 界面、通过注册适配器从插件清单中进行每个组件投影、翻译报告、清单 sha 固定和更新模式（固定/跟踪/手动）。 `update` 命令（唯一与网络相关的动词）刷新市场缓存并预先计算待处理的碰撞。

**架构：** 新的 `internal/marketplace` 包。 `Fetcher` 接口，每种源类型（相对、git、npm）都有一个 impl。缓存位于 `~/.agentsync/.state/cache/marketplaces/<slug>/` 和 `.../plugins/<id>/`。插件安装将清单 sha 复制到 `plugins/<id>.toml` （规范）；每个组件的分解发生在 `internal/marketplace/projection.go` 中，并发出适配器通常使用的 `source.Canonical` 覆盖。

**技术堆栈：** `github.com/go-git/go-git/v5` (git fetch +稀疏)，stdlib `net/http` + `archive/tar` + `compress/gzip` (npm tarball)，运行时不需要`npm`/`Bun`。

---

## 文件



```
NEW:
internal/marketplace/
├── marketplace.go        # types, public API
├── fetcher.go            # Fetcher interface + dispatch
├── fetch_git.go          # github, url, git-subdir
├── fetch_npm.go          # npm tarball
├── fetch_relative.go     # local path
├── manifest.go           # parse marketplace.json + plugin.json schemas
├── projection.go         # plugin manifest -> []FileOp via per-component projection
├── update.go             # update modes (pinned/track/manual), pending-bump computation
└── *_test.go

internal/cli/
├── marketplace.go        # marketplace add/remove/list
├── plugin.go             # plugin install/upgrade/enable/disable/remove/list
├── update.go             # update [--apply [--auto|--auto-safe]]
└── *_test.go
```



---

## 任务 1：市场 + 插件清单模式

**文件：** `internal/marketplace/manifest.go`、`internal/marketplace/manifest_test.go`

逐字镜像已发布的架构 (`code.claude.com/docs/en/plugin-marketplaces`)。

- [ ] **实施**



```go
// Package marketplace models the Claude marketplace.json + plugin.json schemas.
package marketplace

// Marketplace is the .claude-plugin/marketplace.json document.
type Marketplace struct {
    Schema      string                  `json:"$schema,omitempty"`
    Name        string                  `json:"name"`
    Owner       Owner                   `json:"owner"`
    Description string                  `json:"description,omitempty"`
    Version     string                  `json:"version,omitempty"`
    Metadata    *MarketplaceMetadata    `json:"metadata,omitempty"`
    Plugins     []PluginEntry           `json:"plugins"`
    AllowCrossMarketplaceDependenciesOn []string `json:"allowCrossMarketplaceDependenciesOn,omitempty"`
}

type Owner struct {
    Name  string `json:"name"`
    Email string `json:"email,omitempty"`
}

type MarketplaceMetadata struct {
    PluginRoot string `json:"pluginRoot,omitempty"`
}

// PluginEntry is one plugin listed in a marketplace.
type PluginEntry struct {
    Name        string         `json:"name"`
    Source      Source         `json:"source"`
    Description string         `json:"description,omitempty"`
    Version     string         `json:"version,omitempty"`
    Author      *Author        `json:"author,omitempty"`
    Homepage    string         `json:"homepage,omitempty"`
    Repository  string         `json:"repository,omitempty"`
    License     string         `json:"license,omitempty"`
    Keywords    []string       `json:"keywords,omitempty"`
    Category    string         `json:"category,omitempty"`
    Tags        []string       `json:"tags,omitempty"`
    Strict      *bool          `json:"strict,omitempty"` // default true

    // Component config can be inlined here when strict=false:
    Skills      any            `json:"skills,omitempty"`     // string | []string
    Commands    any            `json:"commands,omitempty"`   // string | []string
    Agents      any            `json:"agents,omitempty"`     // string | []string
    Hooks       any            `json:"hooks,omitempty"`      // string | object
    MCPServers  map[string]any `json:"mcpServers,omitempty"`
    LSPServers  map[string]any `json:"lspServers,omitempty"`
}

type Author struct {
    Name  string `json:"name"`
    Email string `json:"email,omitempty"`
}

// Source is the polymorphic plugin source. Tag-based dispatch:
type Source struct {
    Kind     string `json:"source,omitempty"`     // "github" | "url" | "git-subdir" | "npm"
    // Relative-path is encoded as a JSON string at the parent level; see UnmarshalJSON.
    Repo     string `json:"repo,omitempty"`
    URL      string `json:"url,omitempty"`
    Path     string `json:"path,omitempty"`
    Ref      string `json:"ref,omitempty"`
    SHA      string `json:"sha,omitempty"`
    Package  string `json:"package,omitempty"`
    Version  string `json:"version,omitempty"`
    Registry string `json:"registry,omitempty"`
    // Relative is the relative-path string when Source was a JSON string.
    Relative string `json:"-"`
}

// UnmarshalJSON handles the polymorphic shape: string -> Relative; object -> Kind etc.
func (s *Source) UnmarshalJSON(data []byte) error {
    if len(data) > 0 && data[0] == '"' {
        var rel string
        if err := json.Unmarshal(data, &rel); err != nil {
            return err
        }
        s.Relative = rel
        return nil
    }
    type alias Source
    var a alias
    if err := json.Unmarshal(data, &a); err != nil {
        return err
    }
    *s = Source(a)
    return nil
}

// PluginManifest is .claude-plugin/plugin.json for a strict-mode plugin.
type PluginManifest struct {
    Name        string         `json:"name"`
    Description string         `json:"description,omitempty"`
    Version     string         `json:"version,omitempty"`
    MCPServers  map[string]any `json:"mcpServers,omitempty"`
    Skills      any            `json:"skills,omitempty"`
    Commands    any            `json:"commands,omitempty"`
    Agents      any            `json:"agents,omitempty"`
    Hooks       any            `json:"hooks,omitempty"`
    LSPServers  map[string]any `json:"lspServers,omitempty"`
}

// Reserved names trigger a warning on `marketplace add`.
var ReservedMarketplaceNames = []string{
    "claude-code-marketplace", "claude-code-plugins", "claude-plugins-official",
    "anthropic-marketplace", "anthropic-plugins", "agent-skills",
    "knowledge-work-plugins", "life-sciences",
}
```



（需要在文件顶部 `import "encoding/json"`。）

- [ ] **测试解析**



```go
func TestParseMarketplace_StringSource(t *testing.T) {
    raw := []byte(`{
        "name": "x",
        "owner": {"name": "y"},
        "plugins": [{"name": "p", "source": "./plugins/p"}]
    }`)
    var m marketplace.Marketplace
    if err := json.Unmarshal(raw, &m); err != nil {
        t.Fatal(err)
    }
    if m.Plugins[0].Source.Relative != "./plugins/p" {
        t.Fatalf("source = %+v", m.Plugins[0].Source)
    }
}

func TestParseMarketplace_ObjectSource(t *testing.T) {
    raw := []byte(`{
        "name": "x", "owner": {"name": "y"},
        "plugins": [{"name": "p", "source": {"source":"github","repo":"o/r","ref":"v1"}}]
    }`)
    var m marketplace.Marketplace
    _ = json.Unmarshal(raw, &m)
    if m.Plugins[0].Source.Kind != "github" || m.Plugins[0].Source.Repo != "o/r" {
        t.Fatalf("source = %+v", m.Plugins[0].Source)
    }
}
```



承诺。

---

## 任务 2：`Fetcher` 接口 + 相对 + git 源实现

**文件：** `internal/marketplace/{fetcher.go, fetch_relative.go, fetch_git.go, fetch_test.go}`



```go
// Fetcher fetches one plugin source into a local directory.
type Fetcher interface {
    Fetch(src Source, into string) (FetchResult, error)
}

type FetchResult struct {
    HeadSHA string  // for git sources
    Version string  // for npm
}

func Dispatch(src Source) Fetcher {
    if src.Relative != "" {
        return &RelativeFetcher{}
    }
    switch src.Kind {
    case "github", "url", "git-subdir":
        return &GitFetcher{}
    case "npm":
        return &NPMFetcher{}
    }
    return &errFetcher{err: fmt.Errorf("unknown source kind %q", src.Kind)}
}
```



- [ ] **实现RelativeFetcher**：通过afero或`os.CopyFS`简单`cp -r`（Go 1.23+）。对于 1.22，请使用手动行走。使用 tmpdir 源 + 目标进行测试。

- [ ] **实现 GitFetcher**：使用 `go-git/v5` 进行浅层克隆，并带有可选的 sha 签出。对于 `git-subdir`，通过 `core.sparseCheckout=true` 和包含 `Path/*` 的 `info/sparse-checkout` 文件配置稀疏签出。如果 go-git 的稀疏支持在已安装的版本上不完整，请回退到 `exec.Command("git", "clone", ...)`，然后是 `git sparse-checkout init/set`。针对 `file://` 裸存储库进行测试（在 TestMain 中设置）。

- [ ] **实现 NPMFetcher**：HTTP GET `<registry>/<package>/<version>` → 使用 `dist.tarball` URL 返回元数据 → GET tarball →gunzip+untar 到目标中。用户计算机上未安装 `npm`。使用通过 `httptest.Server` 提供的虚假注册表进行测试。

单独提交每个获取器。

---

## 任务3：插件安装/加载/投影

**文件：** `internal/marketplace/projection.go`、`internal/cli/plugin.go`

`agentsync plugin install <id>@<marketplace>`：
1. 从 `~/.agentsync/marketplaces/<name>.toml` + 缓存解析市场。
2. 在`marketplace.json`插件列表中找到`<id>`。
3. 计算清单 sha（如果是严格的，则为 plugin.json 字节的 sha256；如果是非严格的，则为市场入口块的 sha）。
4. 将插件获取到 `~/.agentsync/.state/cache/plugins/<id>/`。
5. 将`~/.agentsync/plugins/<id>.toml`写入`version`、`manifest_sha`、`update`，默认`agents=["*"]`。

下一个 `agentsync apply` 读取 `plugins/*.toml`，将每个 `projection.go` 扩展为适配器正常渲染的每个组件规范条目。

- [ ] **插件投影** 将 `PluginManifest` （严格）或 `PluginEntry` （非严格）分解为：



```go
// ProjectionResult adds entries to a canonical model from a plugin's components.
type ProjectionResult struct {
    MCPServers []source.MCPServer
    Skills     []source.Skill
    Subagents  []source.Subagent
    Commands   []source.Command
    Hooks      []source.Hook
    LSPServers []source.LSPServer
}

// Project loads .claude-plugin/plugin.json from cacheDir (strict) or returns
// the inlined entry's components (non-strict). Resolves ${CLAUDE_PLUGIN_ROOT}
// in commands/MCP server configs to cacheDir for non-Claude agents.
func Project(entry PluginEntry, cacheDir string) (ProjectionResult, error) {
    var pr ProjectionResult
    strict := entry.Strict == nil || *entry.Strict
    if strict {
        // load plugin.json from cacheDir/.claude-plugin/plugin.json
        var manifest PluginManifest
        data, err := os.ReadFile(filepath.Join(cacheDir, ".claude-plugin", "plugin.json"))
        if err == nil {
            if err := json.Unmarshal(data, &manifest); err != nil {
                return pr, fmt.Errorf("parse plugin.json: %w", err)
            }
            applyManifest(manifest, &pr, cacheDir)
        }
        // strict + marketplace-entry component overrides also merge in
        applyEntryOverrides(entry, &pr, cacheDir)
    } else {
        applyEntryFull(entry, &pr, cacheDir)
    }
    return pr, nil
}

func resolvePluginRoot(s string, cacheDir string) string {
    return strings.ReplaceAll(s, "${CLAUDE_PLUGIN_ROOT}", cacheDir)
}
```



（助手 `applyManifest`、`applyEntryOverrides`、`applyEntryFull` 遍历 MCPServers/Skills/etc，构建规范条目，在每个命令/arg/url 字段上运行 `resolvePluginRoot`。）

- [ ] **测试投影**



```go
func TestProject_StrictPluginJSON(t *testing.T) {
    cache := t.TempDir()
    _ = os.MkdirAll(filepath.Join(cache, ".claude-plugin"), 0o755)
    _ = os.WriteFile(filepath.Join(cache, ".claude-plugin", "plugin.json"),
        []byte(`{"name":"x","mcpServers":{"foo":{"command":"${CLAUDE_PLUGIN_ROOT}/run.sh"}}}`),
        0o644)
    pr, err := marketplace.Project(marketplace.PluginEntry{Name: "x"}, cache)
    if err != nil {
        t.Fatal(err)
    }
    if len(pr.MCPServers) != 1 {
        t.Fatalf("mcp = %d", len(pr.MCPServers))
    }
    cmd := pr.MCPServers[0].Server.Command
    if !strings.HasPrefix(cmd, cache) {
        t.Fatalf("CLAUDE_PLUGIN_ROOT not resolved: %s", cmd)
    }
}
```



- [ ] **实现** `internal/cli/plugin.go`：



```go
agentsync plugin install <id>[@<marketplace>]   # cache + write plugins/<id>.toml
agentsync plugin upgrade <id>                   # bump version in plugins/<id>.toml; re-fetch; update sha
agentsync plugin enable <id>                    # set enabled on plugins/<id>.toml (already implicit; flag noop unless we add one)
agentsync plugin disable <id>                   # set agents=[] effectively (or add a disabled bool)
agentsync plugin remove <id>                    # delete plugins/<id>.toml + cache
agentsync plugin list                           # walk plugins/*.toml, print
```



按子命令提交。测试使用 file:// fake repo 作为市场固定装置； npm 通过 httptest.Server。

---

## 任务 4：将插件投影连接到源加载器

**文件：**修改`internal/source/loader.go`

加载 `plugins/<id>.toml` 后，加载器解析每个插件的缓存清单，运行 `marketplace.Project()`，并将结果合并到规范模型中，以便适配器透明地看到插件的组件。

加载器获得一个 `cacheDir` 参数。通过 `~/.agentsync/.state/cache/plugins`。

- [ ] **测试**



```go
func TestLoad_PluginExpandsToMCP(t *testing.T) {
    fs := afero.NewMemMapFs()
    home := "/h"
    cache := "/h/.state/cache/plugins"
    _ = afero.WriteFile(fs, filepath.Join(home, "plugins", "x.toml"), []byte(`
[plugin]
id = "x@m"
version = "1"
`), 0o644)
    _ = afero.WriteFile(fs, filepath.Join(cache, "x", ".claude-plugin", "plugin.json"),
        []byte(`{"name":"x","mcpServers":{"server-from-plugin":{"command":"x"}}}`),
        0o644)
    c, err := source.LoadWithCache(fs, home, cache)
    if err != nil {
        t.Fatal(err)
    }
    var found bool
    for _, m := range c.MCPServers {
        if m.ID == "server-from-plugin" {
            found = true
        }
    }
    if !found {
        t.Fatalf("plugin's MCP not surfaced via projection: %+v", c.MCPServers)
    }
}
```

实现`source.LoadWithCache`（现有的`Load`成为一个薄包装器，传递`""`进行缓存，回退到跳过投影行为）。犯罪。

---

## 任务 5：市场添加/删除/列表

**文件：** `internal/cli/marketplace.go`、`internal/cli/marketplace_test.go`



```
agentsync marketplace add github:owner/repo[@ref]   # writes marketplaces/<owner-repo>.toml; fetches into cache
agentsync marketplace add https://example.com/r.git # url source
agentsync marketplace remove <name>                  # deletes marketplaces/<name>.toml + cache
agentsync marketplace list                           # walks marketplaces/*.toml, prints
```



`add` 通过 `marketplace.Fetcher` 执行初始提取，并将头 SHA 写入 `state.Marketplaces[<name>]`。犯罪。

---

## 任务 6：`update` 命令

**文件：** `internal/cli/update.go`、`internal/marketplace/update.go`

对于每个市场：重新获取（或对 HTTP URL 源使用缓存的 etag）。对于每个插件：根据其 `update` 模式 + 获取的清单计算待处理的碰撞。打印待处理的凹凸。使用 `--apply`：链接到应用逻辑。使用 `--auto-safe`：仅翻译报告无损的凹凸插件。



```go
// in marketplace/update.go:
func ComputePendingBumps(s *state.Targets, marketplaces []source.Marketplace, plugins []source.Plugin, fetched map[string]Marketplace) []Bump {
    var out []Bump
    for _, p := range plugins {
        // find p in fetched marketplaces
        // compute desired version per p.Update mode
        // if differs from current p.Version + manifest_sha, emit Bump{ID, From, To, ManifestSHA}
    }
    return out
}
```



用假的fetcher进行测试；犯罪。

---

## 任务7：翻译报告

**文件：** 修改 `internal/render/pipeline.go` 以发出结构化报告并使其可供 `apply`/`update --apply` 使用。

在计划之后、应用之前，渲染会发出每（插件、代理）行：



```
plugin: atlassian@anthropic
  claude    ✓ full   (1 mcp, 5 commands)
  opencode  ✓ full   (1 mcp, 5 commands)
```



通过 `--json` 以人类可读文本和结构化 JSON 形式发出。

- [ ] 实现 `internal/render/report.go` 从 `plan.PerAgent[i].Skips` + 规范的 MCPServers/Skills/etc-from-plugin 元数据构建报告。测试，提交。

---

## 任务 8：显示 sha pinning

**文件：**修改投影逻辑，以在 `plugin.json`（严格）或市场条目（非严格）的规范字节上计算 sha256。与 `plugins/<id>.toml` 的 `manifest_sha` 进行比较。如果不同，发出 `manifest-sha-mismatch` 警告 + 将其视为 `status` 中的漂移。

- [ ] 实施、测试、提交。

---

## 任务 9：集成测试 — 完整插件扇出



```go
func TestIntegration_M4_PluginFanoutClaudeAndOpenCode(t *testing.T) {
    tmp := t.TempDir()
    env := map[string]string{"AGENTSYNC_TARGET_ROOT": tmp}

    // Set up a fake marketplace as a local relative-path fixture
    fixture := filepath.Join(tmp, "fixture-marketplace")
    _ = os.MkdirAll(filepath.Join(fixture, ".claude-plugin"), 0o755)
    _ = os.WriteFile(filepath.Join(fixture, ".claude-plugin", "marketplace.json"),
        []byte(`{
            "name": "test-mp", "owner": {"name": "x"},
            "plugins": [{"name": "demo", "source": "./plugins/demo"}]
        }`), 0o644)
    plugDir := filepath.Join(fixture, "plugins", "demo", ".claude-plugin")
    _ = os.MkdirAll(plugDir, 0o755)
    _ = os.WriteFile(filepath.Join(plugDir, "plugin.json"),
        []byte(`{
            "name": "demo",
            "version": "1.0.0",
            "mcpServers": {"demo-mcp": {"command":"echo","args":["hi"]}}
        }`), 0o644)

    _, _ = runCLI(t, env, "init")
    _, _ = runCLI(t, env, "agent", "add", "claude")
    _, _ = runCLI(t, env, "agent", "add", "opencode")

    // marketplace add via local path
    if _, err := runCLI(t, env, "marketplace", "add", fixture); err != nil {
        t.Fatal(err)
    }
    if _, err := runCLI(t, env, "plugin", "install", "demo@test-mp"); err != nil {
        t.Fatal(err)
    }
    if _, err := runCLI(t, env, "apply"); err != nil {
        t.Fatal(err)
    }

    // verify both agents have demo-mcp
    body, _ := os.ReadFile(filepath.Join(tmp, ".claude.json"))
    if !strings.Contains(string(body), "demo-mcp") {
        t.Fatal("claude missing demo-mcp")
    }
    body, _ = os.ReadFile(filepath.Join(tmp, ".config", "opencode", "opencode.json"))
    if !strings.Contains(string(body), "demo-mcp") {
        t.Fatal("opencode missing demo-mcp")
    }
}
```



承诺。

---

## 完成时间

- 5 个插件源类型正确获取和提取（相对、github、url、git-subdir、npm）。
- `marketplace add/remove/list` 和 `plugin install/upgrade/enable/disable/remove/list` 端到端工作。
- 通过每个注册适配器的插件的 MCP/skills/subagents/commands/hooks/LSP 项目；翻译报告显示每个单元格 ✓/◐/✗。
- `${CLAUDE_PLUGIN_ROOT}` 解析为非 Claude 代理的缓存路径，并逐字传递给 Claude。
- `update` 轮询市场并打印待处理的碰撞，而无需触及代理配置； `update --apply` 链入 apply。
- `update --apply --auto-safe` 仅自动应用无损凹凸；有损的停止确认。
- 清单 sha pin 将重新上传的相同版本内容检测为漂移。
- 集成测试演示了在应用后单个插件安装扇形到 Claude + OpenCode。
- Linux/macos/windows 上的 CI 绿色。