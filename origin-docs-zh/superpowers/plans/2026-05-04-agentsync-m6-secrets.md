# agentsync M6 — 秘密（环境 + 年龄）

> [`overview`](2026-05-04-agentsync-v1.0-overview.md) 中的约定。基于 M0–M5 构建。

**目标：** `~/.agentsync/secrets/secrets.age` 处的年龄加密文件； `${secret:foo.bar}` 应用时的分辨率； `secrets edit/get/set` 命令；仅限内存中的明文 — 除非通过用户的 $EDITOR 的 tmp 文件，否则不会持久保存到磁盘。

**架构：** 新的 `internal/secrets` 包。 `Resolver` 接口，因此值替换调用点独立于后端（env、age、future 1Password）。 `Resolver.Resolve("github.token")` 返回用于替换的明文。 apply pipeline 会遍历 `source.Canonical` 的字符串值字段，并在传递到适配器之前就地替换 `${secret:foo.bar}` 和 `${env:FOO}` 引用。

**技术堆栈：** `filippo.io/age`（供应库；不需要 `age` CLI）。

---

## 文件



```
NEW:
internal/secrets/
├── secrets.go         # Resolver interface, errors
├── env.go             # EnvBackend (resolves ${env:FOO})
├── age.go             # AgeBackend (encrypts/decrypts secrets.age)
└── *_test.go

internal/cli/
├── secrets.go         # secrets edit/get/set
└── secrets_test.go

MODIFIED:
internal/render/pipeline.go     # walk canonical, substitute ${secret:...} / ${env:...}
internal/source/loader.go       # secrets backend hint loaded from agentsync.toml
```



---

## 任务 1：`Resolver` + EnvBackend



```go
// Package secrets resolves ${secret:foo.bar} and ${env:FOO} references at
// apply-time. The active backend is selected from agentsync.toml [secrets]
// `backend` field (env|age).
package secrets

import (
    "fmt"
    "regexp"
    "strings"
)

// Resolver returns the cleartext value for a key like "github.token". An
// unknown key returns an error.
type Resolver interface {
    Resolve(key string) (string, error)
}

// SubstituteRefs walks s and replaces ${secret:dotted.key} and ${env:NAME}
// references. Unknown references are left as-is and reported in the
// returned []string of unresolved markers (caller decides whether to error).
func SubstituteRefs(s string, secrets Resolver, env Resolver) (string, []string, error) {
    var unresolved []string
    out := re.ReplaceAllStringFunc(s, func(m string) string {
        sub := re.FindStringSubmatch(m)
        if len(sub) < 3 {
            return m
        }
        kind, key := sub[1], sub[2]
        var r Resolver
        switch kind {
        case "secret":
            r = secrets
        case "env":
            r = env
        default:
            unresolved = append(unresolved, m)
            return m
        }
        v, err := r.Resolve(key)
        if err != nil {
            unresolved = append(unresolved, m)
            return m
        }
        return v
    })
    return out, unresolved, nil
}

var re = regexp.MustCompile(`\$\{(secret|env):([A-Za-z0-9._-]+)\}`)

// EnvBackend resolves ${env:NAME} via os.Getenv(NAME). Used both as the env
// resolver in SubstituteRefs and as the "backend = env" mode where secrets
// are also stored as env vars (e.g. by direnv / 1Password CLI).
type EnvBackend struct{}

func (EnvBackend) Resolve(key string) (string, error) {
    v := osGetenv(key)
    if v == "" {
        return "", fmt.Errorf("env var %q not set", key)
    }
    return v, nil
}

// indirection so tests can inject; real impl reads os.Getenv
var osGetenv = func(k string) string {
    return _osLookupEnv(k)
}
```



（`_osLookupEnv` 是 `os.Getenv`；间接使其可测试。）

- [ ] **测试替代参考**



```go
func TestSubstituteRefs_SecretsAndEnv(t *testing.T) {
    secrets := mapBackend{"github.token": "ghp_abc"}
    env := mapBackend{"HOME": "/Users/x"}
    got, unresolved, _ := secrets_pkg.SubstituteRefs(
        "token=${secret:github.token}; home=${env:HOME}; ?=${secret:missing}",
        secrets, env)
    if !strings.Contains(got, "token=ghp_abc") {
        t.Fatalf("substitution failed: %s", got)
    }
    if !strings.Contains(got, "home=/Users/x") {
        t.Fatalf("env substitution failed: %s", got)
    }
    if len(unresolved) != 1 {
        t.Fatalf("expected 1 unresolved, got %v", unresolved)
    }
}
```



（`mapBackend` 是一个实现 `Resolver` 的小测试假类型。）

承诺。

---

## 任务 2：AgeBackend



```bash
go get filippo.io/age@latest
```





