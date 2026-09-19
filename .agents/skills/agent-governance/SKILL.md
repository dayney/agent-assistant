---
description: Enforce the project's existing technology stack, coding rules, font boundary,
  and explicit adapter capability limits before an AI agent edits code.
name: agent-governance
---

# Agent Governance

Use this skill before any executable change in a governed project. The
canonical Baseline and the selected project Profile are the source of truth;
native `AGENTS.md`, `CLAUDE.md`, `GEMINI.md`, Cursor rules, and similar files
are rendered adapters unless the Profile explicitly says they are canonical.

## Preflight

Before planning or editing:

1. Run `agentsync governance scan --project <root>` and identify the matching
   Profile with `agentsync governance capabilities`.
2. Confirm the documentation root, active task capsule, technology-stack
   document, and affected module knowledge document.
3. Classify the change and inspect the current file plus same-pattern
   implementations. Locate the existing component, state owner, request
   helper, middleware, and styling pattern.
4. Run `agentsync governance check --project <root>` before and after the
   change. Use `agentsync diff --scope project --project <root>` to inspect
   native projections before applying them.

## Reuse and stop boundary

Follow current-file pattern, module pattern, then Profile defaults. Prefer
already-installed dependencies and existing state ownership. If no compliant
implementation exists, stop before editing and ask the user. This includes any
new dependency, state system, styling system, font, provider integration,
workflow, hook, subagent, MCP transport, or cross-layer boundary.

## Fonts

New code inherits the current project font. Never add or explicitly select a
font family: no `next/font/*`, `@font-face`, remote font URL, font file,
font package, named `font-family`, inline `fontFamily`, `font-[Poppins]`, or
`font-[Inter]`. Weight utilities remain allowed. Existing global font loading
is compatibility state and must not be expanded.

## Unified components and capability loss

Keep MCP, skills, workflows, hooks, subagents, and rules in the canonical
Agentsync source. Use secret references such as `${secret:NAME}` or
`${env:NAME}`, never cleartext credentials. Read the capability matrix before
projecting a component. `full`, `partial`, and `unsupported` are explicit:
partial loss must be reported, unsupported output must be skipped, and a
required unsupported component requires user approval before continuing.

Do not overwrite an existing native policy adapter during initialization. The
project manifest records which generated paths are local-only; canonical policy
sources remain visible to Git unless the user explicitly chooses otherwise.
