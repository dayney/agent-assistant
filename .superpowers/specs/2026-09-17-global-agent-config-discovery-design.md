# Global Agent Configuration Discovery and Import Design

Date: 2026-09-17
Status: approved direction; deliver one reviewed commit per Desktop section

## Delivery order

The seven sidebar sections are independently reviewed, tested and committed in
this order: Rules, MCP, Skills, Workflows, Hooks, Subagents, Agent. Each section
must distinguish discovery from import. A section may report an artifact as
detected but not importable when its format or lossless canonical mapping has
not been verified. No import is inferred from mere filesystem presence.

## Goal

让 Desktop 客户端完整发现本机所有已安装或留下配置痕迹的 AI Agent 全局配置，并在不泄露敏感值、不静默丢失字段的前提下，支持用户预览后将可识别配置导入 `~/.agentsync/` 母版。

## Problem

当前 Rust Core 主要从 `~/.agentsync/` 读取已纳管 Agent，并只读取少量 Codex、Gemini、Cursor 的原生 Rule/MCP 信息。因此本机已经安装但尚未纳管的 Agent 不会完整出现在客户端中，用户也无法知道哪些全局配置已存在、哪些格式可导入、哪些能力尚未适配。

本机配置存在三种风险：

1. 不同 Agent 的全局配置路径、文件名和格式不同，单一固定路径扫描会漏报。
2. 配置文件中可能包含环境变量、命令参数、Header 或 Token，扫描结果不能把原文送入 React、日志或截图。
3. Canonical 模型无法表达的字段不能被导入后静默删除；必须明确显示为部分导入、不支持或仅原生保留。

## Scope

### In scope

- 启动扫描和手动重新扫描本机全局 Agent 配置。
- 覆盖当前 Agent registry 中的 31 个 Agent，并覆盖已检测到的 IDE/扩展形态 Agent（例如 GitHub Copilot、Augment、Cline、JetBrains AI Assistant）。
- 识别全局 Rule/Memory、MCP、Skills、Commands、Hooks、Subagents、Settings 等配置 artifact 的存在、路径、格式、大小和可读状态。
- 在 Desktop 中展示已发现、已纳管、未纳管、部分支持和不支持状态。
- 对有对应 parser 的 artifact 提供导入预览、差异、能力损失说明和用户确认后的 canonical 写入。
- 对没有对应 parser 的 artifact 保留原生文件不变，并明确显示“已发现但不能导入”，不得静默丢弃。

### Out of scope

- 不自动覆盖、移动、删除或改写任何原生 Agent 配置。
- 不把所有厂商私有配置强行转换成一个猜测性的 canonical schema。
- 不读取或展示 Secret cleartext、认证数据库、会话记录、历史记录、缓存和日志内容。
- 不新增 Web、H5、小程序或 CLI 产品面。
- 不在本任务中重新实现每个 Agent 的全部原生功能；未验证的能力保持显式的 `unknown` / `unsupported`。

## Architecture

```text
native global files
        |
        v
Rust discovery registry  ---- redacted inventory ---->  React Agents view
        |
        +---- parser/capability report ----> import preview
        |
        +---- explicit user confirmation ----> ~/.agentsync canonical source
```

### Rust Core ownership

Rust owns filesystem discovery, path allowlists, symlink checks, file-size limits, format detection, redaction, parser dispatch, import previews and canonical writes. React owns filters, selection, draft state, confirmation sheets and status presentation. Tauri commands expose typed inventory and preview/import results only.

### Discovery registry

The existing adapter registry becomes the source for known Agent identity and verified targets. Each Agent spec gains read-only global discovery entries:

- `agent_id` and display name;
- detector roots (home-relative, Application Support, or IDE extension storage);
- artifact kind and expected format (`markdown`, `json`, `jsonc`, `toml`, `yaml`, directory);
- scope (`user` only for this feature);
- parser/capability state (`full`, `partial`, `unsupported`, `unknown`).

Machine-specific paths are never hard-coded. The scanner resolves the current home and platform application-data roots, then applies explicit allowlists. Generic directory names such as `skills/` are evidence only when paired with an Agent detector or known extension identity.

### Inventory model

The snapshot gains a redacted inventory section:

