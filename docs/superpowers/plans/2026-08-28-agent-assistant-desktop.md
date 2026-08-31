# agent-assistant Desktop Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a macOS-first Tauri 2 client that visualizes the shared governance source, Agent projections, and Project Profiles using the approved Chinese prototype.

**Architecture:** A React/TypeScript frontend runs inside a Tauri 2 shell. A typed `CoreClient` boundary keeps UI state independent from the Go Agentsync Core. Real mode invokes the Go sidecar over Tauri IPC; an explicitly labeled demo client remains available for visual review.

**Tech Stack:** Tauri 2, Rust, React 19, TypeScript, Vite, native CSS tokens, system-inherited font, existing Go Agentsync Core.

**Spec:** `docs/superpowers/specs/2026-08-28-agent-assistant-desktop-design.md`

## Global Constraints

- Preserve the existing Go Core and canonical Agentsync source; native files remain rendered output.
- Use Global → Agent → Project provenance and report `full`, `partial`, and `unsupported` explicitly.
- Demo mode must be visible and must not silently replace a Core error.
- Do not add, load, bundle, or explicitly select fonts; all new text inherits the current system font.
- Do not add a new backend, cloud service, or database in this vertical slice.
- Use semantic controls, visible focus styles, keyboard navigation, and responsive layouts.
- Run TypeScript/Vite and Rust checks separately; a successful frontend build is not a substitute for Core checks.

### Task 1: Scaffold the desktop workspace

**Files:**
- Create: `desktop/package.json`, `desktop/index.html`, `desktop/tsconfig.json`, `desktop/vite.config.ts`
- Create: `desktop/src-tauri/Cargo.toml`, `desktop/src-tauri/tauri.conf.json`, `desktop/src-tauri/src/main.rs`
- Create: `desktop/src/main.tsx`, `desktop/src/app.css`

- [ ] **Step 1: Write the failing toolchain checks**

Create scripts that require `tsc --noEmit` and Vite build, and a Rust shell that exposes a `health` command.

- [ ] **Step 2: Run the checks to verify the scaffold is missing**

Run `cd desktop && npm run typecheck`; expected failure because `package.json` and the source entry do not exist.

- [ ] **Step 3: Create the minimal Tauri/Vite scaffold**

Use React 19, TypeScript, Vite, and Tauri 2 dependencies. Keep Rust commands small and return structured errors.

- [x] **Step 4: Run frontend and Rust checks**

Run `npm run typecheck`, `npm run build`, and `cargo check --manifest-path src-tauri/Cargo.toml`; all must pass.

### Task 2: Define the Core client contract and demo snapshot

**Files:**
- Create: `desktop/src/core/model.ts`, `desktop/src/core/client.ts`, `desktop/src/core/demo-client.ts`
- Test: `desktop/src/core/model.test.ts`

- [ ] **Step 1: Write failing model tests**

Test capability states, provenance order, and explicit `mode: 'demo'` metadata.

- [ ] **Step 2: Run `npm run test` and confirm the expected failure**

The test must fail because the model and client do not exist.

- [ ] **Step 3: Implement typed models and demo client**

Expose `CoreClient.getSnapshot()` and `CoreClient.previewApply()`; the demo client must return the approved Chinese prototype data and no secret values.

- [x] **Step 4: Run the model tests and typecheck**

Run `npm run test -- src/core/model.test.ts` and `npm run typecheck`; both must pass.

### Task 3: Implement the Chinese application shell

**Files:**
- Create: `desktop/src/app/App.tsx`, `desktop/src/app/navigation.ts`, `desktop/src/components/StatusBadge.tsx`, `desktop/src/components/ProvenanceChain.tsx`
- Modify: `desktop/src/main.tsx`, `desktop/src/app.css`

- [ ] **Step 1: Write failing render tests for navigation and status text**

Assert that the shell renders `总览`, `全局配置`, `Agent`, `项目`, and `活动记录`, and that status labels contain text in addition to color.

- [ ] **Step 2: Run the tests and observe the missing component failure**

Run `npm run test -- src/app`; expected failure before the shell exists.

- [ ] **Step 3: Implement the shell and responsive layout**

Use one sidebar/topbar shell, semantic navigation, native buttons, tokenized CSS, and system-inherited fonts. Keep one primary action per page.

- [x] **Step 4: Run tests and build**

Run `npm run test -- src/app`, `npm run typecheck`, and `npm run build`.

### Task 4: Implement Global, Agent, Project, and Activity views

**Files:**
- Create: `desktop/src/views/OverviewView.tsx`, `desktop/src/views/GlobalView.tsx`, `desktop/src/views/AgentsView.tsx`, `desktop/src/views/ProjectsView.tsx`, `desktop/src/views/ActivityView.tsx`
- Modify: `desktop/src/app/App.tsx`, `desktop/src/app.css`

- [ ] **Step 1: Add failing view tests**

Cover matrix labels, project Profile rows, source chain, and demo-mode banner.

- [ ] **Step 2: Verify the tests fail for absent views**

Run `npm run test -- src/views`; expected module/render failures.

- [ ] **Step 3: Implement views from the approved prototype**

Keep data in the typed snapshot, avoid duplicate state, and handle loading/error/demo states explicitly.

- [x] **Step 4: Run focused tests and build**

Run `npm run test -- src/views`, `npm run typecheck`, and `npm run build`.

### Task 5: Add Tauri IPC boundary for the Go Core

**Files:**
- Create: `desktop/src-tauri/src/core.rs`, `desktop/src/core/tauri-client.ts`
- Modify: `desktop/src-tauri/src/main.rs`, `desktop/src/core/client.ts`, `desktop/src/app/App.tsx`

- [ ] **Step 1: Add failing client contract tests**

Test that a Core error produces a visible error state and never silently swaps to demo data.

- [ ] **Step 2: Run the tests to capture the missing IPC implementation**

Run `npm run test -- src/core/client.test.ts`; expected failure before the adapter exists.

- [ ] **Step 3: Implement explicit Core mode selection**

Use Tauri `invoke('get_workspace_snapshot')` in real mode. Use the demo client only when `VITE_CORE_MODE=demo` is explicit. Return errors with a stable code/message shape.

- [x] **Step 4: Run frontend and Rust checks**

Run `npm run typecheck`, `npm run build`, and `cargo check --manifest-path src-tauri/Cargo.toml`.

### Task 6: Verify UI quality and document the client

**Files:**
- Modify: `README.md`, `docs/architecture.md`, `docs/components.md`, `CHANGELOG.md`
- Test: `desktop/` build/type checks and manual screenshot evidence in the task capsule

- [x] **Step 1: Run all frontend and Rust verification commands**

Run `cd desktop && npm run typecheck && npm run build && cargo check --manifest-path src-tauri/Cargo.toml`.

- [ ] **Step 2: Inspect keyboard focus, narrow layout, and demo/error states**

Use a browser preview at 1024px, 736px, and 360px; record any overlap or clipped text and fix it before completion.

- [x] **Step 3: Update docs and changelog**

Document the desktop shell, connected Core boundary, explicit demo mode, and the
remaining apply/drift integration boundary.

- [x] **Step 4: Run `git diff --check` and review the changed file list**

Confirm no fonts, secrets, generated native adapters, or unrelated files were added.

## Implementation status

Tasks 1–6 are implemented for the read-only connected slice. The sidecar
supports `snapshot` and `preview`, uses `schemaVersion: 1`, and redacts native
MCP endpoints and values. Apply, drift projection, file watching, and write-back
remain intentionally outside this first connected slice.
