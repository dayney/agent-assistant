# Rule Management Design

**Date:** 2026-08-31

## Outcome

The desktop client manages one canonical Rule document per scope and projects it
through the existing agentsync adapters. The canonical source remains
`~/.agentsync/memory/` for global scope and `<project>/.agentsync/memory/` for
project scope. Native Agent files are destinations, never competing sources of
truth.

## User model

1. Edit the global or project Rule mother template.
2. Select the target Agents and preview every destination path and fidelity gap.
3. Synchronize only when every destination is safe.
4. If a native file contains an unsynchronized edit, stop and show its Diff.
5. Resolve the stop by importing that native Rule into the mother template, or
   by creating a local backup and overwriting it.

Projects are added manually. Their registry is local application state under
`~/.agentsync/.state/agent-assistant/projects.json`; it is never written into a
project or tracked by that project's Git repository.

## Data flow

```text
global/project mother Rule
          |
          v
existing agentsync adapter Render
          |
          v
preview: destination + drift class + Diff + capability loss
          |
     safe | blocked
          |---- import native -> reviewed mother Rule
          |---- backup native -> explicit overwrite
          v
existing render Writer + targets.json state
```

Project discovery uses adapters as the path authority: a memory-only render
reveals each verified destination, then the Core reads those files. Duplicate
paths are collapsed so Agents sharing `AGENTS.md` do not create duplicate source
evidence.

## Drift policy

The three-state classifier compares desired Rule bytes, the last applied hash,
and current destination bytes. `clean`, `pending`, `new`, and `converged` are
safe. `drift`, `conflict`, and `foreign-collision` block synchronization. A
missing destination that was previously owned also blocks as drift/conflict.

The initial synchronize action never accepts a force flag. The only destructive
resolution is a separate `backup-overwrite` request. It first copies every
blocked destination under `~/.agentsync/.state/backups/`, then applies through
the existing render Writer and records normal agentsync state.

## Project analysis

If all discovered native Rule bodies are identical, the Core creates a
deterministic candidate without AI. If they differ, the user can invoke the
local Codex CLI analyzer. It runs with a read-only sandbox, an ephemeral session,
ignored project rules, and a strict JSON output schema. The analyzer returns only
a proposed Markdown mother Rule and notes; it cannot write the canonical source
or native destinations. Saving the proposal is a separate user action.

The prompt requires semantic preservation, explicit conflict notes, no invented
technology, no new dependencies or fonts, and no Agent-specific wrapper text.
If Codex is unavailable or analysis fails, the UI keeps the source evidence and
reports the error; there is no silent fallback presented as AI output.

## Desktop protocol

The sidecar adds these JSON-line methods:

- `rules_get`: load one canonical Rule and its current target preview.
- `rules_save`: atomically save the reviewed mother Rule while preserving
  fragments.
- `rules_sync`: apply safe targets, or perform the explicit
  `backup-overwrite` resolution.
- `rules_import_native`: replace the mother Rule with one selected native Rule.
- `project_import`: validate and register a local project, then discover native
  Rule evidence.
- `project_analyze_rules`: return a deterministic or Codex-generated proposal.

All mutating requests take the existing agentsync global file lock. Responses
contain Rule text and diffs but never resolved secrets.

## Interface

The Rule tab is a working editor, not a summary card. It contains a scope
selector, canonical path, target Agent checkboxes, Markdown editor, save state,
preview table, and synchronize command. Blocked targets open a conflict dialog
with the Diff and the two approved resolutions.

The Projects view adds a path-based import form. Imported projects show detected
native Rule sources, analysis status, a reviewable proposal, and a save action.
No UI element specifies or downloads a font; the application continues to
inherit the system font.
