// Regenerate test/node_vectors.json deterministically.
//
// 100 seeded vectors (random 32-byte seeds x random 0-300-byte
// messages) signed with Node.js built-in Ed25519 (OpenSSL provider) —
// a fully independent oracle from a different implementation family
// than the Rust reference.
//
// Usage: node test/generate_node_vectors.mjs   (from the package dir)
import { createPrivateKey, createPublicKey, sign } from 'node:crypto';
import { writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const SEED = 20261125;
const COUNT = 100;

// SplitMix64 for deterministic bytes without dependencies.
let state = BigInt(SEED) & 0xffffffffffffffffn;
function nextByte() {
  state = (state + 0x9e3779b97f4a7c15n) & 0xffffffffffffffffn;
  let z = state;
  z = ((z ^ (z >> 30n)) * 0xbf58476d1ce4e5b9n) & 0xffffffffffffffffn;
  z = ((z ^ (z >> 27n)) * 0x94d049bb133111ebn) & 0xffffffffffffffffn;
  return Number((z ^ (z >> 31n)) & 0xffn);
}

// PKCS#8 DER prefix for Ed25519 private keys.
const PKCS8_PREFIX = Buffer.from('302e020100300506032b657004220420', 'hex');

const vectors = [];
for (let i = 0; i < COUNT; i++) {
  const seedLen = 32;
  const seed = Buffer.from({ length: seedLen }, nextByte);
  const msgLen = [0, 1, 2, 31, 32, 33, 63, 64, 65, 127, 128, 200, 300][i % 13];
  const msg = Buffer.from({ length: msgLen }, nextByte);

  const priv = createPrivateKey({
    key: Buffer.concat([PKCS8_PREFIX, seed]),
    format: 'der',
    type: 'pkcs8',
  });
  const pubDer = createPublicKey(priv).export({ format: 'der', type: 'spki' });
  const pub = pubDer.subarray(pubDer.length - 32);
  const sig = sign(null, msg, priv);

  vectors.push({
    seed: seed.toString('hex'),
    msg: msg.toString('hex'),
    pub: pub.toString('hex'),
    sig: sig.toString('hex'),
  });
}

const out = join(dirname(fileURLToPath(import.meta.url)), 'node_vectors.json');
writeFileSync(out, JSON.stringify({ seed: SEED, vectors }, null, 1) + '\n');
console.log(`wrote ${vectors.length} vectors to ${out}`);
