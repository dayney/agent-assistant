import assert from 'node:assert/strict';
import test from 'node:test';

import { resolveTarget, selectTargetTriple } from './build-sidecar.mjs';

const cases = [
  {
    rustTarget: 'aarch64-apple-darwin',
    goos: 'darwin',
    goarch: 'arm64',
    filename: 'agent-assistant-core-aarch64-apple-darwin',
    developmentFilename: 'agent-assistant-core',
  },
  {
    rustTarget: 'x86_64-apple-darwin',
    goos: 'darwin',
    goarch: 'amd64',
    filename: 'agent-assistant-core-x86_64-apple-darwin',
    developmentFilename: 'agent-assistant-core',
  },
  {
    rustTarget: 'x86_64-pc-windows-msvc',
    goos: 'windows',
    goarch: 'amd64',
    filename: 'agent-assistant-core-x86_64-pc-windows-msvc.exe',
    developmentFilename: 'agent-assistant-core.exe',
  },
  {
    rustTarget: 'aarch64-pc-windows-msvc',
    goos: 'windows',
    goarch: 'arm64',
    filename: 'agent-assistant-core-aarch64-pc-windows-msvc.exe',
    developmentFilename: 'agent-assistant-core.exe',
  },
];

for (const expected of cases) {
  test(`resolves ${expected.rustTarget}`, () => {
    assert.deepEqual(resolveTarget(expected.rustTarget), expected);
  });
}

test('rejects an unaudited target', () => {
  assert.throws(
    () => resolveTarget('x86_64-unknown-linux-gnu'),
    /unsupported sidecar target x86_64-unknown-linux-gnu/,
  );
});

test('an explicit target wins over Tauri and host targets', () => {
  assert.equal(
    selectTargetTriple({
      argv: ['--target', 'x86_64-pc-windows-msvc'],
      tauriTarget: 'aarch64-pc-windows-msvc',
      hostTarget: 'aarch64-apple-darwin',
    }),
    'x86_64-pc-windows-msvc',
  );
});

test('Tauri target wins over the Rust host target', () => {
  assert.equal(
    selectTargetTriple({
      argv: [],
      tauriTarget: 'x86_64-apple-darwin',
      hostTarget: 'aarch64-apple-darwin',
    }),
    'x86_64-apple-darwin',
  );
});

test('requires a target value after --target', () => {
  assert.throws(
    () => selectTargetTriple({ argv: ['--target'], hostTarget: 'aarch64-apple-darwin' }),
    /--target requires a Rust target triple/,
  );
});
