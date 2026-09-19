# Contributing to agent-assistant Desktop

This repository ships one product: the React/Tauri Desktop application with an
embedded Rust Core. Changes should preserve that boundary and avoid introducing
a second canonical configuration model in TypeScript.

## Prerequisites

- Node.js 24 and npm
- a current Rust toolchain with `rustfmt` and Clippy
- Tauri 2 platform prerequisites
- `just`

Install Desktop dependencies with `cd desktop && npm ci`.

## Development

```bash
cd desktop
npm run tauri:dev
```

The real application uses typed Tauri commands backed by the embedded Rust
Core. Browser demo mode is for visual review only and must not become a fallback
for native Core failures.

Useful repository commands:

| Command | Purpose |
| --- | --- |
| `just build` | Build the frontend and check the embedded Rust Core. |
| `just test-fast` | Run Rust Core unit tests. |
| `just test-release` | Run the complete Desktop suite (compatibility alias). |
| `just test-desktop` | Run frontend, release-tool, and Rust checks. |
| `just desktop-build` | Build a local Tauri bundle. |
| `just lint` | Check Rust formatting, Clippy, and TypeScript. |
| `just ci` | Run the complete local readiness gate. |

Filesystem tests use private temporary directories and must never inspect a
developer's real home directory.

## Architecture rules

- Put user interface and view state in `desktop/src/`.
- Put native window, menu, update, product behavior, and configuration
  transformations in `desktop/src-tauri/`.
- Expose only narrow typed Tauri commands; never return secret cleartext.
- Keep canonical and native Rule writes atomic and reject non-regular targets.
- Check the Rust standard library and maintained crates before implementing a
  general-purpose capability. Keep custom code for project-specific contracts.
- Update tests and user-facing documentation whenever behavior or a contract
  changes.

This repository's agent configuration is generated from `.agentsync/`. Edit
`.agentsync/memory/AGENTS.md` or `.agentsync/skills/`, then render it through the
project configuration workflow. Do not hand-edit generated `AGENTS.md`,
`CLAUDE.md`, or generated skill copies.

## Pull requests

Keep changes focused and include tests at the layer where the behavior lives.
Before opening a pull request, run:

```bash
just ci
git diff --check
```

Use conventional commits with an explicit scope, for example:

```text
feat(desktop): add project conflict review
fix(updater): reject incomplete release manifests
test(core): cover rule write-back collision
docs(readme): clarify signed release targets
```

For security-sensitive work, use the private reporting route in
[`SECURITY.md`](SECURITY.md) before opening a public issue or pull request.

## Releases

Releases use stable annotated tags only (`vX.Y.Z`). `just release vX.Y.Z` and
the manual GitHub Actions release entry point share
`scripts/release-tag.sh`, so they enforce the same version rule.

The release coordinator first runs CI, then invokes the signed Desktop workflow
when `DESKTOP_RELEASE_ENABLED=true`. That workflow creates or reuses a draft
GitHub Release, builds and signs macOS ARM64 and Windows x64 packages, publishes
the updater manifest only after all referenced assets exist, and finally
publishes the release. A failed signing or packaging job intentionally leaves
the release incomplete rather than exposing a partial update.
