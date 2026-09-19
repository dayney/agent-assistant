# agent-assistant Desktop

The Desktop app combines a React interface with an embedded Rust Core in the
Tauri process. It does not maintain a second canonical configuration model.

## Code layout

The repository keeps one Desktop product and one Tauri crate:

- `src/app/` owns the React shell and navigation.
- `src/views/` and `src/components/` own screens and reusable presentation.
- `src/core/` owns typed Tauri requests, responses, and client-side Rule state.
- `src/update/` owns the signed updater state machine.
- `src-tauri/src/lib.rs` is the narrow Tauri command and native-menu boundary.
- `src-tauri/src/core/desktop/` coordinates snapshots, Rule workflows, and
  project analysis.
- `src-tauri/src/core/source/` owns canonical on-disk fidelity.
- `src-tauri/src/core/{adapter,drift,state,iox,jsonkeys,paths}.rs` owns Agent
  projection, drift policy, persistence, and filesystem safety.

Keep domain decisions in the Rust Core and presentation state in React. Split a
large module inside these boundaries only when it has distinct behavior and
tests; do not introduce another application package or companion runtime merely
to reorganize files.

## Local development

```sh
npm install
npm run tauri:dev
```

`npm run dev:demo` is limited to UI review. Browser/demo mode never contacts the
update service. `npm test` covers the React update state machine, plugin adapter,
release configuration, manifest generation, and the Desktop-only repository
boundary. Rust Core tests run through `cargo test` or `just test-desktop`.

## Native global Rules

The real Tauri app discovers local Rule files separately from the canonical
mother document. Its source list is above the editor; **View original** reads
one selected file without importing or writing it. Browser/demo mode uses
sample data and cannot read local files, so it does not offer a preview button.
In the real Desktop app, **重新扫描** refreshes the native source list without
changing the canonical mother document. Antigravity, Antigravity IDE, and
Antigravity CLI are distinct products but all use `~/.gemini/GEMINI.md` for
global Rules; the same file also serves Gemini CLI. Cursor's account-synced User Rules are not
readable from a verified local file, while machine-local `~/.cursor/rules`
is listed separately. An empty canonical mother document does not imply an
empty native configuration. Review sources before deciding what belongs in
the canonical document; discovery itself never merges or overwrites them.

For MCP, the IDE and CLI share `~/.gemini/config/mcp_config.json`. The app
checks whether the Antigravity desktop app's
`~/.gemini/antigravity/mcp_config.json` points to that file before reporting it
as shared; otherwise the UI says the link is unverified. Each
server appears only once in the redacted native summary. Discovery does not
import MCP definitions into the canonical source.

The Desktop MCP view has three explicit layers. **Global base MCP** is the
canonical source at `~/.agentsync/mcp/<id>.toml`; one definition is maintained
there and projected to selected Agents. **Agent-native MCP** is a de-duplicated
read-only inventory grouped by the same id, runtime, endpoint, and credential
references, with actual Agent targets listed once. **Project MCP** comes from
project `.agentsync/mcp/` and stays project-local until the user selects
**转为全局**. Promotion is locked and atomic, requires a known Recipe, and
rejects an existing global server with a different definition instead of
overwriting it. When importing from an Agent-native file, secret-like plaintext
values are converted to `${env:KEY}` references before they enter the canonical
source; canonical/project writes still reject plaintext credentials. Tokenless
Recipes are valid and do not require a secret.

