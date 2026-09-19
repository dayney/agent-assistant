# Project Coding Rules (agentsync)

## Secret Handling Invariants (CRITICAL)
- **One field list**: Every secret-bearing canonical field MUST be enumerated in `walkSecretFields` (`internal/secrets/walk.go`).
- **One dest→source path**: ALL write-backs (dest→source) MUST go through `capture.Capture` (`internal/capture`). Do not call `source.Write*` directly from a write-back path to bypass it.
- **Fail-Fast**: If a live vault secret value would be written verbatim, or a `${secret:K}` is missing, `capture.Capture` will refuse the write. Let it fail rather than persisting cleartext.

## Documentation Sync (CRITICAL)
- If you change behavior, update the docs in the *same* commit. A reviewer should never have to wonder whether the prose or the code is the source of truth. Treat a stale doc as a bug.
- Any change to `Adapter`, CLI command/flag, or capability coverage must update the corresponding markdown files in `docs/` and `website/src/content/docs/`.

## Testing
- **Filesystem in tests**: Always use `afero.NewMemMapFs()` or `t.TempDir()`. NEVER use `os.UserHomeDir()` in `_test.go`.
- **Environment**: FS-touching tests MUST run in the container (`AGENTSYNC_TEST_IN_CONTAINER=1`). Call `testenv.RequireContainer(t)`.

## Code Conventions
- Use stdlib testing only. Table-driven with a `name` field and `t.Run`.
- Errors wrap with `fmt.Errorf("doing X: %w", err)`. Match with `errors.Is/As`.
- Before completing tasks, always run `just lint` to format/tidy and run golangci-lint. DO NOT leave unformatted or failing code.
