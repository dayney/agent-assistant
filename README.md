# agent-assistant Desktop

agent-assistant is a local desktop application for managing AI coding-agent
configuration from one canonical source. The repository ships the Desktop app;
the former public command-line product and documentation website are no longer
part of the distribution.

The application currently supports signed releases for:

- macOS on Apple Silicon (`aarch64-apple-darwin`)
- Windows on x64 (`x86_64-pc-windows-msvc`)

## Architecture

The runtime has two layers:

1. `desktop/src/` contains the React user interface.
2. `desktop/src-tauri/` owns native windows, menus, updates, canonical data
   access, Rule synchronization, drift detection, and project discovery.

The Core is embedded in the Tauri process. There is no companion process or
JSON-lines IPC boundary in the packaged application.

Canonical user configuration remains under `~/.agentsync/`, with optional
project configuration under a repository's `.agentsync/` directory. The
Desktop app does not maintain a second configuration model.

## Local development

Prerequisites:

- Node.js 24 and npm
- a current Rust toolchain
- the platform prerequisites required by Tauri 2
- `just` for the repository task shortcuts

Install and launch the application:

```bash
cd desktop
npm ci
npm run tauri:dev
```

`tauri:dev` launches the real Desktop runtime with the embedded Rust Core.
`npm run dev:demo` is only for browser-based UI review and never contacts the
update service.

Build a local bundle:

```bash
just desktop-build
```

Local unsigned bundles are development artifacts. They are not trusted update
publishers and do not replace signed release validation.

## Tests

```bash
just test-fast       # fast embedded Rust Core tests
just test-release    # compatibility alias for the complete Desktop suite
just test-desktop    # React, release tooling, Rust tests, and Rust checks
just lint            # Rust formatting, Clippy, and TypeScript checks
just ci              # complete local gate
```

Filesystem-writing Rust tests use private temporary directories and never touch
the real `~/.agentsync/` tree.

## Automatic updates

Updater-enabled release builds check the stable GitHub release channel at startup.
Users can also choose **Check for Updates...** from the native menu or the
version entry in the sidebar. An available update shows its version, notes, and
download progress; installation requires confirmation and is blocked while the
Rule workbench has unsaved edits.

The release workflow creates a Draft GitHub Release, builds macOS ARM64 and
Windows x64 independently, verifies both updater signatures, generates
`SHA256SUMS.txt`, and checks the complete asset inventory before publishing the
Release as stable `latest`. A failed run remains a Draft and is never exposed as
the update channel.

Updater endpoints and artifact URLs are derived from the workflow's
`GITHUB_REPOSITORY` value. Local release-tool execution defaults to
`dayney/agent-assistant`, so production builds always check this repository's
stable `latest` release instead of the repository the project was migrated from.

Tauri updater signing is mandatory. The public key is versioned at
`desktop/src-tauri/updater.pubkey`; only its private key and password are GitHub
Secrets. The default `updater-only` mode produces a macOS ad-hoc signed package
and an unsigned Windows installer, so first-time users should expect Gatekeeper
or SmartScreen warnings. Setting `DESKTOP_SIGNING_MODE=platform-signed` enables
Apple Developer ID/notarization and Windows Authenticode, and fails closed if
any platform credential is missing. See [`desktop/README.md`](desktop/README.md)
for setup and recovery instructions.

Releases accept stable tags only:

```bash
just release v0.1.0
```

The same release can be started manually from GitHub Actions by supplying a
stable `vX.Y.Z` version.

## Repository boundaries

- `desktop/` is the only user-facing product.
- `desktop/src-tauri/src/core/` contains the embedded Rust domain logic.
- `.agentsync/` is the canonical project-level agent configuration; generated
  agent files must not be edited directly.
- `origin-docs/` and `origin-docs-zh/` are migration/reference material and are
  not application delivery surfaces.

See [`CONTRIBUTING.md`](CONTRIBUTING.md) for the development workflow and
[`SECURITY.md`](SECURITY.md) for the local-data and update trust model.

## License

See [`LICENSE`](LICENSE).
