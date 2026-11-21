// Canonical JSON, key ids and Ed25519 signing for the plugin catalog.
// The byte layout matches the desktop's `release.rs` verifier: keys sorted by
// UTF-16 code unit, no whitespace, `keyId`/`signature` excluded from the body.
import { createHash, createPublicKey, sign } from 'node:crypto';

const SPKI_PREFIX = Buffer.from('302a300506032b6570032100', 'hex');

export function canonicalize(value) {
  if (value === null) return 'null';
  switch (typeof value) {
    case 'string':
    case 'boolean':
      return JSON.stringify(value);
    case 'number':
      if (!Number.isFinite(value)) throw new Error('non-finite number');
      return JSON.stringify(value);
    case 'object':
      if (Array.isArray(value)) return `[${value.map(canonicalize).join(',')}]`;
      return `{${Object.keys(value).sort().map(key => `${JSON.stringify(key)}:${canonicalize(value[key])}`).join(',')}}`;
    default:
      throw new Error(`unsupported JSON type ${typeof value}`);
  }
}

export function rawPublicKey(publicKey) {
  const spki = publicKey.export({ format: 'der', type: 'spki' });
  return spki.subarray(SPKI_PREFIX.length);
}

export function keyIdFor(publicKey) {
  const digest = createHash('sha256').update(publicKey.export({ format: 'der', type: 'spki' })).digest('hex');
  return `ed25519-${digest.slice(0, 16)}`;
}

/** Returns the catalog with `keyId` and `signature` added. */
export function signCatalog(body, privateKey) {
  const signature = sign(null, Buffer.from(canonicalize(body), 'utf8'), privateKey);
  return { ...body, keyId: keyIdFor(createPublicKey(privateKey)), signature: signature.toString('base64') };
}
