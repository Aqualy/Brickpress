import { createHash } from 'node:crypto';
import { createReadStream } from 'node:fs';
import { readdir, writeFile } from 'node:fs/promises';
import { extname, join, relative, resolve, sep } from 'node:path';

const root = resolve(process.argv[2] ?? 'src-tauri/target/release/bundle');
const extensions = new Set(['.exe', '.dmg', '.appimage', '.deb']);

async function installers(directory) {
  const paths = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) paths.push(...(await installers(path)));
    else if (entry.isFile() && extensions.has(extname(entry.name).toLowerCase())) paths.push(path);
  }
  return paths;
}

const paths = (await installers(root)).sort();
if (!paths.length) throw new Error(`No installers found in ${root}`);
const lines = [];
for (const path of paths) {
  const name = relative(root, path).split(sep).join('/');
  if (/[\r\n\\]/.test(name)) throw new Error(`Unsupported installer filename: ${name}`);
  const hash = createHash('sha256');
  for await (const chunk of createReadStream(path)) hash.update(chunk);
  lines.push(`${hash.digest('hex')}  ${name}`);
}
await writeFile(join(root, 'SHA256SUMS.txt'), `${lines.join('\n')}\n`, 'utf8');
console.log(`Wrote SHA256SUMS.txt for ${paths.length} installer(s).`);