Skills use separate global roots: Antigravity uses `~/.gemini/config/skills`,
Antigravity IDE uses `~/.gemini/antigravity/skills` (plus its locally found
`global_skills` directory, whose activation is unverified), and Antigravity CLI
uses `~/.gemini/antigravity-cli/skills`. The Skills view keeps their identities
separate and inventories only folder/manifest metadata.
Cursor also reads user-level skills from `~/.agents/skills`,
`~/.claude/skills`, and `~/.codex/skills` in addition to its own
`~/.cursor/skills` ([Cursor skills documentation](https://cursor.com/docs/skills)).
The inventory attributes those shared directories to Cursor as compatible
sources without copying files or treating a missing Cursor-only directory as
proof that Cursor has no skills.

Workflows are inventoried separately from Skills. The IDE's locally found
`~/.gemini/antigravity/global_workflows/*.md` files appear as read-only source
metadata. No independent Workflow directory is claimed for the Antigravity app
or CLI, and no workflow content is imported automatically.
The file count includes only nonempty Markdown files; empty files, missing
directories, and unreadable paths retain distinct statuses in the inventory.

Hooks are inventoried without returning commands or configuration values.
For verified Claude and Gemini settings files, the scanner checks only whether
a nonempty top-level `hooks` object exists (up to 1 MiB); it distinguishes absent
definitions from invalid files without asserting that any hook is active.
Antigravity and Antigravity IDE both use `~/.gemini/config/hooks.json`, counted
as one physical source. The CLI's `~/.gemini/antigravity-cli/settings.json`
and `plugins/*/hooks.json` are separate sources. The CLI settings remain
unverified until their Hook schema is confirmed. Known Claude, Cursor, and
Gemini paths are also listed, with other agents explicitly marked as having no
verified user-level path. None of these sources is automatically imported, and
the file inventory is not an assertion that any hook is active.

The Subagents view inventories `.md` definitions in Antigravity's
`~/.gemini/config/agents` and its global plugin `agents/` directories. The CLI
also uses the shared global definitions, with its own plugin directory listed
separately. Antigravity IDE has no verified independent global definition path
in this inventory; it is not silently mapped to the other app's files. Cursor's
`~/.cursor/agents` and Claude's `~/.claude/agents` are checked independently.
Only names and file states reach the UI, never agent prompts, and discovered
plugin files are not assumed enabled or imported.
Cursor also checks `~/.claude/agents` and `~/.codex/agents` as compatible
user-level sources ([Cursor Subagents](https://cursor.com/docs/subagents)).
Missing compatibility directories remain visible as missing, not as definitions.

The Agent view summarizes every product represented in the native source
inventory, including Cursor's account/local Rule aliases under one product.
It counts distinct confirmed source paths and marks unverified paths separately;
empty directories and absent Hook definitions are not counted as discovered
configuration. A native source does not establish that the app is installed,
enabled, or managed. The configured-adapter capability matrix is based only on
canonical/project agent settings and remains separate from native discovery.

## Automatic updates

Automatic update checks are enabled only in signed release builds carrying the
release updater configuration. The app checks once at startup and remains quiet
when no update is available or a background network check fails. Users can run
**Check for Updates…** from the native application menu or the version entry in
the sidebar to get an explicit result.

When an update is available, the app shows the new version and release notes.
Installation requires confirmation, reports download progress, verifies the
Tauri updater signature, installs the package, and relaunches. If the Rule
workbench contains unsaved edits, installation remains disabled until those
edits are saved or discarded.

The release channel is the stable GitHub `latest` release. Pre-release tags are
rejected by the Desktop release preflight.

## Release trust and credentials

`.github/workflows/desktop-release.yml` builds macOS ARM64 and Windows x64 in
separate signed jobs. A final job publishes installers and updater packages,
then publishes `latest.json` last. The workflow is enabled by the repository
variable `DESKTOP_RELEASE_ENABLED=true` and fails before building if any of the
following GitHub Actions secrets is missing:

- `TAURI_UPDATER_PUBLIC_KEY`
- `TAURI_SIGNING_PRIVATE_KEY`
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
- `APPLE_CERTIFICATE`
- `APPLE_CERTIFICATE_PASSWORD`
- `APPLE_SIGNING_IDENTITY`
- `APPLE_ID`
- `APPLE_PASSWORD`
- `APPLE_TEAM_ID`
- `KEYCHAIN_PASSWORD`
- `WINDOWS_CERTIFICATE`
- `WINDOWS_CERTIFICATE_PASSWORD`
- `WINDOWS_TIMESTAMP_URL`

The updater public key is injected into a temporary Tauri release override. The
private updater key is available only to the build jobs. Neither key is replaced
with a placeholder, and local builds without release credentials do not become
trusted update publishers.
