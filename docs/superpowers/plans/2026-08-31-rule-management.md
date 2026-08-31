# Rule Management Implementation Plan

> Execute this plan in one Rule-area commit. Do not begin Skill management.

1. Add failing Go tests for canonical Rule load/save, adapter-derived targets,
   drift blocking, native import, backup-overwrite, local project registry, and
   deterministic/AI analysis boundaries.
2. Add failing frontend tests for the Rule editor model, preview state, conflict
   resolution, and manual project import.
3. Extract the production adapter registry into a shared internal package so the
   CLI and desktop Core use the same complete adapter set.
4. Implement `internal/desktopcore/rules.go` using `source`, `render`, `drift`,
   `state`, and the global lock. Keep the canonical source and targets state as
   the only write authorities.
5. Implement the optional local Codex analyzer with an injectable command runner
   and strict JSON schema. Tests use a fake runner; no test calls a model.
6. Extend the sidecar request protocol and Tauri IPC commands for Rule and
   project-import operations.
7. Replace the Rule prototype with the editor, target preview, blocked-Diff
   dialog, and explicit resolution actions. Add the manual project import and
   proposal review flow.
8. Update README, architecture, components, user guide, website reference, and
   changelog in the same commit.
9. Run focused Go tests, frontend tests/build, Rust format/check, full relevant
   Go tests, a Tauri release build, and real macOS UI verification.
10. Commit as one conventional Rule-area commit, then stop before Skill work.
