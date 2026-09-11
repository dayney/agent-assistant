# CLI and Desktop Platform Support Design

## Goal

Define and enforce one versioned support policy for the agentsync CLI and the
agent-assistant Desktop application on macOS and Windows. The policy must be
machine-readable, exercised by native CI, reflected in release artifacts, and
documented without implying that best-effort or preview targets receive the
same guarantees as supported targets.

## Support policy

The repository uses three support tiers:

- `supported`: every pull request runs the relevant native build and test gate;
  regressions block release.
- `compatible`: artifacts are produced and expected to work, but the older OS
  is not continuously exercised. Compatibility ends when an upstream toolchain
  can no longer build for it safely.
- `preview`: artifacts and scheduled native validation are provided, but known
  runner or ecosystem gaps may remain and do not block stable releases.

The initial policy is:

| Product | OS | Architecture | Runtime floor | Tier |
| --- | --- | --- | --- | --- |
| CLI | macOS | arm64, amd64 | macOS 14 | supported |
| CLI | macOS | arm64, amd64 | macOS 12 | compatible through macOS 13 |
| CLI | Windows | amd64 | Windows 11 25H2 and later | supported |
| CLI | Windows | amd64 | Windows 10 through Windows 11 24H2 | compatible |
| CLI | Windows | arm64 | Windows 11 25H2 and later | preview |
| Desktop | macOS | arm64, amd64 | macOS 14 and later | supported |
| Desktop | Windows | amd64 | Windows 11 25H2 and later | supported |
| Desktop | Windows | arm64 | Windows 11 25H2 and later | preview |

`platform-support.json` is the canonical machine-readable form of this table.
It also pins the toolchain versions used by CI and local builds. Human-facing
documentation explains the same tiers but does not replace the automated
contract tests.

## CI model

Required pull-request jobs use explicit runner labels instead of `latest`:

- macOS arm64: `macos-14`
- macOS amd64: `macos-15-intel`
- Windows amd64: `windows-2022`

The existing Linux hermetic release gate remains the authoritative full CLI
test suite. Native macOS and Windows CLI jobs add a filesystem-isolated smoke
workflow to the pure unit suite, because platform path and process behavior
cannot be proven inside the Linux container.

`latest` labels and Windows arm64 run in a separate scheduled canary workflow.
Canaries expose ecosystem drift without silently changing the required support
environment underneath a pull request.

Desktop native CI runs frontend tests and type checking, builds the matching Go
sidecar, checks the Rust shell, and builds a platform installer or application
bundle. A successful web build alone does not qualify as Desktop support.

## Toolchain policy

The first rollout pins:

- Go `1.26.5`
- Node.js `24.21.0`
- Rust `1.98.0`
- GoReleaser `2.17.1`

Go remains on the 1.26 line so the CLI can retain best-effort macOS 12 and
Windows 10 compatibility. A future Go minor upgrade must explicitly revise the
compatibility rows instead of silently narrowing them.

## Desktop sidecar and packaging

The Desktop sidecar build is a portable Node script. It derives the Rust host
triple and maps only these audited targets:

| Rust target | Go target | Bundled filename |
| --- | --- | --- |
| `aarch64-apple-darwin` | `darwin/arm64` | `agent-assistant-core-aarch64-apple-darwin` |
| `x86_64-apple-darwin` | `darwin/amd64` | `agent-assistant-core-x86_64-apple-darwin` |
| `x86_64-pc-windows-msvc` | `windows/amd64` | `agent-assistant-core-x86_64-pc-windows-msvc.exe` |
| `aarch64-pc-windows-msvc` | `windows/arm64` | `agent-assistant-core-aarch64-pc-windows-msvc.exe` |

Unknown triples fail closed. The script may accept an explicit target for tests
and cross-builds, but it never guesses an architecture.

Development and tests may still opt into `AGENT_ASSISTANT_CORE_BIN`. Packaged
applications invoke the declared Tauri sidecar through the official shell API;
they do not scan arbitrary sibling files or `PATH`.

Platform-specific Tauri configuration sets macOS 14 as the Desktop deployment
floor and builds `.app` plus `.dmg`. Windows builds an NSIS installer and embeds
the WebView2 bootstrapper. MSI is intentionally excluded from the first Windows
release because it adds a separate Windows-only WiX release surface without
improving the supported runtime boundary.

## Release and signing boundary

CLI release generation remains one GoReleaser job so its checksums and package
manager metadata retain the existing reproducibility invariant.

Desktop signing/notarization is a separate reusable workflow. The main release
workflow calls it only when the repository variable
`DESKTOP_RELEASE_ENABLED=true`. Enabling it without all signing credentials is
a hard failure; it never publishes unsigned installers as supported builds.
Until maintainers provision those external credentials, required CI still
proves source-build support while the public Desktop distribution remains
disabled and is documented as such.

## Safety and failure behavior

- Native smoke tests redirect both canonical source and destination paths into
  a temporary directory; they never touch a contributor's real agent config.
- The Desktop sidecar keeps its existing JSON-line protocol and error prefixes.
- Demo mode remains explicit and is never a fallback for a failed sidecar.
- Release workflow changes must not weaken the CLI's hermetic test gate,
  checksum generation, secret handling, or package-manager publication guards.

## Acceptance criteria

- The platform contract test rejects unsupported tiers, duplicate rows, missing
  required runners, Go/Node/Rust/GoReleaser pin drift, release target drift, and
  CI matrix drift.
- Required native CLI and Desktop jobs cover every `supported` row.
- Scheduled canaries cover `latest` macOS/Windows and Windows arm64 preview.
- Sidecar target mapping is unit tested, including `.exe` naming and rejection
  of unknown targets.
- macOS and Windows Tauri configurations encode the documented package formats
  and runtime floors.
- User-facing installation, support, security, and changelog documentation
  matches the machine-readable policy.
- Local lint, release tests, GoReleaser snapshot, Desktop frontend tests/build,
  Rust checks, and the native smoke test pass, or any host-only limitation is
  reported with the exact unrun platform gate.
