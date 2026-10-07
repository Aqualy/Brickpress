import { spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
const e2e = process.argv.includes('--e2e');
const env = { ...process.env, VITE_DESKTOP_E2E: e2e ? '1' : '0' };
for (const script of ['notices:rust', 'build']) {
  const result = spawnSync(process.platform === 'win32' ? 'npm.cmd' : 'npm', ['run', script], {
    stdio: 'inherit',
    env,
    shell: process.platform === 'win32'
  });
  if (result.status !== 0) process.exit(result.status ?? 1);
}
if (!existsSync('static/RUST_DEPENDENCY_NOTICES.txt')) throw new Error('Rust notices missing');
