# agent-assistant Desktop

The Desktop client is a Tauri 2 shell over the same Go Core as the agentsync
CLI. The canonical `~/.agentsync/` and project `.agentsync/` trees remain the
source of truth on macOS and Windows.

## Platform support

| Platform | Architecture | OS version range | Tier | Native bundle |
| --- | --- | --- | --- | --- |
| macOS | arm64 (Apple Silicon) | 14+ | supported | application + DMG |
| Windows | amd64 | 11 25H2+ | supported | NSIS installer |

Supported targets are required native CI gates. The complete CLI and Desktop
policy lives in [`../platform-support.json`](../platform-support.json).

## Prerequisites

Use the versions pinned by the repository:

- Go 1.26.5 (`go.mod`)
- Node.js 24.21.0 (`.node-version`)
- Rust 1.98.0 (`rust-toolchain.toml`)

Install the normal Tauri system prerequisites for the host platform, then run:

```bash
npm ci
npm test
npm run typecheck
npm run tauri:dev
```

`npm run tauri:dev` builds the matching Go sidecar before starting Tauri. To
build a native bundle:

```bash
npm run tauri:build
```

The sidecar resolver supports `aarch64-apple-darwin` and
`x86_64-pc-windows-msvc`. Pass `--target <triple>` to
`npm run build:core:bundle --` only when cross-building in CI; unknown triples
are rejected.

## Development modes

Normal Tauri launches use the real Core. `npm run dev:demo` starts the approved
browser-only visual prototype. `AGENT_ASSISTANT_CORE_BIN` may override the Core
binary for development and tests; packaged applications use only Tauri's
declared sidecar and do not scan sibling directories or `PATH`.

## Distribution boundary

Local bundles are unsigned source builds, not official distributions. The
Desktop release workflow is separate from the CLI GoReleaser job and is disabled
until maintainers set `DESKTOP_RELEASE_ENABLED=true`. Once enabled, credential
preflight, Apple signing/notarization verification, and Windows Authenticode
verification must all succeed before a DMG or NSIS installer is uploaded.
Release tags keep full SemVer prerelease/build metadata; native Apple and
Windows package metadata uses the corresponding numeric `X.Y.Z` core.
