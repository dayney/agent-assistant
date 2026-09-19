<!-- agentsync:managed memory-banner -->
> **Managed by [agentsync](https://agentsync.cc) — do not edit `CLAUDE.md` directly.**
> To change it, edit `.agentsync/memory/AGENTS.md` (or the relevant
> `.agentsync/memory/fragments/*.md` fragment) and run `agentsync apply`.
> Direct edits here are reported as drift and overwritten on the next apply.
<!-- /agentsync:managed memory-banner -->

# agent-assistant Desktop

Project memory for agent sessions working on the agent-assistant Desktop app.

## Product boundary

This repository ships one product: a local Desktop application for macOS ARM64
and Windows x64. There is no public `agentsync` CLI and no documentation website.

The application keeps agent configuration in `~/.agentsync/` and project
`.agentsync/` trees, translates that canonical data to supported coding-agent
formats, and manages Rule workflows through a native desktop interface.

The runtime boundary is:

```text
desktop/src (React)
  -> desktop/src-tauri (Tauri commands + embedded Rust Core)
```

There is no companion process or JSON-lines protocol. Keep the existing typed
Tauri command names and request/response fields aligned with the frontend
client.

## Managed project configuration

This repository dogfoods its own project configuration. The canonical source is
the `.agentsync/` tree. `AGENTS.md`, `CLAUDE.md`, `.agents/`, and `.claude/` are
rendered outputs.

To change memory, skills, rules, MCP config, or another agent setting, edit the
canonical file under `.agentsync/` and re-render it. Never hand-edit a rendered
adapter file because the next apply overwrites it.

## Desktop code map

- `desktop/src/` - React UI, application state, Core clients, and update flow.
- `desktop/src-tauri/` - native window/menu/update integration and Rust Core.
- `desktop/src-tauri/src/core/desktop/` - snapshots, Rule workflows, and analysis.
- `desktop/src-tauri/src/core/source/` - canonical on-disk readers/writers.
- `desktop/src-tauri/src/core/{adapter,drift,state,iox,jsonkeys,paths}.rs` - Core
  projection, safety, and persistence services.
- `desktop/scripts/` - signed updater manifest/config tools.
- `.github/workflows/desktop-release.yml` - signed macOS/Windows packaging and
  updater publication.

## Build and test

From `desktop/`:

```sh
npm install
npm run tauri:dev
npm test
npm run build
```

From the repository root:

```sh
just build
just test
just lint
just ci
```

`npm run dev:demo` is only for UI review. Normal product development uses
`npm run tauri:dev`, which launches the real embedded Rust Core.

Filesystem tests must use private temporary directories rather than a real home
directory.

## Automatic update invariants

Production updates use Tauri's signed updater and the stable GitHub `latest`
release. Preserve these requirements:

- Only signed release builds register the updater.
- macOS ARM64 and Windows x64 updater artifacts must both exist.
- Apple signing/notarization, Windows Authenticode, and Tauri updater signing
  are separate trust layers.
- Release assets are uploaded before `latest.json`.
- A release is published only after the complete updater manifest exists.
- Installation requires user confirmation and is blocked by unsaved Rule edits.
- Demo, development, and unsigned local builds do not check for updates.

Never place signing credentials or private updater keys in repository files.

## Secret-handling invariants

Canonical fields may contain `${secret:...}` and `${env:...}` references. The
Desktop Core treats them as opaque template values: it does not resolve or
decrypt them. Tauri responses may expose references and redacted endpoints, but
never credential cleartext.

Filesystem writes must remain atomic, path-bounded, and resistant to symlink
replacement. Treat native agent files and imported project paths as untrusted.

## On-disk fidelity

The on-disk artifact is the source of truth, not a partial Rust struct. Round-trip
tests start from complete filesystem fixtures and assert that files and unknown
fields survive. Unsupported data must be reported or explicitly documented;
never drop it silently.

Before asserting a new third-party agent path or capability, verify it against
that harness's current official documentation.

## Documentation contract

User-visible or architectural changes update the relevant files in the same
change:

- `README.md` for setup, architecture, supported platforms, and workflows.
- `desktop/README.md` for Desktop release and updater behavior.
- `SECURITY.md` for trust boundaries and security invariants.
- `CHANGELOG.md` under `[Unreleased]` for user-visible changes.
- `.superpowers/specs/` and `.superpowers/plans/` for approved architectural
  designs and their execution plans.

Historical documentation or user-owned migration directories are not runtime
dependencies and must not be rewritten incidentally.

## Code conventions

- Keep Rust code formatted with `rustfmt`; Clippy warnings fail the build.
- Before reimplementing a general-purpose capability, check Rust's standard
  library and maintained crates. Prefer a proven crate when it matches the
  required behavior and platforms; hand-write only project-specific logic or a
  small boundary whose safety semantics are not covered by an existing crate.
- Use `Result<_, String>` only at the Tauri boundary and add operation context
  when propagating filesystem or process failures.
- Keep React hooks dependency-complete and avoid duplicate state ownership.
- Do not add a dependency, font, state framework, transport, workflow, or new
  cross-layer boundary without explicit approval.
- Keep edits scoped and preserve unrelated working-tree changes.
- Conventional commits use scopes such as `feat(desktop):`, `fix(core):`, and
  `chore(release):`; commit only when explicitly requested.
