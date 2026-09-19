import {
  existsSync,
  readFileSync,
  readdirSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { basename, join } from "node:path";
import { pathToFileURL } from "node:url";

const releaseBase = "https://github.com/spxrogers/agentsync/releases/download";

export function createUpdaterManifest({
  artifactRoot,
  version,
  tag,
  notes,
  pubDate,
}) {
  const platforms = {};
  for (const signaturePath of walkFiles(artifactRoot).filter((path) =>
    path.endsWith(".sig"),
  )) {
    const assetPath = signaturePath.slice(0, -".sig".length);
    if (!existsSync(assetPath)) continue;
    const assetName = basename(assetPath);
    const platform = platformForAsset(assetName);
    if (!platform) continue;
    if (platforms[platform]) {
      throw new Error(`multiple signed updater artifacts for ${platform}`);
    }
    platforms[platform] = {
      signature: readFileSync(signaturePath, "utf8").trim(),
      url: `${releaseBase}/${encodeURIComponent(tag)}/${encodeURIComponent(assetName)}`,
    };
  }

  for (const platform of ["darwin-aarch64", "windows-x86_64"]) {
    if (!platforms[platform]) {
      throw new Error(`missing signed updater artifact for ${platform}`);
    }
  }

  return {
    version: version.trim().replace(/^v/, ""),
    notes,
    pub_date: pubDate,
    platforms,
  };
}

function platformForAsset(assetName) {
  if (assetName.endsWith(".app.tar.gz")) return "darwin-aarch64";
  if (assetName.endsWith(".nsis.zip")) return "windows-x86_64";
  return null;
}

function walkFiles(directory) {
  const files = [];
  for (const entry of readdirSync(directory)) {
    const path = join(directory, entry);
    if (statSync(path).isDirectory()) files.push(...walkFiles(path));
    else files.push(path);
  }
  return files;
}

function main() {
  const outputPath = process.argv[2];
  if (!outputPath) throw new Error("output path is required");
  const notes = process.env.RELEASE_NOTES_FILE
    ? readFileSync(process.env.RELEASE_NOTES_FILE, "utf8")
    : "";
  const manifest = createUpdaterManifest({
    artifactRoot: process.env.UPDATER_ARTIFACT_ROOT ?? "",
    version: process.env.DESKTOP_VERSION ?? "",
    tag: process.env.RELEASE_TAG ?? "",
    notes,
    pubDate: process.env.RELEASE_PUB_DATE ?? new Date().toISOString(),
  });
  writeFileSync(outputPath, `${JSON.stringify(manifest, null, 2)}\n`);
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
