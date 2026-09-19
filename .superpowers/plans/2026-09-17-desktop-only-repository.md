# Desktop-only Repository Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the public CLI and documentation website while preserving a working, testable, signed, auto-updating macOS/Windows Desktop application.

**Architecture:** Keep the existing React -> Tauri/Rust -> Go sidecar boundary. Delete only delivery surfaces and code that are exclusive to the public CLI or documentation website, then make CI, release automation, project memory, and user documentation describe and validate the Desktop product directly.

**Tech Stack:** React 19, TypeScript 5.8, Vite 7, Tauri 2, Rust 2021, Go 1.26.2, GitHub Actions, Node 24.

**Spec:** `.superpowers/specs/2026-09-17-desktop-only-repository-design.md`

## Global Constraints

- Retain `desktop/`, `cmd/agent-assistant-core/`, and every Go package used by the sidecar.
- Retain macOS ARM64 and Windows x64 signed release/update behavior.
- Do not touch `docs`, `origin-docs/`, or `origin-docs-zh/`.
- Preserve all pre-existing uncommitted Desktop updater and UI changes.
- Do not add dependencies.
- Do not commit, push, tag, or publish a release.
- Update `.agentsync/memory/AGENTS.md`; do not hand-edit managed `AGENTS.md` or `CLAUDE.md`.

---

### Task 1: Add the Desktop-only structural guard

**Files:**
- Create: `desktop/scripts/desktop-only.node-test.mjs`
- Modify: `desktop/package.json`

**Interfaces:**
- Consumes: repository root layout and active workflow/task-runner files.
- Produces: a Node test that rejects restored CLI/site delivery surfaces.

- [x] **Step 1: Write the failing structural test**

Create a Node test that resolves the repository root and asserts that these paths do not exist:

```js
const removedPaths = [
  "website",
  "cmd/agentsync",
  "internal/cli",
  ".goreleaser.yaml",
  ".github/workflows/docs-publish.yml",
  "test/e2e",
  "test/bdd",
];
```

The same test reads `.github/workflows/ci.yml`, `.github/workflows/release.yml`, and `justfile`, then rejects active references to `website`, `docs-publish`, `goreleaser`, and `cmd/agentsync`.

- [x] **Step 2: Run the test and verify RED**

Run: `node --test desktop/scripts/desktop-only.node-test.mjs`

Expected: FAIL because `website/` and the public CLI still exist.

- [x] **Step 3: Include the guard in the Desktop test command**

Change `desktop/package.json` so `npm test` runs both `scripts/release-tools.node-test.mjs` and `scripts/desktop-only.node-test.mjs` after Vitest.

- [x] **Step 4: Leave the guard red until Task 3**

Do not weaken the assertions to make the existing repository pass.

### Task 2: Remove CLI and website delivery surfaces

**Files:**
- Modify: `.agentsync/memory/AGENTS.md`
- Regenerate: `AGENTS.md`, `CLAUDE.md`
- Delete: `website/`
- Delete: `cmd/agentsync/`
- Delete: `internal/cli/`
- Delete: `.goreleaser.yaml`
- Delete: `.github/workflows/docs-publish.yml`
- Delete: `test/e2e/`
- Delete: `test/bdd/`
- Delete: `internal/release/`
- Delete: `scripts/reproducibility-diff.sh`
- Modify: `test/container/entrypoint.sh`
- Modify: `test/container/Containerfile`
- Modify: `justfile`

**Interfaces:**
- Consumes: `cmd/agent-assistant-core` as the only shipped Go executable.
- Produces: root commands that build/test the sidecar and Desktop application only.

- [x] **Step 1: Replace and render canonical project memory while the old CLI is available**

Describe the Desktop architecture, retained package map, build/test commands,
managed-file rule, documentation expectations, and updater invariants in
`.agentsync/memory/AGENTS.md`. Run `go run ./cmd/agentsync diff --scope project`
first. Apply only when the output is limited to the intended managed memory;
otherwise leave rendered files untouched and report the unrelated drift.

- [x] **Step 2: Remove the tracked CLI/site paths**

Use Git-aware deletion for tracked files. Do not delete ignored or untracked paths outside the exact list above.

- [x] **Step 3: Replace CLI container gates with sidecar gates**

The container entrypoint must run `go vet ./...`, `go build ./cmd/agent-assistant-core`, and `go test -race -count=1 ./...`. Remove CLI smoke, E2E, BDD, and GoReleaser steps.

- [x] **Step 4: Reduce the task runner**

Keep recipes for sidecar build, Go tests, Desktop tests/build/Rust tests, lint, CI, release tagging, and cleanup. Remove docs-site, GoReleaser, CLI E2E, CLI BDD, and live marketplace publishing recipes.

- [x] **Step 5: Tidy the Go module**

Run: `GOTOOLCHAIN=go1.26.2 go mod tidy`

Expected: CLI-only dependencies disappear while sidecar dependencies remain.

