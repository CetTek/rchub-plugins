#!/usr/bin/env node
// Builds, publishes and catalogs every plugin.
//
//   node tools/build-catalog.mjs --dist dist [--publish]
//
// For each plugins/<name>/ the zip is built by its package.py. The release tag
// `<id>-v<version>` is immutable: if it already exists its zip is downloaded and
// cataloged (the catalog must carry the hash of the shipped bytes); otherwise the
// fresh zip is uploaded as a new release (only with --publish). The signed
// catalog is written to <dist>/plugins.json and, with --publish, uploaded to the
// rolling `catalog` release. The private key comes from PLUGIN_SIGNING_KEY (PEM
// text) or PLUGIN_SIGNING_KEY_FILE and is never printed.
import { execFileSync, spawnSync } from 'node:child_process';
import { createHash, createPrivateKey } from 'node:crypto';
import { existsSync, mkdirSync, readdirSync, readFileSync, renameSync, statSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { signCatalog } from './catalog-core.mjs';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const repo = process.env.GITHUB_REPOSITORY || 'CetTek/rchub-plugins';
const PLATFORM = 'windows-x86_64';
const MIN_APP_VERSION = '0.1.0';

const args = process.argv.slice(2);
const publish = args.includes('--publish');
const distIndex = args.indexOf('--dist');
const dist = resolve(root, distIndex >= 0 ? args[distIndex + 1] : 'dist');
mkdirSync(dist, { recursive: true });

const run = (cmd, argv, options = {}) => execFileSync(cmd, argv, { stdio: ['ignore', 'pipe', 'inherit'], encoding: 'utf8', ...options });
const sha256 = path => createHash('sha256').update(readFileSync(path)).digest('hex');
const python = process.platform === 'win32' ? 'python' : 'python3';

function zipManifest(path) {
  const code = 'import sys,zipfile;sys.stdout.write(zipfile.ZipFile(sys.argv[1]).read("manifest.json").decode("utf-8"))';
  return JSON.parse(run(python, ['-c', code, path]));
}

function releaseExists(tag) {
  return spawnSync('gh', ['release', 'view', tag, '-R', repo], { stdio: 'ignore' }).status === 0;
}

const entries = [];
for (const name of readdirSync(join(root, 'plugins')).sort()) {
  const dir = join(root, 'plugins', name);
  if (!statSync(dir).isDirectory() || !existsSync(join(dir, 'package.py'))) continue;
  console.log(`== ${name}: build`);
  run(python, ['package.py'], { cwd: dir, stdio: 'inherit' });
  const built = readdirSync(dir).filter(file => file.endsWith(`-${PLATFORM}.zip`));
  if (built.length !== 1) throw new Error(`${name}: expected one zip, found ${built.length}`);
  const manifest = zipManifest(join(dir, built[0]));
  if (!/^rchub\.[a-z0-9-]+$/.test(manifest.id)) throw new Error(`${name}: unexpected id ${manifest.id}`);
  const asset = `${manifest.id}-${manifest.version}-${PLATFORM}.zip`;
  const tag = `${manifest.id}-v${manifest.version}`;
  const file = join(dist, asset);
  if (releaseExists(tag)) {
    console.log(`   ${tag} exists: using the released zip`);
    run('gh', ['release', 'download', tag, '-R', repo, '--pattern', asset, '--dir', dist, '--clobber']);
  } else {
    renameSync(join(dir, built[0]), file);
    if (publish) {
      console.log(`   publishing ${tag}`);
      run('gh', ['release', 'create', tag, file, '-R', repo, '--title', `${manifest.name} ${manifest.version}`,
        '--notes', `${manifest.name} ${manifest.version} for ${PLATFORM}.`, '--latest=false']);
    }
  }
  const released = zipManifest(file);
  if (released.id !== manifest.id || released.version !== manifest.version) throw new Error(`${name}: released zip does not match source`);
  entries.push({
    id: released.id,
    name: released.name,
    version: released.version,
    protocol: released.protocol,
    platform: PLATFORM,
    url: `https://github.com/${repo}/releases/download/${tag}/${asset}`,
    size: statSync(file).size,
    sha256: sha256(file),
    minAppVersion: MIN_APP_VERSION,
  });
}

const keyPem = process.env.PLUGIN_SIGNING_KEY || (process.env.PLUGIN_SIGNING_KEY_FILE ? readFileSync(process.env.PLUGIN_SIGNING_KEY_FILE, 'utf8') : '');
if (!keyPem) throw new Error('no signing key: set PLUGIN_SIGNING_KEY or PLUGIN_SIGNING_KEY_FILE');
const privateKey = createPrivateKey(keyPem);
if (privateKey.asymmetricKeyType !== 'ed25519') throw new Error('signing key must be Ed25519');

const catalog = signCatalog({ schemaVersion: 1, generatedAt: new Date().toISOString(), plugins: entries }, privateKey);
const out = join(dist, 'plugins.json');
writeFileSync(out, `${JSON.stringify(catalog, null, 2)}\n`);
console.log(`catalog: ${entries.length} plugins -> ${out}`);

if (publish) {
  if (!releaseExists('catalog')) {
    run('gh', ['release', 'create', 'catalog', '-R', repo, '--title', 'Plugin catalog',
      '--notes', 'Signed plugin catalog, replaced on every publish. Not a plugin release.', '--prerelease', '--latest=false']);
  }
  run('gh', ['release', 'upload', 'catalog', out, '-R', repo, '--clobber']);
  console.log('catalog published');
}
