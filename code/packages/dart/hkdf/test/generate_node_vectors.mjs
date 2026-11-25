// Regenerate test/node_vectors.json deterministically.
//
// 60 seeded vectors (30 per digest) exchanged with Node.js built-in
// HKDF (OpenSSL provider) — a fully independent oracle from a
// different implementation family than the Rust reference. Each
// vector carries salt, IKM, info, output length, and the OKM.
//
// The BLAKE2b variant is library-specific (keyed mode, no HMAC
// construction) and has no standard oracle; it rides the structural
// unit tests instead.
//
// Usage: node test/generate_node_vectors.mjs   (from the package dir)
import { hkdfSync } from 'node:crypto';
import { writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const SEED = 20261130;
const PER_DIGEST = 30;

const DIGESTS = ['sha256', 'sha512'];
const SALT_LENS = [0, 8, 13, 32, 64, 131];
const IKM_LENS = [1, 22, 32, 80];
const INFO_LENS = [0, 5, 10, 80];
const LENGTHS = [1, 16, 31, 32, 33, 42, 64, 82, 100, 128];

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

function pick(arr, i) {
  return arr[i % arr.length];
}

const vectors = [];
for (const digest of DIGESTS) {
  const hashLen = digest === 'sha256' ? 32 : 64;
  for (let i = 0; i < PER_DIGEST; i++) {
    const salt = nextBytes(pick(SALT_LENS, i));
    const ikm = nextBytes(pick(IKM_LENS, i + 1));
    const info = nextBytes(pick(INFO_LENS, i + 2));
    // Last vector per digest exercises the full 255-block chain.
    const length = i === PER_DIGEST - 1 ? 255 * hashLen : pick(LENGTHS, i + 3);
    const okm = hkdfSync(digest, ikm, salt, info, length);
    vectors.push({
      digest,
      salt: salt.toString('hex'),
      ikm: ikm.toString('hex'),
      info: info.toString('hex'),
      length,
      okm: Buffer.from(okm).toString('hex'),
    });
  }
}

const distinct = new Set(vectors.map((v) => v.okm));
if (distinct.size < vectors.length) {
  throw new Error(
    `degenerate vectors: only ${distinct.size} distinct OKMs (RNG broken?)`
  );
}

const out = join(dirname(fileURLToPath(import.meta.url)), 'node_vectors.json');
writeFileSync(out, JSON.stringify({ seed: SEED, vectors }, null, 1) + '\n');
console.log(`wrote ${vectors.length} vectors to ${out}`);
