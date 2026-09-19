import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDirectory = dirname(fileURLToPath(import.meta.url));

export const DEFAULT_UPDATER_PUBLIC_KEY_PATH = resolve(
  scriptDirectory,
  "..",
  "src-tauri",
  "updater.pubkey",
);

export function readUpdaterPublicKey(
  path = DEFAULT_UPDATER_PUBLIC_KEY_PATH,
) {
  try {
    return readFileSync(path, "utf8");
  } catch (error) {
    const detail = error instanceof Error ? error.message : String(error);
    throw new Error(`read updater public key ${path}: ${detail}`);
  }
}
