import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import { createUpdaterManifest } from "./create-updater-manifest.mjs";
import { evaluateReleasePolicy } from "./release-policy.mjs";
import { createUpdaterConfig } from "./write-updater-config.mjs";

const repositoryRoot = resolve(
  dirname(fileURLToPath(import.meta.url)),
  "..",
  "..",
);

function assertTextOrder(content, markers) {
  const positions = markers.map((marker) => {
    const position = content.indexOf(marker);
    assert.notEqual(position, -1, `missing workflow marker: ${marker}`);
    return position;
  });
  for (let index = 1; index < positions.length; index += 1) {
    assert.ok(
      positions[index - 1] < positions[index],
      `${markers[index - 1]} must precede ${markers[index]}`,
    );
  }
}

test("local and GitHub release tooling share Node 24.21.0", () => {
  const expectedVersion = "24.21.0";
  const localVersion = readFileSync(
    join(repositoryRoot, ".node-version"),
    "utf8",
  ).trim();
  const desktopPackage = JSON.parse(
    readFileSync(join(repositoryRoot, "desktop", "package.json"), "utf8"),
  );

  assert.equal(localVersion, expectedVersion);
  assert.equal(desktopPackage.engines?.node, expectedVersion);

  for (const workflowName of ["ci.yml", "desktop-release.yml"]) {
    const workflow = readFileSync(
      join(repositoryRoot, ".github", "workflows", workflowName),
      "utf8",
    );
    assert.match(workflow, /node-version-file: \.node-version/);
    assert.doesNotMatch(workflow, /node-version: /);
  }
});

test("resolves updater-only releases without commercial platform certificates", () => {
  assert.deepEqual(
    evaluateReleasePolicy({
      tag: "v1.2.3",
      signingMode: "updater-only",
      updaterPublicKey: "PUBLIC",
      updaterPrivateKey: "PRIVATE",
      updaterPrivateKeyPassword: "PASSWORD",
      appleCredentials: {},
      windowsCredentials: {},
    }),
    {
      version: "1.2.3",
      macosSigning: "adhoc",
      windowsSigning: "unsigned",
    },
  );
});

test("requires every updater signing credential", () => {
  const completeInput = {
    tag: "v1.2.3",
    signingMode: "updater-only",
    updaterPublicKey: "PUBLIC",
    updaterPrivateKey: "PRIVATE",
    updaterPrivateKeyPassword: "PASSWORD",
    appleCredentials: {},
    windowsCredentials: {},
  };

  for (const field of [
    "updaterPublicKey",
    "updaterPrivateKey",
    "updaterPrivateKeyPassword",
  ]) {
    assert.throws(
      () => evaluateReleasePolicy({ ...completeInput, [field]: "  " }),
      new RegExp(`Desktop release credentials are missing: ${field}`),
    );
  }
});

test("accepts only immutable stable release tags", () => {
  const input = {
    signingMode: "updater-only",
    updaterPublicKey: "PUBLIC",
    updaterPrivateKey: "PRIVATE",
    updaterPrivateKeyPassword: "PASSWORD",
    appleCredentials: {},
    windowsCredentials: {},
  };

  for (const tag of [
    "1.2.3",
    "v1.2",
    "v01.2.3",
    "v1.2.3-rc.1",
    "v1.2.3+build.1",
  ]) {
    assert.throws(
      () => evaluateReleasePolicy({ ...input, tag }),
      /stable vX\.Y\.Z tag/,
    );
  }
});

test("resolves fully platform-signed releases", () => {
  assert.deepEqual(
    evaluateReleasePolicy({
      tag: "v1.2.3",
      signingMode: "platform-signed",
      updaterPublicKey: "PUBLIC",
      updaterPrivateKey: "PRIVATE",
      updaterPrivateKeyPassword: "PASSWORD",
      appleCredentials: {
        APPLE_CERTIFICATE: "CERTIFICATE",
        APPLE_CERTIFICATE_PASSWORD: "CERTIFICATE-PASSWORD",
        APPLE_SIGNING_IDENTITY: "Developer ID Application: Example",
        APPLE_ID: "release@example.com",
        APPLE_PASSWORD: "APP-PASSWORD",
        APPLE_TEAM_ID: "TEAMID",
        KEYCHAIN_PASSWORD: "KEYCHAIN-PASSWORD",
      },
      windowsCredentials: {
        WINDOWS_CERTIFICATE: "CERTIFICATE",
        WINDOWS_CERTIFICATE_PASSWORD: "CERTIFICATE-PASSWORD",
        WINDOWS_TIMESTAMP_URL: "https://timestamp.example.com",
      },
    }),
    {
      version: "1.2.3",
      macosSigning: "developer-id",
      windowsSigning: "authenticode",
    },
  );
});