```text
AgentInventory {
  agent_id
  display_name
  evidence[]
  artifacts[]
  discovery_state
}

Artifact {
  kind
  scope = user
  relative_path
  format
  state = present | empty | unreadable | unsupported
  size_bytes
  modified_at
  parser_state
  secret_refs[]
}
```

The inventory contains relative paths and metadata only. It never contains raw environment values, HTTP headers, command arguments, auth files, session data or arbitrary file bodies. `secret_refs` contains names or redacted classifications, never values.

### Import contract

Import is always a separate explicit operation:

1. User selects one Agent and one or more artifacts.
2. Rust parses only the selected allowlisted artifacts.
3. Rust returns an import preview containing proposed canonical changes, preserved fields, unsupported fields, conflicts and redacted secret references.
4. React displays the preview and blocks confirmation when the parser would lose unrepresented data unless the user explicitly accepts a partial import.
5. After confirmation, Rust writes through the existing canonical writer and records an activity item.

Every parser must produce an import report. An artifact is never reported as fully imported if any field was skipped. Unknown keys are either preserved in an existing passthrough field or listed as unsupported; if neither is possible, the import is refused.

## Desktop workflow

The Agent view becomes the discovery entry point:

- `重新扫描` performs a read-only scan and updates the inventory.
- Filters separate `已发现`, `已纳管`, `未纳管`, `可导入`, `部分支持` and `不支持`.
- Selecting an Agent shows its evidence and artifact rows without opening raw secret-bearing files.
- `预览导入` opens a review sheet with proposed canonical changes and loss report.
- `导入母版` is enabled only after preview; confirmation is explicit.
- Unreadable, unsupported and ambiguous artifacts remain visible with a reason and no destructive action.

The existing Rule workbench safety behavior remains unchanged: unsaved edits block navigation or import until saved, discarded or cancelled.

## Error and safety policy

- Missing files are normal `not-found` evidence, not errors.
- Permission failures, malformed files and oversized files become per-artifact warnings; one bad Agent cannot hide the rest of the inventory.
- Symlinks escaping an approved root are rejected.
- Scanner limits file size and excludes caches, logs, sessions, history, databases, auth stores and temporary files.
- Parser errors never fall back to string slicing or guessed schemas.
- All writes remain atomic and use the existing canonical path and backup policy.
- Secret-bearing fields use references such as `${secret:NAME}` or `${env:NAME}`. Cleartext values are redacted before crossing the Tauri response boundary.

## Verification

### Rust tests

- registry covers every known Agent and its verified global artifact paths;
- scan returns all fixture artifacts, including unknown and malformed files;
- cache/session/auth paths are excluded;
- symlink escape and file-size limits are rejected;
- MCP env/header/command values are redacted and only secret references cross the boundary;
- import preview reports partial/unsupported fields and refuses silent loss;
- canonical write is unchanged until explicit import confirmation;
- one unreadable Agent does not prevent other Agents from appearing.

### React tests

- inventory renders all detected Agents and artifacts;
- filters and rescan state are accessible;
- unsupported and partial states explain why import is unavailable or gated;
- preview must appear before import confirmation;
- unsaved Rule edits block the import navigation;
- no raw secret values appear in rendered markup.

### Manual Desktop verification

- scan on the current macOS machine at 1280x820 and 960x640;
- verify detected evidence for Claude, Codex, Cursor, Gemini, Antigravity, Cline, Trae, Windsurf, OpenCode, Qwen, Augment, Copilot and JetBrains AI Assistant without exposing contents;
- preview one fully supported, one partially supported and one unsupported artifact;
- confirm no native file changes after scan or preview;
- confirm only an explicit import changes `~/.agentsync/`.

## Acceptance criteria

1. A fresh Desktop launch displays every detected local Agent evidence without requiring it to be enabled in `~/.agentsync/`.
2. No scanner result contains secret cleartext or raw session/auth content.
3. Every artifact is classified as importable, partially importable, unsupported, unreadable or absent with a reason.
4. No import can silently discard an unmodeled field.
5. Scan and preview are read-only; canonical files change only after explicit confirmation.
6. The complete Desktop test gate and manual scan workflow pass.
