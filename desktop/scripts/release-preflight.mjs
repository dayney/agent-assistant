import { appendFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

import {
  APPLE_CREDENTIAL_FIELDS,
  evaluateReleasePolicy,
  WINDOWS_CREDENTIAL_FIELDS,
} from "./release-policy.mjs";
import { readUpdaterPublicKey } from "./updater-public-key.mjs";

export function evaluateReleaseEnvironment(environment = process.env) {
  const publicKeyPath =
    environment.TAURI_UPDATER_PUBLIC_KEY_PATH;
  const updaterPublicKey = readUpdaterPublicKey(publicKeyPath);

  return evaluateReleasePolicy({
    tag: environment.RELEASE_TAG ?? "",
    signingMode: environment.DESKTOP_SIGNING_MODE ?? "updater-only",
    updaterPublicKey,
    updaterPrivateKey: environment.TAURI_SIGNING_PRIVATE_KEY ?? "",
    updaterPrivateKeyPassword:
      environment.TAURI_SIGNING_PRIVATE_KEY_PASSWORD ?? "",
    appleCredentials: selectEnvironment(
      environment,
      APPLE_CREDENTIAL_FIELDS,
    ),
    windowsCredentials: selectEnvironment(
      environment,
      WINDOWS_CREDENTIAL_FIELDS,
    ),
  });
}

function selectEnvironment(environment, fields) {
  return Object.fromEntries(fields.map((field) => [field, environment[field] ?? ""]));
}

function main() {
  const outputPath = process.env.GITHUB_OUTPUT;
  if (!outputPath) throw new Error("GITHUB_OUTPUT is required");

  const policy = evaluateReleaseEnvironment();
  appendFileSync(
    outputPath,
    [
      `version=${policy.version}`,
      `macos_signing=${policy.macosSigning}`,
      `windows_signing=${policy.windowsSigning}`,
      "",
    ].join("\n"),
  );
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    main();
  } catch (error) {
    process.stderr.write(
      `::error::${error instanceof Error ? error.message : String(error)}\n`,
    );
    process.exitCode = 1;
  }
}
