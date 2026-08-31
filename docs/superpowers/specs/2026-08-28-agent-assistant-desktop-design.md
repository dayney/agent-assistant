# agent-assistant Desktop Design

## Goal

Deliver a macOS-first Tauri 2 desktop client branded `agent-assistant` that
visualizes and safely manages the existing Agentsync governance model through
Global, Agent, and Project dimensions.

## Product model

The client shows one canonical source and its derived projections rather than
duplicating configuration per Agent. The effective value is resolved in this
order:

```text
Baseline < Global source < Agent override < Project Profile < runtime resolution
```

Native Agent files are rendered output. The client must show provenance for each
value, the adapter capability (`full`, `partial`, or `unsupported`), and the
diff before a write. Required unsupported components stop the operation and
require an explicit approval decision.

Global configuration contains shared rules, MCP, Skills, Workflows, Hooks, and
Subagents. An Agent view compares native capability and shows the Agent-specific
projection. A Project view binds a local Profile, documentation root, stack,
verification commands, enabled Agents, and local overrides. Project and Agent
are independent navigation dimensions over the same effective configuration.

Secrets are never placed in UI state or ordinary config JSON. The UI shows a
`${secret:...}` or `${env:...}` reference; macOS Keychain owns resolved values.
New work inherits the current system font and may not add a font package,
`next/font`, `@font-face`, remote font URL, named family, or arbitrary font
selector.

## Architecture

```text
Tauri 2 shell (Rust)
    ├── macOS window, Keychain boundary, file watch, IPC
    ├── React + TypeScript UI
    └── Go Agentsync Core sidecar
          ├── Baseline / Profiles / governance
          ├── source / project overlays
          ├── Agent adapters and capability report
          ├── preview / apply / drift / reconcile
          └── JSON output contract
```

The first vertical slice uses a typed `CoreClient` interface. The default Tauri
runtime invokes the Go Core via Tauri IPC; the Go sidecar reads the local
canonical/project trees and redacted native Agent inventory through a JSON-line
contract. An explicit demo client remains available for visual review only;
demo mode is visible in the UI and is not a silent fallback.

## Information architecture

The shell has `总览`, `全局配置`, `Agent`, `项目`, and `活动记录` navigation.
Every detail view supports `有效配置`, `配置源`, `原生投影`, and `差异` tabs.
The overview shows health counts, a capability warning, and a provenance chain.
The Agent page is a component matrix. The Project page is a Profile registry
with stack and verification state. Apply is always Preview → review warnings →
explicit approval → write → result.

## Acceptance criteria

- `npm run build` produces a Vite bundle for the desktop frontend.
- `cargo check` validates the Tauri shell.
- All primary controls are semantic buttons or navigation elements with visible
  focus states and accessible names.
- No new font family is loaded or explicitly selected.
- The UI reflows without overlap at desktop and compact widths.
- Demo data is labeled as demo data; Core errors are visible and do not fall
  back silently.
- Real mode reports `schemaVersion: 1` and discovers one level below
  `$HOME/git/work` (overridable with `AGENT_ASSISTANT_PROJECTS_ROOT`). Native
  inventory is read-only evidence until apply-state and drift projection are
  connected.
