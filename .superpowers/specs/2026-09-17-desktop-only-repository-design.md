# Desktop-only Repository Design

Date: 2026-09-17
Status: approved direction, pending implementation

## Goal

Turn this repository into a Desktop-only product. The supported deliverable is
the Tauri application for macOS ARM64 and Windows x64. The public `agentsync`
CLI and the Astro documentation website stop being products and are removed.

## Chosen approach

Keep the current React/Tauri shell and its Go sidecar boundary. Remove delivery
surfaces that are unrelated to the Desktop product, but do not rewrite the Go
Core in Rust or duplicate its logic inside the frontend.

This is preferred over the alternatives:

1. Keeping the CLI and website dormant leaves their tests, documentation, and
   release pipelines as permanent maintenance obligations.
2. Moving the Go Core into Rust would be a much larger rewrite with no Desktop
   user benefit and substantial regression risk.

## Product boundary

The resulting runtime remains:

```text
React UI -> Tauri/Rust commands -> agent-assistant-core -> shared Go packages
```

`cmd/agent-assistant-core` is not a public CLI. It is the private local sidecar
bundled into the Desktop application and must remain.

The following are retained:

- `desktop/`
- `cmd/agent-assistant-core/`
- Go packages reachable from the sidecar
- Desktop tests and shared Go package tests
- `.github/workflows/desktop-release.yml`
- signing, notarization, updater signatures, `latest.json`, and restart flow
- macOS ARM64 and Windows x64 release targets

The following are removed:

- `website/`
- `cmd/agentsync/`
- `internal/cli/`
- `.goreleaser.yaml`
- CLI-only E2E and BDD suites
- documentation-site publishing workflow and CI jobs
- CLI package-manager publishing and installation instructions
- Go packages that are unreachable from the Desktop sidecar and exist only for
  the removed CLI, after dependency and test verification

User-created or currently untracked migration material is outside this cleanup.
In particular, `docs`, `origin-docs/`, and `origin-docs-zh/` are not deleted or
rewritten by this task.

## Release design

`.github/workflows/release.yml` becomes a Desktop-only coordinator:

1. Validate a stable `vX.Y.Z` version and run Desktop-focused CI.
2. Create the version tag for a manual dispatch when needed.
3. Create a draft GitHub Release so release notes exist before packaging.
4. Call `.github/workflows/desktop-release.yml`.
5. Build signed macOS and Windows packages.
6. Upload installers and updater archives, then upload `latest.json` last.
7. Publish the draft release only after every required asset exists.

The updater continues to read the stable GitHub `latest` release. No website or
CLI artifact is required by the update path.

## CI and developer commands

The root task runner is reduced to Desktop-relevant operations:

- build the Go sidecar
- run Go tests for retained packages
- run Desktop TypeScript and release-tool tests
- build the Desktop frontend
- run Rust tests/checks
- run formatting and linting for retained code
- cut a Desktop release tag

CI removes GoReleaser, documentation-site, CLI smoke, CLI lifecycle, and CLI BDD
jobs. It retains Go validation and adds the Desktop npm/Rust gates needed to
protect the shipped application.

## Documentation and governance

The root README becomes a Desktop product README. Security documentation keeps
the canonical-file and updater trust boundaries. The changelog receives an
Unreleased breaking-change entry without rewriting historical entries.

The managed project memory is updated at its canonical `.agentsync/` source and
re-rendered; generated `AGENTS.md` and `CLAUDE.md` are not edited directly.

The repository currently has no CodeWiki configuration and no governance
profile manifest. Their unavailable checks are recorded as pre-existing setup
gaps rather than recreated as part of this product cleanup.

## Verification

Implementation uses a structural regression test that fails while the removed
CLI/site surfaces still exist. Completion requires fresh evidence for:

- no `website/`, `cmd/agentsync/`, `internal/cli/`, or `.goreleaser.yaml`
- no active CI/release/task-runner dependency on those surfaces
- `go test ./...` for the retained Go module
- Desktop unit/release-tool tests
- Desktop TypeScript build
- Rust tests/checks
- Go sidecar build
- Tauri configuration generation and release-script tests

Signed/notarized production publication cannot be executed locally without the
repository's GitHub Actions credentials; its workflow is validated statically
and by the existing release-tool tests.

## Non-goals

- Adding Linux, iOS, Android, H5, Web, or mini-program targets
- Rewriting Go business logic in Rust or TypeScript
- Redesigning the Desktop UI
- Deleting user-owned documentation migration folders or unrelated untracked
  files
- Publishing a release, pushing a tag, or committing changes
