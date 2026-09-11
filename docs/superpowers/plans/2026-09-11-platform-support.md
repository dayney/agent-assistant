# CLI and Desktop Platform Support Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enforce the approved macOS and Windows support policy across the CLI and Desktop build, CI, release, and documentation surfaces.

**Architecture:** A root machine-readable policy drives contract tests that cross-check native CI and release targets. The CLI keeps its Linux hermetic release gate and gains isolated native smoke coverage. Desktop uses one tested sidecar target resolver, Tauri's declared sidecar API, platform-specific bundles, and a separately gated signed-release workflow.

**Tech Stack:** Go 1.26.5, GitHub Actions, GoReleaser 2.17.1, Node.js 24.21.0, Rust 1.98.0, Tauri 2, React 19, TypeScript, Vite, Vitest.

**Spec:** `docs/superpowers/specs/2026-09-11-platform-support-design.md`

## Global Constraints

- Preserve the existing CLI release gate, checksum, secret, and adapter invariants.
- Do not edit agentsync-rendered memory or skill files; canonical governance changes belong under `.agentsync/`.
- Keep Desktop React code presentational, Rust limited to platform transport, and Go as the domain Core.
- Do not publish or claim unsigned Desktop installers as supported distributions.
- Keep all native smoke filesystem writes under a test-owned temporary root.
- Update user-facing docs and `[Unreleased]` in the same change.

### Task 1: Add the platform policy and contract tests

**Files:**
- Create: `platform-support.json`
- Create: `internal/release/platform_contract_test.go`

- [x] **Step 1: Write the failing contract tests**

Decode the policy and assert its schema, tiers, uniqueness, required native
runners, release targets, and toolchain pin parity.

- [x] **Step 2: Run the focused test and confirm the expected failure**

Run `go test ./internal/release -run Platform`; it must fail because the policy
and new CI shape do not exist yet.

- [x] **Step 3: Add the minimal policy file**

Encode the approved CLI and Desktop support rows plus exact Go, Node, Rust, and
GoReleaser versions.

- [x] **Step 4: Run the focused test and retain only expected config failures**

The policy schema checks should pass while CI/toolchain parity still reports
the old values.

### Task 2: Pin toolchains and make the sidecar build portable

**Files:**
- Create: `.node-version`
- Create: `rust-toolchain.toml`
- Create: `desktop/scripts/build-sidecar.mjs`
- Create: `desktop/scripts/build-sidecar.node-test.mjs`
- Modify: `go.mod`, `desktop/package.json`, `desktop/package-lock.json`, `justfile`

- [x] **Step 1: Write failing sidecar resolver tests**

Cover all four audited Rust triples, Windows `.exe` output, explicit target
override, and unknown-target rejection.

- [x] **Step 2: Run the focused Node test and confirm failure**

Run `node --test scripts/build-sidecar.node-test.mjs`; it must fail before the
resolver exists.

- [x] **Step 3: Implement the resolver and build command**

Use `spawnSync` argument arrays, create the output directory portably, copy the
native development binary, and never invoke a shell.

- [x] **Step 4: Pin toolchains and update package metadata**

Bump Go within 1.26, add Node/Rust pins and Node engine metadata, and update the
GoReleaser local snapshot pin.

- [x] **Step 5: Run resolver, package, and Go tests**

Run the Node resolver tests, `npm test`, `npm run typecheck`, and focused Go
contract tests.

### Task 3: Add isolated native CLI smoke coverage

**Files:**
- Create: `test/platform/main_test.go`
- Modify: `justfile`, `test/container/entrypoint.sh`

- [x] **Step 1: Write the failing smoke test**

Build the CLI in a test temp directory and exercise init, agent add, apply, and
status with redirected canonical/destination roots.

- [x] **Step 2: Prove the test is hermetic**

The test must reject direct invocation unless its dedicated opt-in environment
variable is set, and all child process environment paths must point under the
test temp directory.

- [x] **Step 3: Add native and container recipes**

Expose one `test-platform` command for CI and include the smoke package in the
release container entrypoint without weakening the general filesystem guard.

- [x] **Step 4: Run the smoke test locally**

Run `just test-platform`; it must complete without writing outside its temporary
root.

### Task 4: Replace floating required runners with explicit matrices

**Files:**
- Modify: `.github/workflows/ci.yml`
- Create: `.github/workflows/platform-canary.yml`

- [x] **Step 1: Update required CLI runners**

Use explicit Linux, macOS arm64, macOS Intel, and Windows amd64 rows. Run both
the pure unit suite and native platform smoke on non-Linux supported rows.

- [x] **Step 2: Add required Desktop native build rows**

