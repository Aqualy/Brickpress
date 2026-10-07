import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync } from 'node:fs';
import { join } from 'node:path';
const cargo =
  process.env.CARGO ??
  (process.platform === 'win32' &&
  existsSync(join(process.env.USERPROFILE ?? '', '.cargo/bin/cargo.exe'))
    ? join(process.env.USERPROFILE, '.cargo/bin/cargo.exe')
    : 'cargo');
mkdirSync('static', { recursive: true });
const result = spawnSync(
  cargo,
  [
    'about',
    'generate',
    '--locked',
    '--manifest-path',
    'src-tauri/Cargo.toml',
    '-c',
    'src-tauri/about.toml',
    '-o',
    'static/RUST_DEPENDENCY_NOTICES.txt',
    'src-tauri/licenses.hbs'
  ],
  {
    stdio: 'inherit'
  }
);
if (result.status !== 0) {
  process.stderr.write(
    'Install cargo-about: cargo install cargo-about --version 0.9.2 --locked --features cli\n'
  );
  process.exit(result.status ?? 1);
}
console.log('Generated Rust dependency notices from Cargo.lock.');
