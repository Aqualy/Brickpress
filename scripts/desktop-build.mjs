import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';

const e2e = process.argv.includes('--e2e');
const args = process.argv.slice(2).filter((arg) => arg !== '--e2e');
const options = e2e
  ? [
      '--debug',
      '--no-bundle',
      '--features',
      'desktop-e2e',
      '--config',
      'src-tauri/tauri.e2e.conf.json'
    ]
  : [];
// Cargo's --locked belongs after the Tauri argument separator. Keep user bundle
// and target options before it, and spawn directly without a command shell.
const result = spawnSync(
  process.execPath,
  [
    resolve('node_modules/@tauri-apps/cli/tauri.js'),
    'build',
    ...options,
    ...args,
    '--',
    '--locked'
  ],
  {
    stdio: 'inherit',
    env: process.env
  }
);
process.exit(result.status ?? 1);