test("platform-signed releases never downgrade incomplete credentials", () => {
  const input = {
    tag: "v1.2.3",
    signingMode: "platform-signed",
    updaterPublicKey: "PUBLIC",
    updaterPrivateKey: "PRIVATE",
    updaterPrivateKeyPassword: "PASSWORD",
    appleCredentials: {
      APPLE_CERTIFICATE: "CERTIFICATE",
      APPLE_CERTIFICATE_PASSWORD: "CERTIFICATE-PASSWORD",
      APPLE_SIGNING_IDENTITY: "Developer ID Application: Example",
      APPLE_ID: "release@example.com",
      APPLE_PASSWORD: "APP-PASSWORD",
      APPLE_TEAM_ID: "TEAMID",
      KEYCHAIN_PASSWORD: "KEYCHAIN-PASSWORD",
    },
    windowsCredentials: {
      WINDOWS_CERTIFICATE: "CERTIFICATE",
      WINDOWS_CERTIFICATE_PASSWORD: "CERTIFICATE-PASSWORD",
      WINDOWS_TIMESTAMP_URL: "https://timestamp.example.com",
    },
  };

  assert.throws(
    () =>
      evaluateReleasePolicy({
        ...input,
        appleCredentials: { ...input.appleCredentials, APPLE_TEAM_ID: "" },
      }),
    /Apple signing credentials are missing: APPLE_TEAM_ID/,
  );
  assert.throws(
    () =>
      evaluateReleasePolicy({
        ...input,
        windowsCredentials: {
          ...input.windowsCredentials,
          WINDOWS_TIMESTAMP_URL: "  ",
        },
      }),
    /Windows signing credentials are missing: WINDOWS_TIMESTAMP_URL/,
  );
});

test("rejects unknown release signing modes", () => {
  assert.throws(
    () =>
      evaluateReleasePolicy({
        tag: "v1.2.3",
        signingMode: "automatic",
        updaterPublicKey: "PUBLIC",
        updaterPrivateKey: "PRIVATE",
        updaterPrivateKeyPassword: "PASSWORD",
        appleCredentials: {},
        windowsCredentials: {},
      }),
    /expected updater-only or platform-signed/,
  );
});

test("release preflight writes only non-sensitive policy outputs", () => {
  const root = mkdtempSync(join(tmpdir(), "agent-assistant-preflight-"));
  const publicKeyPath = join(root, "updater.pubkey");
  const outputPath = join(root, "github-output.txt");
  writeFileSync(publicKeyPath, "PUBLIC\n");

  const result = spawnSync(
    process.execPath,
    ["desktop/scripts/release-preflight.mjs"],
    {
      cwd: repositoryRoot,
      encoding: "utf8",
      env: {
        ...process.env,
        RELEASE_TAG: "v1.2.3",
        DESKTOP_SIGNING_MODE: "updater-only",
        TAURI_UPDATER_PUBLIC_KEY_PATH: publicKeyPath,
        TAURI_SIGNING_PRIVATE_KEY: "PRIVATE",
        TAURI_SIGNING_PRIVATE_KEY_PASSWORD: "PASSWORD",
        GITHUB_OUTPUT: outputPath,
      },
    },
  );

  assert.equal(result.status, 0, result.stderr);
  assert.equal(
    readFileSync(outputPath, "utf8"),
    "version=1.2.3\nmacos_signing=adhoc\nwindows_signing=unsigned\n",
  );
});

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
            "https://github.com/dayney/agent-assistant/releases/latest/download/latest.json",
          ],
        },
      },
    },
  );
});

