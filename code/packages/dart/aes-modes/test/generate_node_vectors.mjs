// Regenerate test/node_vectors.json deterministically.
//
// 120 seeded vectors (30 ECB + 30 CBC + 30 CTR + 30 GCM) exchanged
// with Node.js built-in AES (OpenSSL provider) — a fully
// independent oracle from a different implementation family than
// the Rust reference. ECB/CBC use PKCS#7 padding (the default),
// matching the lane; CTR/GCM are unpadded with 12-byte nonces.
//
// Usage: node test/generate_node_vectors.mjs   (from the package dir)
import { createCipheriv, createDecipheriv } from 'node:crypto';
import { writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const SEED = 20261130;
const PER_MODE = 30;

const KEY_LENS = [16, 24, 32];
const PT_LENS = [0, 1, 7, 15, 16, 17, 31, 32, 33, 48, 64, 100];
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

const bitsOf = (keyLen) => keyLen * 8;
const vectors = [];

function ecbCase(i) {
  const key = nextBytes(KEY_LENS[i % KEY_LENS.length]);
  const pt = nextBytes(PT_LENS[(i + 1) % PT_LENS.length]);
  const enc = createCipheriv(`aes-${bitsOf(key.length)}-ecb`, key, null);
  const ct = Buffer.concat([enc.update(pt), enc.final()]);
  const dec = createDecipheriv(`aes-${bitsOf(key.length)}-ecb`, key, null);
  const back = Buffer.concat([dec.update(ct), dec.final()]);
  if (!back.equals(pt)) throw new Error(`ECB round-trip failed: ${i}`);
  return { mode: 'ecb', key, iv: null, aad: null, pt, ct, tag: null };
}

function cbcCase(i) {
  const key = nextBytes(KEY_LENS[(i + 1) % KEY_LENS.length]);
  const iv = nextBytes(16);
  const pt = nextBytes(PT_LENS[(i + 2) % PT_LENS.length]);
  const enc = createCipheriv(`aes-${bitsOf(key.length)}-cbc`, key, iv);
  const ct = Buffer.concat([enc.update(pt), enc.final()]);
  const dec = createDecipheriv(`aes-${bitsOf(key.length)}-cbc`, key, iv);
  const back = Buffer.concat([dec.update(ct), dec.final()]);
  if (!back.equals(pt)) throw new Error(`CBC round-trip failed: ${i}`);
  return { mode: 'cbc', key, iv, aad: null, pt, ct, tag: null };
}

function ctrCase(i) {
  const key = nextBytes(KEY_LENS[(i + 2) % KEY_LENS.length]);
  const nonce = nextBytes(12);
  const pt = nextBytes(PT_LENS[i % PT_LENS.length]);
  // Lane counter starts at 1: seed Node's full-16-byte IV as
  // nonce || 0x00000001 with autoPadding off.
  const iv = Buffer.concat([nonce, Buffer.from([0, 0, 0, 1])]);
  const enc = createCipheriv(`aes-${bitsOf(key.length)}-ctr`, key, iv);
  enc.setAutoPadding(false);
  const ct = Buffer.concat([enc.update(pt), enc.final()]);
  return { mode: 'ctr', key, iv: nonce, aad: null, pt, ct, tag: null };
}

function gcmCase(i) {
  const key = nextBytes(KEY_LENS[i % KEY_LENS.length]);
  const iv = nextBytes(12);
  const aad = nextBytes(AAD_LENS[(i + 1) % AAD_LENS.length]);
  const pt = nextBytes(PT_LENS[(i + 2) % PT_LENS.length]);
  const enc = createCipheriv(`aes-${bitsOf(key.length)}-gcm`, key, iv);
  if (aad.length > 0) enc.setAAD(aad);
  const ct = Buffer.concat([enc.update(pt), enc.final()]);
  const tag = enc.getAuthTag();
  const dec = createDecipheriv(`aes-${bitsOf(key.length)}-gcm`, key, iv);
  if (aad.length > 0) dec.setAAD(aad);
  dec.setAuthTag(tag);
  const back = Buffer.concat([dec.update(ct), dec.final()]);
  if (!back.equals(pt)) throw new Error(`GCM round-trip failed: ${i}`);
  return { mode: 'gcm', key, iv, aad, pt, ct, tag };
}

const cases = [ecbCase, cbcCase, ctrCase, gcmCase];
for (const fn of cases) {
  for (let i = 0; i < PER_MODE; i++) {
    const v = fn(i);
    vectors.push({
      mode: v.mode,
      key: v.key.toString('hex'),
      iv: v.iv === null ? null : v.iv.toString('hex'),
      aad: v.aad === null ? null : v.aad.toString('hex'),
      pt: v.pt.toString('hex'),
      ct: v.ct.toString('hex'),
      tag: v.tag === null ? null : v.tag.toString('hex'),
    });
  }
}

// Triples, not bare ciphertexts: empty-plaintext CTR/GCM vectors
// legitimately share the empty ciphertext.
const distinct = new Set(vectors.map((v) => v.key + v.ct + v.tag));
if (distinct.size < vectors.length) {
  throw new Error(
    `degenerate vectors: only ${distinct.size} distinct triples (RNG broken?)`
  );
}

const out = join(dirname(fileURLToPath(import.meta.url)), 'node_vectors.json');
writeFileSync(out, JSON.stringify({ seed: SEED, vectors }, null, 1) + '\n');
console.log(`wrote ${vectors.length} vectors to ${out}`);
