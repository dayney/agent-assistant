# Agent Governance Baseline

This baseline is the technology-neutral contract shared by every project
Profile and every coding agent adapter.

## Before Any Edit

Before planning or editing executable code, the Agent must:

1. Confirm the application root and Git repository.
2. Confirm the project's documentation root and active task capsule when the
   project uses task capsules.
3. Read the project's technology-stack document and the CodeWiki or equivalent
   knowledge document for the affected module.
4. Classify the change and inspect the current file plus at least one
   same-pattern implementation (two for a new file).
5. Identify the existing component, helper, store, request path, middleware,
   and styling pattern to reuse.

The current file's pattern wins, followed by the module pattern, then the
project Profile. Package presence alone is not evidence that a dependency is
the correct choice.

## Decision Order

Use this order for implementation decisions:

1. The current file's established pattern.
2. The affected module's established pattern.
3. The project's declared technology stack and coding rules.
4. A new approach only after explicit approval by the user.

Prefer existing dependencies, state ownership, UI primitives, request
helpers, middleware, and styling systems. Preserve existing Redux, Zustand,
Context, or other domain boundaries instead of creating a second source of
truth.

## Stop-and-Ask Boundary

If no compliant existing implementation can be found, stop before editing and
ask the user for a decision. This boundary covers any new dependency, state
management approach, styling system, font, provider integration pattern,
workflow, hook, subagent, MCP transport, or cross-layer boundary. Never invent
a fallback silently and never defer the approval until after code is written.

## Font Boundary

New work inherits the current project font. Do not add, load, bundle, or
explicitly select a font family. This forbids `next/font/*`, `@font-face`,
remote font URLs, font files or font packages, named `font-family` declarations,
inline `style={{ fontFamily: ... }}`, and arbitrary classes such as
`font-[Poppins]` or `font-[Inter]`. Weight utilities such as `font-medium` and
`font-bold` remain allowed. Existing global font loading may be preserved for
compatibility but must not be expanded.

## MCP, Skills, Workflows, Hooks, and Subagents

Manage these components from the canonical source and project Profile. Native
agent files are rendered adapters, not policy sources. MCP credentials must
remain references such as `${secret:NAME}` or `${env:NAME}`; never commit a
secret value, token, password, or connection string. Remote MCP endpoints must
use HTTPS unless the user explicitly approves a documented local exception.

Each adapter reports a capability state for every managed component:

- **full** means the native representation preserves the component contract.
- **partial** means translation loses documented fields or scope and the loss
  is reported.
- **unsupported** means no honest representation exists; the component is
  skipped and the user is told why.

If the target agent is `unsupported` for a required component, stop and ask
before continuing. Never pretend that a lossy or skipped projection is full
fidelity.

## Verification

Run the Profile's formatter, lint, type, test, and build checks relevant to the
changed surface. Run the governance verifier and report every failed, skipped,
or unavailable check. A successful build alone is not evidence of lint or type
correctness when the project build suppresses those errors.
