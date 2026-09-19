import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import { createUpdaterManifest } from "./create-updater-manifest.mjs";
import { createUpdaterConfig } from "./write-updater-config.mjs";

test("creates a signed updater release override", () => {
  assert.deepEqual(
    createUpdaterConfig({ version: "1.2.3", publicKey: "PUBLIC-KEY\n" }),
    {
      version: "1.2.3",
      bundle: { createUpdaterArtifacts: true },
      plugins: {
        updater: {
          pubkey: "PUBLIC-KEY",
          endpoints: [
            "https://github.com/spxrogers/agentsync/releases/latest/download/latest.json",
          ],
        },
      },
    },
  );
});

test("refuses to build an updater config without a public key", () => {
  assert.throws(
    () => createUpdaterConfig({ version: "1.2.3", publicKey: "  " }),
    /public key is required/,
  );
});

test("creates latest.json from both signed platform artifacts", () => {
  const root = mkdtempSync(join(tmpdir(), "agentsync-updater-"));
  const macDirectory = join(root, "macos");
  const windowsDirectory = join(root, "windows");
  mkdirSync(macDirectory);
  mkdirSync(windowsDirectory);
  writeFileSync(join(macDirectory, "agent-assistant.app.tar.gz"), "mac");
  writeFileSync(
    join(macDirectory, "agent-assistant.app.tar.gz.sig"),
    "mac-signature\n",
  );
  writeFileSync(join(windowsDirectory, "agent-assistant.nsis.zip"), "windows");
  writeFileSync(
    join(windowsDirectory, "agent-assistant.nsis.zip.sig"),
    "windows-signature\n",
  );

  const manifest = createUpdaterManifest({
    artifactRoot: root,
    version: "v1.2.3",
    tag: "v1.2.3",
    notes: "Release notes",
    pubDate: "2026-09-16T00:00:00.000Z",
  });

  assert.deepEqual(manifest, {
    version: "1.2.3",
    notes: "Release notes",
    pub_date: "2026-09-16T00:00:00.000Z",
    platforms: {
      "darwin-aarch64": {
        signature: "mac-signature",
        url: "https://github.com/spxrogers/agentsync/releases/download/v1.2.3/agent-assistant.app.tar.gz",
      },
      "windows-x86_64": {
        signature: "windows-signature",
        url: "https://github.com/spxrogers/agentsync/releases/download/v1.2.3/agent-assistant.nsis.zip",
      },
    },
  });
});

test("refuses an incomplete updater manifest", () => {
  const root = mkdtempSync(join(tmpdir(), "agentsync-updater-"));
  writeFileSync(join(root, "agent-assistant.app.tar.gz"), "mac");
  writeFileSync(join(root, "agent-assistant.app.tar.gz.sig"), "signature");

  assert.throws(
    () =>
      createUpdaterManifest({
        artifactRoot: root,
        version: "1.2.3",
        tag: "v1.2.3",
        notes: "",
        pubDate: "2026-09-16T00:00:00.000Z",
      }),
    /missing signed updater artifact for windows-x86_64/,
  );
});

test("Desktop-only release publishes latest.json after both signed platforms", () => {
  const repositoryRoot = resolve(
    dirname(fileURLToPath(import.meta.url)),
    "..",
    "..",
  );
  const workflow = readFileSync(
    join(repositoryRoot, ".github", "workflows", "desktop-release.yml"),
    "utf8",
  );
  const releaseWorkflow = readFileSync(
    join(repositoryRoot, ".github", "workflows", "release.yml"),
    "utf8",
  );

  assert.match(workflow, /needs: \[preflight, macos, windows\]/);
  assert.match(workflow, /TAURI_UPDATER_PUBLIC_KEY/);
  assert.match(workflow, /TAURI_SIGNING_PRIVATE_KEY/);
  assert.match(workflow, /AGENT_ASSISTANT_UPDATER_ENABLED: "true"/);
  assert.match(workflow, /VITE_UPDATER_ENABLED: "true"/);
  assert.doesNotMatch(workflow, /release-tag\.sh --(?:validate-only|package-version)/);
  assert.match(workflow, /create-updater-manifest\.mjs/);
  assert.match(workflow, /gh release create "\$TAG" --draft/);
  assert.ok(
    workflow.indexOf("Upload signed Desktop assets") <
      workflow.indexOf("Publish latest.json after every referenced asset exists"),
  );
  assert.ok(
    workflow.indexOf("Publish latest.json after every referenced asset exists") <
      workflow.indexOf("Publish complete GitHub Release"),
  );
  assert.match(
    releaseWorkflow,
    /uses: \.\/\.github\/workflows\/desktop-release\.yml/,
  );
  assert.doesNotMatch(releaseWorkflow, /goreleaser/i);
  assert.doesNotMatch(releaseWorkflow, /docs-publish/);
});