```go
// AgeBackend reads ~/.agentsync/secrets/secrets.age, decrypts using identity
// file specified in agentsync.toml [secrets].identity_file, parses as TOML,
// and resolves dotted keys.
package secrets

import (
    "fmt"
    "io"
    "os"
    "strings"

    "filippo.io/age"
    "github.com/pelletier/go-toml/v2"
)

type AgeBackend struct {
    AgeFile      string // path to secrets.age
    IdentityFile string // path to age identity (private key)
    cache        map[string]string
}

func NewAgeBackend(ageFile, identityFile string) *AgeBackend {
    return &AgeBackend{AgeFile: ageFile, IdentityFile: identityFile}
}

func (b *AgeBackend) load() error {
    if b.cache != nil {
        return nil
    }
    idData, err := os.ReadFile(b.IdentityFile)
    if err != nil {
        return fmt.Errorf("read identity %s: %w", b.IdentityFile, err)
    }
    ids, err := age.ParseIdentities(strings.NewReader(string(idData)))
    if err != nil {
        return fmt.Errorf("parse age identity: %w", err)
    }
    encFile, err := os.Open(b.AgeFile)
    if err != nil {
        return fmt.Errorf("open age file %s: %w", b.AgeFile, err)
    }
    defer encFile.Close()
    rd, err := age.Decrypt(encFile, ids...)
    if err != nil {
        return fmt.Errorf("decrypt %s: %w", b.AgeFile, err)
    }
    raw, err := io.ReadAll(rd)
    if err != nil {
        return fmt.Errorf("read decrypted: %w", err)
    }
    var top map[string]any
    if err := toml.Unmarshal(raw, &top); err != nil {
        return fmt.Errorf("parse decrypted as TOML: %w", err)
    }
    b.cache = flatten("", top)
    return nil
}

func (b *AgeBackend) Resolve(dottedKey string) (string, error) {
    if err := b.load(); err != nil {
        return "", err
    }
    v, ok := b.cache[dottedKey]
    if !ok {
        return "", fmt.Errorf("secret %q not found", dottedKey)
    }
    return v, nil
}

func flatten(prefix string, m map[string]any) map[string]string {
    out := map[string]string{}
    for k, v := range m {
        key := k
        if prefix != "" {
            key = prefix + "." + k
        }
        switch vv := v.(type) {
        case map[string]any:
            for kk, vvv := range flatten(key, vv) {
                out[kk] = vvv
            }
        case string:
            out[key] = vv
        default:
            out[key] = fmt.Sprint(vv)
        }
    }
    return out
}

// Encrypt writes plaintext as TOML, encrypted to recipient.
func Encrypt(plaintext []byte, recipient string, dest string) error {
    rec, err := age.ParseX25519Recipient(recipient)
    if err != nil {
        return fmt.Errorf("parse age recipient: %w", err)
    }
    f, err := os.OpenFile(dest, os.O_WRONLY|os.O_CREATE|os.O_TRUNC, 0o600)
    if err != nil {
        return err
    }
    defer f.Close()
    w, err := age.Encrypt(f, rec)
    if err != nil {
        return fmt.Errorf("init age encrypt: %w", err)
    }
    if _, err := w.Write(plaintext); err != nil {
        return err
    }
    return w.Close()
}
```



- [ ] **测试往返**



```go
func TestAgeRoundTrip(t *testing.T) {
    tmp := t.TempDir()
    // generate identity
    id, err := age.GenerateX25519Identity()
    if err != nil { t.Fatal(err) }
    idPath := filepath.Join(tmp, "id.txt")
    _ = os.WriteFile(idPath, []byte(id.String()), 0o600)
    rec := id.Recipient().String()

    plain := []byte(`[github]
token = "ghp_abc"
[linear]
api_key = "lin_xyz"
`)
    agePath := filepath.Join(tmp, "secrets.age")
    if err := secrets_pkg.Encrypt(plain, rec, agePath); err != nil {
        t.Fatal(err)
    }

    b := secrets_pkg.NewAgeBackend(agePath, idPath)
    if v, _ := b.Resolve("github.token"); v != "ghp_abc" {
        t.Fatalf("github.token = %q", v)
    }
    if v, _ := b.Resolve("linear.api_key"); v != "lin_xyz" {
        t.Fatalf("linear.api_key = %q", v)
    }
}
```



承诺。

---

## 任务 3：`secrets edit/get/set`

`internal/cli/secrets.go`：



```go
agentsync secrets edit            # decrypt -> tmp -> $EDITOR -> re-encrypt
agentsync secrets get <key>       # print one value
agentsync secrets set <key>=<val> # mutate one value (decrypt -> patch -> re-encrypt)
```



实施：