- [x] **Step 6: Run the retained Go suite**

Run: `AGENTSYNC_TEST_IN_CONTAINER=1 go test -count=1 ./...`

Expected: PASS for every retained Go package.

### Task 3: Make CI and release Desktop-only

**Files:**
- Modify: `.github/workflows/ci.yml`
- Modify: `.github/workflows/release.yml`
- Modify: `.github/workflows/desktop-release.yml`
- Modify: `scripts/release-tag.sh`
- Modify: `desktop/scripts/release-tools.node-test.mjs`

**Interfaces:**
- Consumes: stable `vX.Y.Z` tags and existing signing secrets.
- Produces: a GitHub Release containing signed macOS/Windows installers, updater archives/signatures, and `latest.json`.

- [x] **Step 1: Extend release workflow tests and verify RED**

Add assertions that the active release path contains no GoReleaser/docs jobs, creates or reuses a draft GitHub Release before asset upload, and publishes it only after `latest.json`.

Run: `node --test desktop/scripts/release-tools.node-test.mjs`

Expected: FAIL against the old CLI-first release workflow.

- [x] **Step 2: Rewrite CI around retained products**

Keep Go lint/tests and add Desktop npm build/tests plus Rust tests on macOS and Windows. Remove documentation-site and GoReleaser jobs.

- [x] **Step 3: Rewrite the release coordinator**

On a tag, validate and forward `github.ref_name`. On manual dispatch, call `scripts/release-tag.sh`, then forward the requested tag. Call only the reusable Desktop release workflow when `DESKTOP_RELEASE_ENABLED=true`; retain an explicit disabled notice otherwise.

- [x] **Step 4: Make the Desktop release self-contained**

In preflight, create a draft GitHub Release when the validated tag has no release. After signed assets and `latest.json` are uploaded, publish the draft and mark it latest. Preserve credential preflight, code signing, notarization, updater signing, and upload ordering.

- [x] **Step 5: Restrict release tags to stable versions**

Make `scripts/release-tag.sh` accept only `vX.Y.Z`, matching Desktop updater preflight. Update its self-test cases and product wording.

- [x] **Step 6: Verify GREEN**

Run:

```bash
./scripts/release-tag.sh --self-test
node --test desktop/scripts/release-tools.node-test.mjs
node --test desktop/scripts/desktop-only.node-test.mjs
```

Expected: all tests PASS.

### Task 4: Rewrite project-facing documentation and managed memory

**Files:**
- Modify: `README.md`
- Modify: `CONTRIBUTING.md`
- Modify: `SECURITY.md`
- Modify: `CHANGELOG.md`
- Modify: `context7.json` or delete it if its only consumer was the removed site

**Interfaces:**
- Consumes: the Desktop-only architecture and release flow.
- Produces: contributor and agent instructions that no longer direct work toward removed surfaces.

- [x] **Step 1: Rewrite README and contributing guidance**

Document Desktop purpose, three-layer runtime, local setup, commands, supported targets, tests, and signed updater release flow. Remove CLI installation and website instructions.

- [x] **Step 2: Preserve security invariants relevant to Desktop**

Keep canonical-file, symlink/path, secret-handling, sidecar, and updater trust boundaries. Remove claims that only apply to public CLI distribution.

- [x] **Step 3: Record the breaking scope change**

Add an `[Unreleased]` changelog entry stating that public CLI and docs-site delivery were removed in favor of Desktop-only distribution. Do not rewrite historical entries.

### Task 5: Full Desktop-only verification

**Files:**
- Verify all changed and retained files.

**Interfaces:**
- Consumes: Tasks 1-4.
- Produces: fresh evidence that the repository ships only Desktop and remains buildable.

- [x] **Step 1: Verify removed surfaces and active references**

Run: `node --test desktop/scripts/desktop-only.node-test.mjs`

Expected: PASS.

- [x] **Step 2: Verify Go Core**

Run:

```bash
GOTOOLCHAIN=go1.26.2 go build ./cmd/agent-assistant-core
AGENTSYNC_TEST_IN_CONTAINER=1 GOTOOLCHAIN=go1.26.2 go test -count=1 ./...
```

Expected: PASS.

- [x] **Step 3: Verify Desktop frontend and release tools**

Run from `desktop/`:

```bash
npm test
npm run build
```

Expected: PASS.

- [x] **Step 4: Verify the Tauri native layer**

Run:

```bash
cargo test --manifest-path desktop/src-tauri/Cargo.toml
cargo check --manifest-path desktop/src-tauri/Cargo.toml
```

Expected: PASS.

- [x] **Step 5: Run formatting/lint checks and inspect scope**

Run the retained lint recipe, then inspect `git status --short` and `git diff --stat`. Confirm no user-owned migration path was modified by this task.

- [x] **Step 6: Report release-only residual risk**

State that Apple notarization, Windows Authenticode, and updater-key publication remain GitHub Actions-only checks because their credentials are unavailable locally.
