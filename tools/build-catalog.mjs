#!/usr/bin/env node
// Builds, publishes and catalogs every plugin.
//
//   node tools/build-catalog.mjs --dist dist [--publish]
//
// For each plugins/<name>/ the id and version are read from its package.py. The
// release tag `<id>-v<version>` is immutable: if it already exists its zip is
// downloaded and cataloged without building (the catalog must carry the hash of
// the shipped bytes); otherwise package.py builds it and the fresh zip is uploaded
// as a new release (only with --publish). The signed
// catalog is written to <dist>/plugins.json and, with --publish, uploaded to the
// rolling `catalog` release. The private key comes from PLUGIN_SIGNING_KEY (PEM
// text) or PLUGIN_SIGNING_KEY_FILE and is never printed.
import { execFileSync, spawnSync } from 'node:child_process';
import { createHash, createPrivateKey } from 'node:crypto';
import { existsSync, mkdirSync, readdirSync, readFileSync, renameSync, statSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { minAppVersionFor, signCatalog } from './catalog-core.mjs';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const repo = process.env.GITHUB_REPOSITORY || 'CetTek/rchub-plugins';
const PLATFORM = 'windows-x86_64';

const args = process.argv.slice(2);
const publish = args.includes('--publish');
const distIndex = args.indexOf('--dist');
const dist = resolve(root, distIndex >= 0 ? args[distIndex + 1] : 'dist');
mkdirSync(dist, { recursive: true });

const run = (cmd, argv, options = {}) => execFileSync(cmd, argv, {
  stdio: ['ignore', 'pipe', 'inherit'],
  encoding: 'utf8',
  env: { ...process.env, PYTHONUTF8: '1', PYTHONIOENCODING: 'utf-8' },
  ...options,
});
const sha256 = path => createHash('sha256').update(readFileSync(path)).digest('hex');
const python = process.platform === 'win32' ? 'python' : 'python3';

function zipManifest(path) {
  const code = 'import sys,zipfile;sys.stdout.write(zipfile.ZipFile(sys.argv[1]).read("manifest.json").decode("utf-8"))';
  return JSON.parse(run(python, ['-c', code, path]));
}

function releaseExists(tag) {
  return spawnSync('gh', ['release', 'view', tag, '-R', repo], { stdio: 'ignore' }).status === 0;
}

// The manifest dict in package.py holds exactly one "id" and one "version" literal.
function singleMatch(text, regex, label) {
  const found = [...text.matchAll(regex)];
  if (found.length !== 1) throw new Error(`${label}: expected exactly one match, found ${found.length}`);
  return found[0][1];
}

const entries = [];
for (const name of readdirSync(join(root, 'plugins')).sort()) {
  const dir = join(root, 'plugins', name);
  if (!statSync(dir).isDirectory() || !existsSync(join(dir, 'package.py'))) continue;
  const source = readFileSync(join(dir, 'package.py'), 'utf8');
  const sourceId = singleMatch(source, /"id":\s*"([^"]+)"/g, `${name}: id in package.py`);
  const sourceVersion = singleMatch(source, /"version":\s*"([^"]+)"/g, `${name}: version in package.py`);
  if (!/^rchub\.[a-z0-9-]+$/.test(sourceId)) throw new Error(`${name}: unexpected id ${sourceId}`);
  const asset = `${sourceId}-${sourceVersion}-${PLATFORM}.zip`;
  const tag = `${sourceId}-v${sourceVersion}`;
  const file = join(dist, asset);
  if (releaseExists(tag)) {
    // Published versions are immutable: reuse the released zip, no build needed.
    console.log(`== ${name}: ${tag} exists, using the released zip (build skipped)`);
    run('gh', ['release', 'download', tag, '-R', repo, '--pattern', asset, '--dir', dist, '--clobber']);
  } else {
    console.log(`== ${name}: build ${tag}`);
    run(python, ['package.py'], { cwd: dir, stdio: 'inherit' });
    const built = readdirSync(dir).filter(file => file.endsWith(`-${PLATFORM}.zip`));
    if (built.length !== 1) throw new Error(`${name}: expected one zip, found ${built.length}`);
    const manifest = zipManifest(join(dir, built[0]));
    if (manifest.id !== sourceId || manifest.version !== sourceVersion) throw new Error(`${name}: built zip ${manifest.id}@${manifest.version} does not match package.py ${sourceId}@${sourceVersion}`);
    renameSync(join(dir, built[0]), file);
    if (publish) {
      console.log(`   publishing ${tag}`);
      run('gh', ['release', 'create', tag, file, '-R', repo, '--title', `${manifest.name} ${manifest.version}`,
        '--notes', `${manifest.name} ${manifest.version} for ${PLATFORM}.`, '--latest=false']);
    }
  }
  const released = zipManifest(file);
  if (released.id !== sourceId || released.version !== sourceVersion) throw new Error(`${name}: released zip does not match source`);
  entries.push({
    id: released.id,
    name: released.name,
    version: released.version,
    protocol: released.protocol,
    platform: PLATFORM,
    url: `https://github.com/${repo}/releases/download/${tag}/${asset}`,
    size: statSync(file).size,
    sha256: sha256(file),
    minAppVersion: minAppVersionFor(released),
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
