import { writeFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

import { githubReleasesBase } from "./github-release-url.mjs";

export function createUpdaterConfig({ version, publicKey, repository }) {
  const normalizedVersion = version.trim().replace(/^v/, "");
  if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/.test(normalizedVersion)) {
    throw new Error(`invalid Desktop version ${version}`);
  }
  const normalizedPublicKey = publicKey.trim();
  if (!normalizedPublicKey) throw new Error("updater public key is required");
  const updateEndpoint = `${githubReleasesBase(repository)}/latest/download/latest.json`;

  return {
    version: normalizedVersion,
    bundle: { createUpdaterArtifacts: true },
    plugins: {
      updater: {
        pubkey: normalizedPublicKey,
        endpoints: [updateEndpoint],
      },
    },
  };
}

function main() {
  const outputPath = process.argv[2];
  if (!outputPath) throw new Error("output path is required");
  const config = createUpdaterConfig({
    version: process.env.DESKTOP_VERSION ?? "",
    publicKey: process.env.TAURI_UPDATER_PUBLIC_KEY ?? "",
    repository: process.env.GITHUB_REPOSITORY,
  });
  writeFileSync(outputPath, `${JSON.stringify(config, null, 2)}\n`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    main();
  } catch (error) {
    process.stderr.write(
      `${error instanceof Error ? error.message : String(error)}\n`,
    );
    process.exitCode = 1;
  }
}
