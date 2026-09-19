# Security Policy

## Reporting a vulnerability

Report security issues privately through GitHub Security Advisories using the
repository's **Report a vulnerability** action. Do not open a public issue with
exploit details, credentials, or private configuration. Include the Desktop
version shown in the application, operating system, reproduction steps, and
impact.

## Scope and threat model

agent-assistant is a local Desktop application. Its React interface talks to an
embedded Rust Core through typed Tauri commands. The Core reads canonical
configuration and selected native agent files on the same machine. The primary
security boundaries are local file access, secret references, native Rule
writes, Tauri command responses, and signed updates.

### Canonical configuration and secrets

Canonical configuration lives in `~/.agentsync/` and optional project
`.agentsync/` trees. The Desktop Core treats `${secret:...}` and `${env:...}` as
opaque references: it does not decrypt or substitute them. Snapshot responses
return references only, redact URL paths and queries, and never include secret
cleartext. Do not commit credentials or `~/.agentsync/.state/` to a public
repository.

### Filesystem writes

Native Rule writes use same-directory temporary files, file synchronization,
atomic replacement, and parent-directory synchronization where supported.
Existing drifted files are backed up before an explicit overwrite, including
foreign content that appears after preview. Symlinked and other non-regular
destinations are rejected, as are symlinked parent components beneath an
imported project's root. Temporary files, backups, and the imported-project
registry use owner-only permissions while normal Rule files are finalized as
`0644` on Unix. Mutations use a cross-process lock with a bounded wait.

### Embedded Desktop Core

The Rust Core is compiled into the Tauri application; no external executable or
stdio protocol is shipped. Requests and responses remain explicitly typed. The
Desktop API must not expose secret values, private configuration outside the
documented summaries, or unrestricted filesystem operations to the React
layer. Production mode must not silently fall back to demo data.

### Desktop update trust

Automatic updates are registered only for signed production builds that carry
the release updater configuration. Browser demo mode, development mode, and
unsigned local builds do not perform automatic checks.

Production updates require a Tauri updater signature matching the public key
embedded at release time. The private updater key remains in GitHub Actions and
is separate from Apple code signing/notarization and Windows Authenticode. The
release workflow requires both supported platform artifacts and detached
signatures, uploads all assets first, publishes `latest.json` last, and only
then publishes the complete GitHub Release. This prevents clients from seeing a
manifest that references a partial release.

The application checks the stable release channel only. Installation requires
user confirmation and remains blocked while editable Rule state is unsaved.

## Supported versions

Until the first stable `v1.0.0` release, security fixes target the latest tagged
Desktop version.
