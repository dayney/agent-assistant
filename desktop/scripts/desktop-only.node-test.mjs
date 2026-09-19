import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const repositoryRoot = resolve(
  dirname(fileURLToPath(import.meta.url)),
  "..",
  "..",
);

test("repository exposes only the Desktop product", () => {
  const removedPaths = [
    "website",
    "cmd",
    "internal",
    "go.mod",
    "go.sum",
    ".golangci.yml",
    "scripts/test-in-container.sh",
    "test/container",
    "desktop/scripts/build-sidecar.mjs",
    "desktop/src-tauri/binaries",
    ".goreleaser.yaml",
    ".github/workflows/docs-publish.yml",
    "test/e2e",
    "test/bdd",
  ];

  for (const path of removedPaths) {
    assert.equal(
      existsSync(join(repositoryRoot, path)),
      false,
      `${path} must not exist in the Desktop-only repository`,
    );
  }
});

test("active automation has no CLI or documentation-site delivery path", () => {
  const activeFiles = [
    ".github/workflows/ci.yml",
    ".github/workflows/release.yml",
    "justfile",
  ];
  const removedDeliveryReferences = [
    /\bwebsite\//,
    /docs-publish/,
    /goreleaser/i,
    /setup-go/,
    /go\.mod/,
    /agent-assistant-core/,
    /build:core/,
  ];

  for (const path of activeFiles) {
    const content = readFileSync(join(repositoryRoot, path), "utf8");
    for (const reference of removedDeliveryReferences) {
      assert.doesNotMatch(content, reference, `${path} still references ${reference}`);
    }
  }
});
