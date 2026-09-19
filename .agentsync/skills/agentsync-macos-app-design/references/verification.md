# macOS Desktop Verification

Verification must prove both agentsync behavior and Mac interaction quality.
Visual resemblance alone is not acceptance.

## Evidence order

Use evidence in this order:

1. Current repository source, task specs, manifests, and tests for product
   behavior and installed capability.
2. Current official Apple Human Interface Guidelines for platform behavior.
3. Current official Tauri documentation for APIs and platform integration.

Do not infer an installed API from a sample skill, a blog post, or memory. When
adding a new platform claim, verify it against the dependency version in this
repository and the current official documentation.

Primary references:

- Apple, Designing for macOS:
  https://developer.apple.com/design/human-interface-guidelines/designing-for-macos
- Apple, Menus:
  https://developer.apple.com/design/human-interface-guidelines/menus
- Apple, Toolbars:
  https://developer.apple.com/design/human-interface-guidelines/toolbars
- Apple, Sidebars:
  https://developer.apple.com/design/human-interface-guidelines/sidebars
- Apple, Keyboards:
  https://developer.apple.com/design/human-interface-guidelines/keyboards
- Tauri JavaScript API:
  https://v2.tauri.app/reference/javascript/api/
- Tauri window and menu guidance:
  https://v2.tauri.app/learn/window-menu/
- Tauri window-state plugin:
  https://v2.tauri.app/plugin/window-state/

## Review matrix

### Product safety

- Canonical scope and destination provenance remain visible.
- Capability loss is explicit for full, partial, and unsupported destinations.
- Any draft, scope, or target change invalidates a prior preview.
- Safe synchronization and blocked conflict resolution have different paths.
- Backup-overwrite requires a separate, explicit confirmation.
- Core errors preserve the draft and never fall back silently to demo data.
- No resolved secret appears in state, UI, logs, diffs, fixtures, or captures.

### Window and layout

- Test the configured default size, configured minimum size, and a wider window.
- Resize continuously; panes must not overlap, strand actions, or hide status.
- Sidebar, inspector, and toolbar controls have stable dimensions and sensible
  collapse behavior at widths the native window can actually reach.
- Verify relaunch restoration for window geometry, pane visibility, selection,
  and recoverable draft state when those behaviors are in scope.

### Commands and input

- Every important toolbar action has the intended menu and keyboard path.
- Command enablement follows selection, focus, dirty state, preview freshness,
  capability, and drift.
- Tab and reverse-Tab order are coherent. Focus is visible and returns to the
  invoking control after a sheet or popover closes.
- Arrow-key navigation, text editing, selection, copy/paste, and any drag/drop
  behavior match the object model and do not trigger destructive work.
- Escape cancels only the current transient interaction; Return does not become
  a hidden destructive default.

### Accessibility and appearance

- Operate the complete primary flow without a pointer.
- Inspect names, roles, states, descriptions, live updates, and error association
  with macOS accessibility tooling or the closest available webview inspection.
- Verify light and dark appearance, increased contrast, reduced transparency,
  and reduced motion. Record staged gaps explicitly.
- Text and controls remain legible without relying on color, hover, or animation.
- Decorative icons are hidden from assistive technology; meaningful icons have
  an accessible name and, when unfamiliar, a tooltip.

### Behavior and failures

- Exercise loading, empty, dirty, stale preview, partial capability, blocked
  drift, success, failure, Core unavailable, and explicit demo states.
- Interrupt or fail each mutation and confirm the user can understand recovery.
- Confirm stale responses cannot replace a newer scope, draft, or target set.
- Confirm Go Core revalidates write safety; UI state is never the security gate.

## Implementation checks

Derive exact commands from the current manifests and repository task runner.
Run the narrowest relevant unit tests first, then the desktop type/build checks,
Rust checks, and affected Go tests. Inspect the rendered app at all required
window sizes and capture evidence for the changed workflow. A passing build
does not replace interaction or accessibility verification.
