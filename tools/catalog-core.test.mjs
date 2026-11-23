import test from 'node:test';
import assert from 'node:assert/strict';
import { createPublicKey, generateKeyPairSync, verify } from 'node:crypto';
import { canonicalize, keyIdFor, minAppVersionFor, signCatalog } from './catalog-core.mjs';

test('canonical JSON sorts keys and drops whitespace', () => {
  assert.equal(canonicalize({ b: [1, { z: 1, a: 'x' }], a: null }), '{"a":null,"b":[1,{"a":"x","z":1}]}');
});

test('signed catalog verifies over its canonical body and carries the key id', () => {
  const { privateKey } = generateKeyPairSync('ed25519');
  const body = { schemaVersion: 1, generatedAt: '2026-01-01T00:00:00.000Z', plugins: [] };
  const signed = signCatalog(body, privateKey);
  assert.equal(signed.keyId, keyIdFor(createPublicKey(privateKey)));
  assert.ok(verify(null, Buffer.from(canonicalize(body)), createPublicKey(privateKey), Buffer.from(signed.signature, 'base64')));
});

test('minAppVersion follows the newest manifest field the plugin uses', () => {
  assert.equal(minAppVersionFor({ id: 'rchub.plain' }), '0.1.0');
  assert.equal(minAppVersionFor({ id: 'rchub.icon', icon: 'ui/icon.svg' }), '0.1.1');
  assert.equal(minAppVersionFor({ id: 'rchub.desc', description: 'Read the clipboard.' }), '0.1.2');
  assert.equal(minAppVersionFor({ id: 'rchub.both', icon: 'ui/icon.svg', description: 'Read the clipboard.' }), '0.1.2');
});
