#!/usr/bin/env node
// Generates the Ed25519 plugin-catalog signing key pair. The private key is
// written to --out-dir and never printed.
import { chmodSync, existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { generateKeyPairSync } from 'node:crypto';
import { keyIdFor, rawPublicKey } from './catalog-core.mjs';

const args = process.argv.slice(2);
const dir = args.indexOf('--out-dir');
if (dir < 0 || !args[dir + 1]) { console.error('usage: node tools/keygen.mjs --out-dir DIR'); process.exit(1); }
const out = resolve(args[dir + 1]);
const files = ['plugin-private-key.pem', 'plugin-public-key.pem', 'plugin-public-key.b64'].map(name => resolve(out, name));
if (files.some(existsSync)) { console.error('key files already exist; refusing to overwrite'); process.exit(1); }
const { privateKey, publicKey } = generateKeyPairSync('ed25519');
mkdirSync(out, { recursive: true });
writeFileSync(files[0], privateKey.export({ format: 'pem', type: 'pkcs8' }), { mode: 0o600 });
chmodSync(files[0], 0o600);
writeFileSync(files[1], publicKey.export({ format: 'pem', type: 'spki' }));
writeFileSync(files[2], `${rawPublicKey(publicKey).toString('base64')}\n`);
console.log(`key id: ${keyIdFor(publicKey)}`);
console.log(`written to ${out}`);
