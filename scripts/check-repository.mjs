import { execFileSync } from 'node:child_process';

// Check names only. Credential contents are handled by Gitleaks and GitHub.
const history = process.argv.includes('--history');
const paths = history
  ? execFileSync('git', ['rev-list', '--objects', '--all'], { encoding: 'utf8' })
      .split('\n')
      .filter((line) => line.includes(' '))
      .map((line) => line.slice(line.indexOf(' ') + 1))
  : execFileSync('git', ['ls-files', '-z'], { encoding: 'utf8' }).split('\0');
const forbidden = [
  /(^|\/)(node_modules|\.svelte-kit|build|artifacts|test-results|playwright-report|\.codex|\.agents|\.aws|\.ssh|\.idea|\.vscode)(\/|$)/i,
  /^src-tauri\/(target|gen)(\/|$)/i,
  /(^|\/)(\.npmrc|credentials(?:\.toml)?|recovery\.json|preferences\.json|trace-metadata\.json|window-state\.json)$/i,
  /(^|\/)\.env(?:$|\.(?!example$).+)/i,
  /\.(pem|key|p12|pfx|p8|keystore|exe|msi|dmg|appimage|deb|zip|7z|log|bak|tmp)$/i,
  /\.(brickpress|legopress)(-presets)?\.json$/i
];
const rejected = [
  ...new Set(paths.filter((path) => path && forbidden.some((rule) => rule.test(path))))
];
if (rejected.length) {
  console.error(`Files that belong outside the public source repository:\n${rejected.join('\n')}`);
  process.exitCode = 1;
} else {
  console.log(`Repository hygiene passed (${history ? 'all Git revisions' : 'tracked files'}).`);
}
