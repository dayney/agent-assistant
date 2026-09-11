import { copyFileSync, mkdirSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { spawnSync } from 'node:child_process';

const sidecarTargets = new Map([
  ['aarch64-apple-darwin', { goos: 'darwin', goarch: 'arm64', extension: '' }],
  ['x86_64-apple-darwin', { goos: 'darwin', goarch: 'amd64', extension: '' }],
  ['x86_64-pc-windows-msvc', { goos: 'windows', goarch: 'amd64', extension: '.exe' }],
  ['aarch64-pc-windows-msvc', { goos: 'windows', goarch: 'arm64', extension: '.exe' }],
]);

export function resolveTarget(rustTarget) {
  const target = sidecarTargets.get(rustTarget);
  if (!target) {
    throw new Error(`unsupported sidecar target ${rustTarget}`);
  }
  return {
    rustTarget,
    goos: target.goos,
    goarch: target.goarch,
    filename: `agent-assistant-core-${rustTarget}${target.extension}`,
    developmentFilename: `agent-assistant-core${target.extension}`,
  };
}

export function selectTargetTriple({ argv, tauriTarget, hostTarget }) {
  const targetIndex = argv.findIndex((argument) => argument === '--target');
  if (targetIndex >= 0) {
    const value = argv[targetIndex + 1];
    if (!value || value.startsWith('--')) {
      throw new Error('--target requires a Rust target triple');
    }
    return value;
  }
  const inlineTarget = argv.find((argument) => argument.startsWith('--target='));
  if (inlineTarget) {
    const value = inlineTarget.slice('--target='.length);
    if (!value) {
      throw new Error('--target requires a Rust target triple');
    }
    return value;
  }
  const selected = tauriTarget || hostTarget;
  if (!selected) {
    throw new Error('could not determine a Rust target triple');
  }
  return selected;
}

function rustHostTarget() {
  const result = run('rustc', ['-vV'], { encoding: 'utf8' });
  const match = /^host: (.+)$/m.exec(result.stdout);
  if (!match) {
    throw new Error('rustc -vV did not report a host target');
  }
  return match[1].trim();
}

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    stdio: options.encoding ? ['ignore', 'pipe', 'inherit'] : 'inherit',
    ...options,
    shell: false,
  });
  if (result.error) {
    throw new Error(`${command} failed to start: ${result.error.message}`);
  }
  if (result.status !== 0) {
    throw new Error(`${command} exited with status ${result.status}`);
  }
  return result;
}

function main() {
  const scriptDirectory = dirname(fileURLToPath(import.meta.url));
  const repositoryRoot = resolve(scriptDirectory, '..', '..');
  const tauriDirectory = join(repositoryRoot, 'desktop', 'src-tauri');
  const argv = process.argv.slice(2);
  const hasExplicitTarget = argv.some(
    (argument) => argument === '--target' || argument.startsWith('--target='),
  );
  const rustTarget = selectTargetTriple({
    argv,
    tauriTarget: process.env.TAURI_ENV_TARGET_TRIPLE,
    hostTarget: hasExplicitTarget || process.env.TAURI_ENV_TARGET_TRIPLE ? undefined : rustHostTarget(),
  });
  const target = resolveTarget(rustTarget);
  const bundleDirectory = join(tauriDirectory, 'binaries');
  const bundlePath = join(bundleDirectory, target.filename);
  const developmentPath = join(tauriDirectory, target.developmentFilename);

  mkdirSync(bundleDirectory, { recursive: true });
  run('go', ['build', '-trimpath', '-o', bundlePath, './cmd/agent-assistant-core'], {
    cwd: repositoryRoot,
    env: {
      ...process.env,
      CGO_ENABLED: '0',
      GOOS: target.goos,
      GOARCH: target.goarch,
    },
  });
  copyFileSync(bundlePath, developmentPath);
  process.stdout.write(`built ${target.filename}\n`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    main();
  } catch (error) {
    process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
    process.exitCode = 1;
  }
}
