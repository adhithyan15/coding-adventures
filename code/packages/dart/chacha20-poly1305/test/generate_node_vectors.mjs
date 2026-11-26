// Regenerate test/node_vectors.json deterministically.
//
// 90 seeded ChaCha20-Poly1305 AEAD vectors exchanged with Node.js
// built-in crypto (OpenSSL provider) — a fully independent oracle
// from a different implementation family than the Rust reference.
// (Node's raw `chacha20` takes a 16-byte IV with a different
// counter layout than RFC 8439's 12-byte-nonce API, so the stream
// rides the RFC §2.4.2 vector, fixed-vs-compat agreement, and the
// AEAD ciphertexts below, which verify the keystream transitively.
// XChaCha20 has no Node oracle and rides the draft vectors plus
// structural tests instead.)
//
// Usage: node test/generate_node_vectors.mjs   (from the package dir)
import { createCipheriv, createDecipheriv } from 'node:crypto';
import { writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const SEED = 20261130;

const PT_LENS = [0, 1, 7, 31, 63, 64, 65, 100, 127, 128, 200];
const AAD_LENS = [0, 5, 16, 20, 64];

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

// ChaCha20-Poly1305 AEAD.
for (let i = 0; i < 90; i++) {
  const key = nextBytes(32);
  const nonce = nextBytes(12);
  const aad = nextBytes(AAD_LENS[i % AAD_LENS.length]);
  const pt = nextBytes(PT_LENS[(i + 3) % PT_LENS.length]);
  const enc = createCipheriv('chacha20-poly1305', key, nonce, { authTagLength: 16 });
  if (aad.length > 0) enc.setAAD(aad);
  const ct = Buffer.concat([enc.update(pt), enc.final()]);
  const tag = enc.getAuthTag();
  const dec = createDecipheriv('chacha20-poly1305', key, nonce, { authTagLength: 16 });
  if (aad.length > 0) dec.setAAD(aad);
  dec.setAuthTag(tag);
  const back = Buffer.concat([dec.update(ct), dec.final()]);
  if (!back.equals(pt)) throw new Error(`AEAD round-trip failed: ${i}`);
  vectors.push({
    kind: 'aead',
    key: key.toString('hex'),
    nonce: nonce.toString('hex'),
    aad: aad.toString('hex'),
    pt: pt.toString('hex'),
    ct: ct.toString('hex'),
    tag: tag.toString('hex'),
  });
}

const distinct = new Set(vectors.map((v) => v.key + v.ct + v.tag));
if (distinct.size < vectors.length) {
  throw new Error(
    `degenerate vectors: only ${distinct.size} distinct triples (RNG broken?)`
  );
}

const out = join(dirname(fileURLToPath(import.meta.url)), 'node_vectors.json');
writeFileSync(out, JSON.stringify({ seed: SEED, vectors }, null, 1) + '\n');
console.log(`wrote ${vectors.length} vectors to ${out}`);
