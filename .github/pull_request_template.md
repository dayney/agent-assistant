## Summary

<!-- What does this change do, and why? Link any related issue. -->

## Type of change

- [ ] Bug fix
- [ ] New feature / enhancement
- [ ] Refactor (no behavior change)
- [ ] Docs
- [ ] Tests / CI / tooling

## Test plan

<!-- How did you verify this? Note any new tests. -->

- [ ] `just ci` is green (the release bar)
- [ ] `just lint` is clean

## Checklist

- [ ] Conventional commit messages with a scope (e.g. `fix(secrets): …`).
- [ ] Tests added/updated for the behavior changed.
- [ ] If this touches canonical or native file writes, I've re-read the
      secret-handling and atomic-write invariants in `AGENTS.md` / `SECURITY.md`.
- [ ] Docs updated if behavior, CLI surface, or capability coverage changed.
