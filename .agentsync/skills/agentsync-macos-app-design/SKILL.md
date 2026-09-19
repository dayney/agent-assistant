---
name: agentsync-macos-app-design
description: Use when designing, reviewing, or implementing the macOS-first agentsync desktop client under desktop/, especially its navigation, Rule workflows, windows, menus, keyboard interaction, visual polish, accessibility, or Apple HIG alignment in the existing Tauri and React stack.
---

# Agentsync macOS App Design

Design a Mac workflow, not an Apple-colored web dashboard. Platform behavior,
agentsync's safety model, and task clarity come before visual styling.

## Scope

Apply this skill only to this repository's desktop product under `desktop/` and
the directly supporting Tauri or Go Core boundary. Do not use it for the docs
website, CLI-only work, another repository, iOS, or a SwiftUI rewrite.

## Load the right context

Before making a design claim or editing code:

1. Read `references/design-contract.md`.
2. Read the current desktop design spec and the task-specific spec under
   `docs/superpowers/specs/`.
3. Inspect the affected React, CSS, Tauri, Core client, and Go Core files plus
   the current dependency manifests. Treat source as authoritative when a spec
   and implementation differ.
4. For implementation or review, also read `references/verification.md`.

## Route the task

| Request | Required result |
| --- | --- |
| Explore or design | Complete the design deliverable in `design-contract.md`. |
| Implement or refactor | Map the deliverable to existing modules, preserve layer ownership, then make the smallest coherent change. |
| Review | Audit behavior, safety, accessibility, and verification before visual preference. |
| Add native behavior | Verify the installed Tauri version and current official API before proposing an integration. |

## Project boundaries

- Keep the existing Tauri + React + TypeScript + native CSS + Go Core stack.
- Inherit the project font. Do not add a font, styling system, icon package,
  editor framework, or other dependency without explicit user approval.
- React owns presentation and draft interaction state. Rust owns transport and
  necessary platform integration. Go Core owns canonical data, capabilities,
  drift, diffs, locking, backup, and writes.
- Preserve preview-before-write, explicit capability loss, provenance, blocked
  conflicts, and separate backup-and-overwrite confirmation.
- Never expose resolved secret values in UI state, logs, diffs, fixtures, or
  screenshots. Demo data must remain explicit and never mask a Core failure.

## Mac-native decision order

For every important object or action, decide in this order:

1. What is the user's object, selection, and current context?
2. What is the standard visible control?
3. What menu command and keyboard path represent the same action?
4. What persists across focus changes, window resizing, and relaunch?
5. How do keyboard-only and assistive-technology users understand and operate it?
6. Only then choose spacing, color, material, radius, and motion.

Use semantic, appearance-aware tokens and restrained hierarchy. Do not copy
Apple.com marketing colors, pill buttons, product-page spacing, or decorative
glass effects as a shortcut to "Apple style."

## Stop boundaries

Stop and get explicit approval before adding a dependency, replacing the
technology stack, introducing custom window chrome, or moving domain decisions
out of Go Core. Stop a write path that hides capability loss, bypasses preview,
turns a blocked conflict into generic force sync, or could reveal a secret.