test("uses the workflow repository for the updater endpoint", () => {
  const config = createUpdaterConfig({
    version: "1.2.3",
    publicKey: "PUBLIC-KEY",
    repository: "example/agent-assistant-fork",
  });

  assert.deepEqual(config.plugins.updater.endpoints, [
    "https://github.com/example/agent-assistant-fork/releases/latest/download/latest.json",
  ]);
});

test("refuses to build an updater config without a public key", () => {
  assert.throws(
    () => createUpdaterConfig({ version: "1.2.3", publicKey: "  " }),
    /public key is required/,
  );
});

test("release config CLI reads the tracked updater public key", () => {
  const root = mkdtempSync(join(tmpdir(), "agent-assistant-config-"));
  const publicKeyPath = join(root, "updater.pubkey");
  const outputPath = join(root, "release-config.json");
  writeFileSync(publicKeyPath, "PUBLIC-FROM-FILE\n");

  const result = spawnSync(
    process.execPath,
    ["desktop/scripts/write-updater-config.mjs", outputPath],
    {
      cwd: repositoryRoot,
      encoding: "utf8",
      env: {
        ...process.env,
        DESKTOP_VERSION: "1.2.3",
        GITHUB_REPOSITORY: "dayney/agent-assistant",
        TAURI_UPDATER_PUBLIC_KEY: "",
        TAURI_UPDATER_PUBLIC_KEY_PATH: publicKeyPath,
      },
    },
  );

  assert.equal(result.status, 0, result.stderr);
  assert.equal(
    JSON.parse(readFileSync(outputPath, "utf8")).plugins.updater.pubkey,
    "PUBLIC-FROM-FILE",
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
  writeFileSync(
    join(windowsDirectory, "agent-assistant_1.2.3_x64-setup.exe"),
    "windows",
  );
  writeFileSync(
    join(windowsDirectory, "agent-assistant_1.2.3_x64-setup.exe.sig"),
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
        url: "https://github.com/dayney/agent-assistant/releases/download/v1.2.3/agent-assistant.app.tar.gz",
      },
      "windows-x86_64": {
        signature: "windows-signature",
        url: "https://github.com/dayney/agent-assistant/releases/download/v1.2.3/agent-assistant_1.2.3_x64-setup.exe",
      },
    },
  });
});

