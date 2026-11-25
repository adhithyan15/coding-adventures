// Regenerate test/node_vectors.json deterministically.
//
// 100 seeded vectors (random 32-byte scalar pairs) exchanged with
// Node.js built-in X25519 (OpenSSL provider) — a fully independent
// oracle from a different implementation family than the Rust
// reference. Each vector carries both keypairs and the shared
// secret computed in both directions (asserted equal here).
//
// Usage: node test/generate_node_vectors.mjs   (from the package dir)
import { createPrivateKey, createPublicKey, diffieHellman } from 'node:crypto';
import { writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const SEED = 20261129;
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

function next32() {
  // NOTE: Buffer.from({ length }, fn) IGNORES the map function (it is
  // only honored for real arrays) — fill explicitly instead. The
  // ed25519 lane's generator made this mistake and shipped 100
  // all-zero vectors; the distinct-seed assertion below guards it.
  const b = Buffer.alloc(32);
  for (let i = 0; i < 32; i++) {
    b[i] = nextByte();
  }
  return b;
}

// PKCS#8 DER prefix for X25519 private keys.
const PKCS8_PREFIX = Buffer.from('302e020100300506032b656e04220420', 'hex');
// SPKI DER prefix for X25519 public keys.
const SPKI_PREFIX = Buffer.from('302a300506032b656e032100', 'hex');

function privOf(raw) {
  return createPrivateKey({
    key: Buffer.concat([PKCS8_PREFIX, raw]),
    format: 'der',
    type: 'pkcs8',
  });
}

function pubOf(raw) {
  return createPublicKey({
    key: Buffer.concat([SPKI_PREFIX, raw]),
    format: 'der',
    type: 'spki',
  });
}

function baseOf(privRaw) {
  const pubDer = createPublicKey(privOf(privRaw)).export({
    format: 'der',
    type: 'spki',
  });
  return pubDer.subarray(pubDer.length - 32);
}

const vectors = [];
for (let i = 0; i < COUNT; i++) {
  const seedA = next32();
  const seedB = next32();
  const pubA = baseOf(seedA);
  const pubB = baseOf(seedB);
  const sharedAB = diffieHellman({ privateKey: privOf(seedA), publicKey: pubOf(pubB) });
  const sharedBA = diffieHellman({ privateKey: privOf(seedB), publicKey: pubOf(pubA) });
  if (!sharedAB.equals(sharedBA)) {
    throw new Error(`oracle disagreement on vector ${i}`);
  }
  if (sharedAB.every((b) => b === 0)) {
    throw new Error(`all-zero shared secret on vector ${i} (regenerate)`);
  }
  vectors.push({
    seedA: seedA.toString('hex'),
    seedB: seedB.toString('hex'),
    pubA: pubA.toString('hex'),
    pubB: pubB.toString('hex'),
    shared: sharedAB.toString('hex'),
  });
}

const distinctSeeds = new Set(vectors.flatMap((v) => [v.seedA, v.seedB]));
if (distinctSeeds.size < COUNT) {
  throw new Error(
    `degenerate vectors: only ${distinctSeeds.size} distinct seeds (RNG broken?)`
  );
}

const out = join(dirname(fileURLToPath(import.meta.url)), 'node_vectors.json');
writeFileSync(out, JSON.stringify({ seed: SEED, vectors }, null, 1) + '\n');
console.log(`wrote ${vectors.length} vectors to ${out}`);
