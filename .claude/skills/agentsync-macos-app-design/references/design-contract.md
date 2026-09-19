# Agentsync macOS Design Contract

This contract turns a visual request into an implementation-ready desktop
interaction design. It applies to the agentsync desktop app in this repository,
not to generic websites or other projects.

## Product truth

Read these before designing:

- `docs/superpowers/specs/2026-08-28-agent-assistant-desktop-design.md`
- the current task spec under `docs/superpowers/specs/`
- `desktop/package.json`
- `desktop/src-tauri/tauri.conf.json`
- the affected files under `desktop/src/`, `desktop/src-tauri/`, and the Go Core

The app manages one canonical agentsync source and projects it into agent-native
destinations. The interface must expose origin, destination, fidelity, drift,
and the consequence of a write. Native files are projections, not silent
alternate sources of truth.

For Rule management specifically:

- one canonical Rule exists per global or imported-project scope;
- editing, saving, previewing, synchronizing, importing native content, and
  backup-overwrite are distinct operations;
- changing the draft, scope, or selected destinations invalidates a preview;
- safe and blocked destinations require visibly different actions;
- Go Core responses are authoritative after every mutation.

## Required design deliverable

Every design or implementation plan must contain the following sections. If a
section does not apply, say why instead of omitting it.

### 1. Product and Mac identity

State the user's repeated desktop job, the primary object being manipulated,
and why the proposed layout fits a resizable Mac window. Name the existing
stack and the product safety constraints being preserved.

### 2. Information and window model

Define the main window hierarchy: sidebar or source list, toolbar, content,
optional inspector, status area, sheets, popovers, and any justified secondary
window. Explain resizing at the configured default and minimum window sizes.
Specify what window, pane, selection, and draft state restores after relaunch.

Prefer a standard title bar and a single coherent main window until a real user
workflow justifies custom chrome or another window. Use a sheet for decisions
that must block the current window, such as conflict resolution or destructive
confirmation. Use a popover only for lightweight, reversible choices.

### 3. Object and affordance map

Provide a table with one row for each important object or action:

| Object or action | Visible control | Menu command / shortcut | Selection and focus | Copy, paste, or drag | Persistence | Accessibility |
| --- | --- | --- | --- | --- | --- | --- |

Cover at least navigation, scope selection, editing, save, preview, destination
selection, diff review, synchronization, conflict resolution, import, and any
destructive operation. Do not treat toolbar buttons as the complete command
model.

### 4. Command and menu model

Provide a command table:

| Command | Menu placement | Shortcut | Enabled when | Disabled explanation | Core operation |
| --- | --- | --- | --- | --- | --- |

Use standard macOS command meanings where they apply. Include menu equivalents
for important toolbar actions, predictable keyboard navigation, and current
context in command enablement. Avoid inventing shortcuts that conflict with
system conventions. Give a product-specific command no shortcut by default;
propose one only when repeated-use value is demonstrated and the combination is
audited against system and existing app commands. Mark unverified shortcuts as
candidates, not decisions. Verify new Tauri menu claims before implementation.

### 5. State and safety matrix

Describe loading, empty, ready, dirty, saving, previewing, safe, partially
supported, blocked, syncing, success, failure, Core unavailable, and explicit
demo states. For each state, identify the primary action, unavailable actions,
preserved user input, and recovery path.

Never encode capability, drift, or danger by color alone. Pair semantic color
with text, an icon or shape, and an accessible description. A blocked conflict
must not degrade into a generic force action.

### 6. Visual system

Start from semantic roles such as window background, sidebar material, content
surface, separator, primary text, secondary text, accent, focus, success,
warning, and destructive. Make them respond to system appearance rather than
copying a fixed Apple marketing palette.

Use inherited system typography, compact desktop density, stable control sizes,
clear selection, and restrained separators. Prefer unframed work surfaces to
nested cards. Use familiar symbols only when the project already has a suitable
source; otherwise use clear text or existing assets rather than adding an icon
dependency by default. Motion must explain state and respect reduced motion.

Address light and dark appearance, increased contrast, reduced transparency,
and reduced motion in the design even when implementation is staged. Identify
an unsupported appearance as a known gap instead of calling the result complete.

### 7. Architecture and file impact

Map each state and operation to its owner:

- React: views, accessible controls, drafts, selection, focus, dialogs, and
  client-side request coordination;
- Rust/Tauri: typed transport and verified platform integration;
- Go Core: canonical data, capabilities, drift, diffs, validation, locking,
  backup, and writes.

List existing files to change and justify any new module. Do not invent request
or response fields without inspecting the current implementation.

### 8. Verification plan

Use `references/verification.md`. Include concrete keyboard, accessibility,
window-size, appearance, state-transition, Core-failure, and destructive-flow
checks in addition to build and unit tests.

## Common design failures

- Recoloring the current dashboard without redesigning desktop behavior.
- Specifying fixed Apple website colors, blue pills, blur, or press-scale motion
  as proof of Mac quality.
- Adding an icon or editor package before checking installed capabilities.
- Hiding menus, shortcuts, selection, focus, clipboard behavior, or restoration.
- Treating mobile breakpoints below the Tauri minimum width as verification.
- Recomputing capability or drift policy in React or Rust.
- Assuming light-only output is complete without naming the system-appearance gap.