```go
func newSecretsCmd() *cobra.Command {
    sec := &cobra.Command{Use: "secrets", Short: "manage age-encrypted secrets"}
    sec.AddCommand(
        &cobra.Command{Use: "edit", RunE: secretsEdit},
        &cobra.Command{Use: "get <key>", Args: cobra.ExactArgs(1), RunE: secretsGet},
        &cobra.Command{Use: "set <key=value>", Args: cobra.ExactArgs(1), RunE: secretsSet},
    )
    return sec
}

func secretsEdit(cmd *cobra.Command, _ []string) error {
    // 1. Resolve agePath + identityPath + recipient from agentsync.toml
    // 2. Read existing or create empty plaintext
    // 3. Write to tmp file in os.TempDir() (RAM-backed on macOS)
    // 4. Run os.Getenv("EDITOR") on the tmp file (default to vi)
    // 5. Read the edited bytes back; encrypt to agePath atomically via iox.AtomicWrite (after writing tmp)
    // 6. Remove the cleartext tmp file
    return nil
}
```



（每个函数都是简单的样板文件 - 工程师按照 M0/M1 中的模式填写显式读/写。）

承诺。

---

## 任务 4：将其替换为 apply

在 `internal/render/pipeline.go` 中，在调用每个适配器的 Render 之前，遍历 `source.Canonical` 并通过 `secrets.SubstituteRefs` 替换 `${secret:...}` / `${env:...}` 字符串：



```go
// in Plan(), after canonical loaded:
backend := secrets.SelectBackend(c.Config.Secrets) // returns AgeBackend or EnvBackend per [secrets].backend
env := secrets.EnvBackend{}
if err := substituteCanonicalSecrets(&c, backend, env); err != nil {
    return Plan{}, err
}
```



`substituteCanonicalSecrets` 遍历 `c.MCPServers[*].Server.Env`、`Args`、`Command`、`Headers` 等，在每个字符串上调用 `SubstituteRefs`。未解析的引用返回错误（应用块；用户看到缺少哪个秘密）。

- [ ] **实施** `substituteCanonicalSecrets`。测试 MCP 环境值 `${secret:github.token}` 在适配器渲染器看到它之前已得到解析。犯罪。

---

## 任务 5：集成测试



```go
func TestIntegration_M6_AgeSecretsResolveOnApply(t *testing.T) {
    tmp := t.TempDir()
    env := map[string]string{"AGENTSYNC_TARGET_ROOT": tmp}

    _, _ = runCLI(t, env, "init")
    _, _ = runCLI(t, env, "agent", "add", "claude")

    // generate age key
    id, _ := age.GenerateX25519Identity()
    idPath := filepath.Join(tmp, ".config", "agentsync", "age.key")
    _ = os.MkdirAll(filepath.Dir(idPath), 0o755)
    _ = os.WriteFile(idPath, []byte(id.String()), 0o600)

    // configure agentsync.toml [secrets]
    cfg := filepath.Join(tmp, ".agentsync", "agentsync.toml")
    body, _ := os.ReadFile(cfg)
    body = append(body, []byte(fmt.Sprintf(`
[secrets]
backend       = "age"
file          = "secrets/secrets.age"
recipient     = "%s"
identity_file = "%s"
`, id.Recipient().String(), idPath))...)
    _ = os.WriteFile(cfg, body, 0o644)

    // write secrets.age
    plain := []byte(`[github]
token = "ghp_abc"
`)
    _ = secrets_pkg.Encrypt(plain, id.Recipient().String(),
        filepath.Join(tmp, ".agentsync", "secrets", "secrets.age"))

    // mcp file with ${secret:...}
    _ = os.WriteFile(filepath.Join(tmp, ".agentsync", "mcp", "github.toml"), []byte(`
[server]
type    = "stdio"
command = "npx"
args    = ["-y", "@modelcontextprotocol/server-github"]
[server.env]
GITHUB_TOKEN = "${secret:github.token}"
`), 0o644)

    if _, err := runCLI(t, env, "apply"); err != nil {
        t.Fatal(err)
    }

    // verify .claude.json has the literal token
    body, _ = os.ReadFile(filepath.Join(tmp, ".claude.json"))
    if !strings.Contains(string(body), "ghp_abc") {
        t.Fatalf("token not substituted: %s", body)
    }
    // verify cleartext is NOT in the agentsync repo
    repoBytes, _ := os.ReadFile(filepath.Join(tmp, ".agentsync", "mcp", "github.toml"))
    if strings.Contains(string(repoBytes), "ghp_abc") {
        t.Fatalf("token leaked into source repo: %s", repoBytes)
    }
}
```



承诺。

---

## 完成时间

- `~/.agentsync/secrets/secrets.age` 通过 `filippo.io/age` 库解密（不需要 `age` CLI）。
- 规范中的 `${secret:dotted.key}` 引用（MCP env、挂钩命令等）在应用时解析为明文，从不保留在规范中。
- `secrets edit` 往返保留数据； `get`/`set` 以非交互方式工作。
- 当无法解析 `${secret:...}` 引用（缺少密钥）时应用错误 - 失败时大声，从不沉默。
- 自述文件记录了年龄密钥备份规则（丢失密钥=丢失所有秘密）；测试自述文件的快速启动命令是否有效。
- Linux/macos/windows 上的 CI 绿色（age 库是纯 Go；跨平台良好）。