Install pinned Go/Node/Rust tools, run frontend tests/typecheck, build the
sidecar, check Rust, and build the platform bundle.

- [x] **Step 3: Add scheduled canaries**

Exercise `macos-latest`, `windows-latest`, and `windows-11-arm`; keep their
failures visible but outside the stable pull-request gate.

- [x] **Step 4: Upgrade pinned Actions and GoReleaser together**

Move checkout/setup-go/GoReleaser/cosign to the audited versions and preserve
the existing pin-parity guard.

- [x] **Step 5: Run the platform contract test**

It must prove every supported row is covered and no `latest` runner appears in
the required native matrices.

### Task 5: Use Tauri's declared sidecar and platform bundles

**Files:**
- Modify: `desktop/src-tauri/src/lib.rs`
- Modify: `desktop/src-tauri/Cargo.toml`, `desktop/src-tauri/Cargo.lock`
- Modify: `desktop/src-tauri/tauri.conf.json`
- Create: `desktop/src-tauri/tauri.macos.conf.json`
- Create: `desktop/src-tauri/tauri.windows.conf.json`
- Modify: `desktop/vite.config.ts`

- [x] **Step 1: Add Rust tests for command selection and response parsing**

Keep the environment override as an explicit development path and cover JSON
protocol errors independently from process transport.

- [x] **Step 2: Add the official Tauri shell plugin**

Packaged mode must invoke only the declared `agent-assistant-core` sidecar.
Remove sibling-directory and `PATH` scanning.

- [x] **Step 3: Add platform bundle configuration**

Set macOS minimum version 14 with app/DMG targets and Windows NSIS with embedded
WebView2 bootstrapper. Set the web target to Safari 17 compatibility.

- [x] **Step 4: Verify frontend, sidecar, Rust, and local Tauri bundle**

Run Node tests, frontend tests/build, sidecar build, `cargo test`, `cargo check`,
and a native Tauri build on the current host.

### Task 6: Add a fail-closed signed Desktop release boundary

**Files:**
- Create: `.github/workflows/desktop-release.yml`
- Modify: `.github/workflows/release.yml`

- [x] **Step 1: Add signing credential preflight**

When Desktop release is enabled, missing Apple or Windows signing credentials
must fail before any bundle is uploaded.

- [x] **Step 2: Build signed/notarized platform artifacts separately**

Keep Desktop jobs outside the GoReleaser checksum job. Upload only signed
macOS DMGs and signed Windows NSIS installers to the existing tag.

- [x] **Step 3: Gate rollout with a repository variable**

The existing CLI release remains operational until maintainers set
`DESKTOP_RELEASE_ENABLED=true`; the disabled state must be visible in release
job output and documentation.

- [x] **Step 4: Validate workflow structure and contract tests**

Parse all workflow YAML and rerun the platform/pin guards.

### Task 7: Synchronize documentation

**Files:**
- Modify: `README.md`
- Modify: `docs/user-guide.md`
- Modify: `website/src/content/docs/getting-started/install.mdx`
- Modify: `desktop/README.md`
- Modify: `SECURITY.md`
- Modify: `CHANGELOG.md`

- [x] **Step 1: Document the support matrix and tier meanings**

State exact macOS/Windows floors, architectures, and compatibility/preview
limits for both products.

- [x] **Step 2: Document source builds and release status**

Explain pinned toolchains, Windows NSIS/macOS DMG output, and the external
signing-credential gate without advertising disabled downloads.

- [x] **Step 3: Update security and changelog**

Record signing expectations, WebView2 bootstrap behavior, and the complete
user-visible change under `[Unreleased]`.

### Task 8: Run final verification and review

**Files:** All changed files.

- [ ] **Step 1: Run formatting, lint, and the hermetic release gate**

Run `just lint` and `just test-release`, then inspect any rewritten files.

Status: `just lint` passes with zero issues. `just test-release` reaches the
container build and is blocked because the local Docker daemon is not running.

- [x] **Step 2: Run release and Desktop build gates**

Run `just test-platform`, a GoReleaser snapshot, all Desktop tests/typecheck/web
build, Rust tests/checks, and the current-host Tauri bundle.

- [ ] **Step 3: Re-run governance and drift checks**

Run the source-built governance verifier where local governance state exists,
plus `agentsync diff --scope project`.

Status: project diff reports only the intentionally uncommitted generated
agent-governance mirrors. Governance verification cannot run because this
checkout has no `.agent-governance/manifest.json`.

- [x] **Step 4: Review scope and evidence**

Run `git diff --check`, inspect the complete diff, and confirm no current user
worktree changes, secrets, generated native adapters, or unrelated files were
included.
