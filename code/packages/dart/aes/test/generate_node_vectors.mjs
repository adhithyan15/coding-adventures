// Regenerate test/node_vectors.json deterministically.
//
// 90 seeded vectors (30 per key size) encrypted with Node.js
// built-in AES-ECB (OpenSSL provider, no padding — exactly one
// block) — a fully independent oracle from a different
// implementation family than the Rust reference.
//
// Usage: node test/generate_node_vectors.mjs   (from the package dir)
import { createCipheriv, createDecipheriv } from 'node:crypto';
import { writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const SEED = 20261130;
const PER_SIZE = 30;

const SIZES = [
  { bits: 128, keyLen: 16 },
  { bits: 192, keyLen: 24 },
  { bits: 256, keyLen: 32 },
];

// SplitMix64 for deterministic bytes without dependencies.
let state = BigInt(SEED) & 0xffffffffffffffffn;
function nextByte() {
  state = (state + 0x9e3779b97f4a7c15n) & 0xffffffffffffffffn;
  let z = state;
  z = ((z ^ (z >> 30n)) * 0xbf58476d1ce4e5b9n) & 0xffffffffffffffffn;
  z = ((z ^ (z >> 27n)) * 0x94d049bb133111ebn) & 0xffffffffffffffffn;
  return Number((z ^ (z >> 31n)) & 0xffn);
}

function nextBytes(n) {
  // NOTE: Buffer.from({ length }, fn) IGNORES the map function —
  // fill explicitly (see the x25519 lane's generator note).
  const b = Buffer.alloc(n);
  for (let i = 0; i < n; i++) {
    b[i] = nextByte();
  }
  return b;
}

const vectors = [];
for (const { bits, keyLen } of SIZES) {
  const algo = `aes-${bits}-ecb`;
  for (let i = 0; i < PER_SIZE; i++) {
    // First vector per size pins the all-zero corner.
    const key = i === 0 ? Buffer.alloc(keyLen) : nextBytes(keyLen);
    const plain = i === 0 ? Buffer.alloc(16) : nextBytes(16);
    const enc = createCipheriv(algo, key, null);
    enc.setAutoPadding(false);
    const ct = Buffer.concat([enc.update(plain), enc.final()]);
    const dec = createDecipheriv(algo, key, null);
    dec.setAutoPadding(false);
    const back = Buffer.concat([dec.update(ct), dec.final()]);
    if (!back.equals(plain)) {
      throw new Error(`oracle round-trip failed: ${algo} vector ${i}`);
    }
    vectors.push({
      bits,
      key: key.toString('hex'),
      plain: plain.toString('hex'),
      ct: ct.toString('hex'),
    });
  }
}

const distinct = new Set(vectors.map((v) => v.ct));
if (distinct.size < vectors.length) {
  throw new Error(
    `degenerate vectors: only ${distinct.size} distinct ciphertexts (RNG broken?)`
  );
}

const out = join(dirname(fileURLToPath(import.meta.url)), 'node_vectors.json');
writeFileSync(out, JSON.stringify({ seed: SEED, vectors }, null, 1) + '\n');
console.log(`wrote ${vectors.length} vectors to ${out}`);
