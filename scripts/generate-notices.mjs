import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';

// Include shipped UI/font dependencies and their dependency notices. Build-only
// tools and peer dependencies are deliberately outside this distribution list.
const roots = [
  '@tauri-apps/api',
  '@fontsource/inter',
  '@fontsource/ibm-plex-mono',
  '@lucide/svelte',
  '@sveltejs/kit',
  'svelte',
  'bits-ui',
  'paneforge',
  'clsx',
  'tailwind-merge',
  'tailwind-variants',
  'tailwindcss',
  'tw-animate-css'
];
const project = resolve(import.meta.dirname, '..');
const packages = new Map();
// The static client ships these Svelte/SvelteKit helpers. Their parsers,
// compilers, Node server and development server are build tools, not shipped UI.
const clientDependencies = {
  svelte: ['clsx', 'devalue', 'esm-env'],
  '@sveltejs/kit': ['devalue', 'esm-env']
};

function packagePath(name, from) {
  let directory = from;
  while (true) {
    const candidate = join(directory, 'node_modules', name, 'package.json');
    if (existsSync(candidate)) return candidate;
    const parent = dirname(directory);
    if (parent === directory)
      throw new Error(`Cannot locate license source for ${name}. Run npm ci.`);
    directory = parent;
  }
}

function collect(name, from = project) {
  const path = packagePath(name, from);
  if (packages.has(path)) return;
  const pkg = JSON.parse(readFileSync(path, 'utf8'));
  const directory = dirname(path);
  const licenses = readdirSync(directory).filter((file) =>
    /^(licen[sc]e|copying|notice)(\.|$)/i.test(file)
  );
  if (!licenses.length) throw new Error(`Missing license text for ${pkg.name}@${pkg.version}.`);
  packages.set(path, {
    name: pkg.name,
    version: pkg.version,
    license: pkg.license,
    texts: licenses.sort().map((file) => readFileSync(join(directory, file), 'utf8').trim())
  });
  for (const dependency of clientDependencies[name] ?? Object.keys(pkg.dependencies ?? {}))
    collect(dependency, directory);
}

for (const root of roots) collect(root);
const sections = [...packages.values()]
  .sort((a, b) => `${a.name}@${a.version}`.localeCompare(`${b.name}@${b.version}`))
  .map(
    (pkg) =>
      `${pkg.name}@${pkg.version} (${pkg.license})\n${'-'.repeat(72)}\n${pkg.texts.join('\n\n')}`
  );
const uiLicense = readFileSync(join(project, 'licenses', 'shadcn-svelte.txt'), 'utf8').trim();
const projectLicense = readFileSync(join(project, 'LICENSE'), 'utf8').trim();
const output = [
  'Brickpress third-party notices',
  'Generated from the installed versions pinned in package-lock.json. Keep these notices with distributed builds.',
  'The following project license covers original Brickpress code and catalog assets only. Dependency licenses below remain unchanged. See NOTICE.md for provenance.',
  `Brickpress project license\n${'-'.repeat(72)}\n${projectLicense}`,
  `shadcn-svelte UI foundation\n${'-'.repeat(72)}\n${uiLicense}`,
  ...sections
].join('\n\n');
mkdirSync(join(project, 'static'), { recursive: true });
writeFileSync(join(project, 'static', 'THIRD_PARTY_NOTICES.txt'), `${output}\n`, 'utf8');
writeFileSync(join(project, 'static', 'LICENSE.txt'), `${projectLicense}\n`, 'utf8');
console.log(
  `Collected license notices for ${packages.size} dependency packages and shadcn-svelte.`
);