test("uses the workflow repository for updater artifact URLs", () => {
  const root = mkdtempSync(join(tmpdir(), "agentsync-updater-"));
  writeFileSync(join(root, "agent-assistant.app.tar.gz"), "mac");
  writeFileSync(
    join(root, "agent-assistant.app.tar.gz.sig"),
    "mac-signature",
  );
  writeFileSync(join(root, "agent-assistant_1.2.3_x64-setup.exe"), "windows");
  writeFileSync(
    join(root, "agent-assistant_1.2.3_x64-setup.exe.sig"),
    "windows-signature",
  );

  const manifest = createUpdaterManifest({
    artifactRoot: root,
    version: "1.2.3",
    tag: "v1.2.3",
    notes: "",
    pubDate: "2026-09-16T00:00:00.000Z",
    repository: "example/agent-assistant-fork",
  });

  assert.equal(
    manifest.platforms["darwin-aarch64"].url,
    "https://github.com/example/agent-assistant-fork/releases/download/v1.2.3/agent-assistant.app.tar.gz",
  );
  assert.equal(
    manifest.platforms["windows-x86_64"].url,
    "https://github.com/example/agent-assistant-fork/releases/download/v1.2.3/agent-assistant_1.2.3_x64-setup.exe",
  );
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

test("refuses multiple signed updater artifacts for one platform", () => {
  const root = mkdtempSync(join(tmpdir(), "agentsync-updater-"));
  writeFileSync(join(root, "agent-assistant.app.tar.gz"), "mac");
  writeFileSync(
    join(root, "agent-assistant.app.tar.gz.sig"),
    "mac-signature",
  );
  for (const name of ["agent-assistant-setup.exe", "agent-assistant-alt-setup.exe"]) {
    writeFileSync(join(root, name), "windows");
    writeFileSync(join(root, `${name}.sig`), "windows-signature");
  }

  assert.throws(
    () =>
      createUpdaterManifest({
        artifactRoot: root,
        version: "1.2.3",
        tag: "v1.2.3",
        notes: "",
        pubDate: "2026-09-16T00:00:00.000Z",
      }),
    /multiple signed updater artifacts for windows-x86_64/,
  );
});

test("ignores signatures that are not Tauri updater artifacts", () => {
  const root = mkdtempSync(join(tmpdir(), "agentsync-updater-"));
  for (const [name, content] of [
    ["agent-assistant.app.tar.gz", "mac"],
    ["agent-assistant_1.2.3_x64-setup.exe", "windows"],
    ["agent-assistant.dmg", "installer"],
  ]) {
    writeFileSync(join(root, name), content);
    writeFileSync(join(root, `${name}.sig`), `${name}-signature`);
  }

  const manifest = createUpdaterManifest({
    artifactRoot: root,
    version: "1.2.3",
    tag: "v1.2.3",
    notes: "",
    pubDate: "2026-09-16T00:00:00.000Z",
  });

  assert.deepEqual(Object.keys(manifest.platforms).sort(), [
    "darwin-aarch64",
    "windows-x86_64",
  ]);
});

test("Desktop-only release publishes updater-signed packages after both platforms", () => {
  const workflow = readFileSync(
    join(repositoryRoot, ".github", "workflows", "desktop-release.yml"),
    "utf8",
  );
  const releaseWorkflow = readFileSync(
    join(repositoryRoot, ".github", "workflows", "release.yml"),
    "utf8",
  );

  assert.match(workflow, /needs: \[preflight, macos, windows\]/);
  assert.match(workflow, /signing_mode:/);
  assert.match(workflow, /release-preflight\.mjs/);
  assert.match(workflow, /TAURI_SIGNING_PRIVATE_KEY/);
  assert.match(workflow, /TAURI_SIGNING_PRIVATE_KEY_PASSWORD/);
  assert.doesNotMatch(workflow, /TAURI_UPDATER_PUBLIC_KEY/);
  assert.match(workflow, /macos_signing/);
  assert.match(workflow, /windows_signing/);
  assert.match(workflow, /APPLE_SIGNING_IDENTITY: "-"/);
  assert.match(workflow, /notarytool/);
  assert.match(workflow, /signtool/i);
  assert.match(workflow, /--bundles app,dmg/);
  assert.match(workflow, /AGENT_ASSISTANT_UPDATER_ENABLED: "true"/);
  assert.match(workflow, /VITE_UPDATER_ENABLED: "true"/);
  assert.doesNotMatch(workflow, /release-tag\.sh --(?:validate-only|package-version)/);
  assert.match(workflow, /create-updater-manifest\.mjs/);
  assert.match(workflow, /SHA256SUMS\.txt/);
  assert.match(workflow, /gh release create "\$TAG" --draft/);
  assertTextOrder(workflow, [
    "Upload Desktop release assets",
    "Publish latest.json",
    "Verify Draft release asset inventory",
    "Publish complete GitHub Release",
    "Verify public updater manifest",
  ]);
  assert.match(
    releaseWorkflow,
    /uses: \.\/\.github\/workflows\/desktop-release\.yml/,
  );
  assert.match(
    releaseWorkflow,
    /desktop-release:\n(?:.*\n)*?    if: github\.event_name == 'push'/,
  );
  assert.match(
    releaseWorkflow,
    /signing_mode: \$\{\{ vars\.DESKTOP_SIGNING_MODE \|\| 'updater-only' \}\}/,
  );
  assert.doesNotMatch(releaseWorkflow, /DESKTOP_RELEASE_ENABLED/);
  assert.doesNotMatch(releaseWorkflow, /Desktop distribution disabled/);
  assert.doesNotMatch(releaseWorkflow, /git push origin HEAD:main/);
  assert.doesNotMatch(releaseWorkflow, /update-changelog/);
  assert.doesNotMatch(releaseWorkflow, /goreleaser/i);
  assert.doesNotMatch(releaseWorkflow, /docs-publish/);
});
