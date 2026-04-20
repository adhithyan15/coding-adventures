// argon2d -- data-dependent memory-hard password hashing (RFC 9106).
//
// Argon2d uses data-dependent addressing throughout every segment: the
// reference block for each new block is chosen from the first 64 bits of
// the previously computed block.  This maximises GPU/ASIC resistance at
// the cost of leaking a noisy channel through memory-access timing, so
// Argon2d is appropriate in contexts where side-channel attacks are not
// in the threat model (e.g. proof-of-work).  For password hashing prefer
// ``argon2id``.
//
// Required reading: ``code/specs/KD03-argon2.md``.

import { blake2b } from "@coding-adventures/blake2b";

const MASK64 = 0xFFFFFFFFFFFFFFFFn;
const MASK32 = 0xFFFFFFFFn;

const BLOCK_SIZE = 1024;
const BLOCK_WORDS = BLOCK_SIZE / 8;
const SYNC_POINTS = 4;

const VERSION = 0x13;
const TYPE_D = 0;

function rotr64(x: bigint, n: bigint): bigint {
  return ((x >> n) | (x << (64n - n))) & MASK64;
}

function GB(v: bigint[], a: number, b: number, c: number, d: number): void {
  let va = v[a], vb = v[b], vc = v[c], vd = v[d];

  va = (va + vb + 2n * (va & MASK32) * (vb & MASK32)) & MASK64;
  vd = rotr64(vd ^ va, 32n);
  vc = (vc + vd + 2n * (vc & MASK32) * (vd & MASK32)) & MASK64;
  vb = rotr64(vb ^ vc, 24n);
  va = (va + vb + 2n * (va & MASK32) * (vb & MASK32)) & MASK64;
  vd = rotr64(vd ^ va, 16n);
  vc = (vc + vd + 2n * (vc & MASK32) * (vd & MASK32)) & MASK64;
  vb = rotr64(vb ^ vc, 63n);

  v[a] = va; v[b] = vb; v[c] = vc; v[d] = vd;
}

function P(v: bigint[]): void {
  GB(v, 0, 4, 8, 12);
  GB(v, 1, 5, 9, 13);
  GB(v, 2, 6, 10, 14);
  GB(v, 3, 7, 11, 15);
  GB(v, 0, 5, 10, 15);
  GB(v, 1, 6, 11, 12);
  GB(v, 2, 7, 8, 13);
  GB(v, 3, 4, 9, 14);
}

function G(X: bigint[], Y: bigint[]): bigint[] {
  const R = new Array<bigint>(BLOCK_WORDS);
  for (let i = 0; i < BLOCK_WORDS; i++) R[i] = X[i] ^ Y[i];
  const Q = [...R];

  for (let i = 0; i < 8; i++) {
    const row = Q.slice(i * 16, (i + 1) * 16);
    P(row);
    for (let j = 0; j < 16; j++) Q[i * 16 + j] = row[j];
  }

  for (let c = 0; c < 8; c++) {
    const col = new Array<bigint>(16);
    for (let r = 0; r < 8; r++) {
      col[2 * r] = Q[r * 16 + 2 * c];
      col[2 * r + 1] = Q[r * 16 + 2 * c + 1];
    }
    P(col);
    for (let r = 0; r < 8; r++) {
      Q[r * 16 + 2 * c] = col[2 * r];
      Q[r * 16 + 2 * c + 1] = col[2 * r + 1];
    }
  }

  const out = new Array<bigint>(BLOCK_WORDS);
  for (let i = 0; i < BLOCK_WORDS; i++) out[i] = R[i] ^ Q[i];
  return out;
}

function blockToBytes(block: bigint[]): Uint8Array {
  const out = new Uint8Array(BLOCK_SIZE);
  const view = new DataView(out.buffer);
  for (let i = 0; i < BLOCK_WORDS; i++) {
    view.setBigUint64(i * 8, block[i], /*littleEndian=*/ true);
  }
  return out;
}

function bytesToBlock(data: Uint8Array): bigint[] {
  const view = new DataView(data.buffer, data.byteOffset, data.byteLength);
  const out = new Array<bigint>(BLOCK_WORDS);
  for (let i = 0; i < BLOCK_WORDS; i++) {
    out[i] = view.getBigUint64(i * 8, /*littleEndian=*/ true);
  }
  return out;
}

function le32(n: number): Uint8Array {
  const out = new Uint8Array(4);
  new DataView(out.buffer).setUint32(0, n >>> 0, /*littleEndian=*/ true);
  return out;
}

function concatBytes(...parts: Uint8Array[]): Uint8Array {
  let total = 0;
  for (const p of parts) total += p.length;
  const out = new Uint8Array(total);
  let off = 0;
  for (const p of parts) {
    out.set(p, off);
    off += p.length;
  }
  return out;
}

function blake2bLong(T: number, X: Uint8Array): Uint8Array {
  if (T <= 0) throw new RangeError(`H' output length must be positive, got ${T}`);
  const Tpref = le32(T);

  if (T <= 64) {
    return blake2b(concatBytes(Tpref, X), { digestSize: T });
  }

  const r = Math.ceil(T / 32) - 2;
  let V = blake2b(concatBytes(Tpref, X), { digestSize: 64 });
  const out = new Uint8Array(T);
  out.set(V.subarray(0, 32), 0);
  let off = 32;
  for (let i = 1; i < r; i++) {
    V = blake2b(V, { digestSize: 64 });
    out.set(V.subarray(0, 32), off);
    off += 32;
  }
  const finalSize = T - 32 * r;
  V = blake2b(V, { digestSize: finalSize });
  out.set(V, off);
  return out;
}

function indexAlpha(
  J1: bigint,
  r: number,
  sl: number,
  c: number,
  sameLane: boolean,
  q: number,
  SL: number,
): number {
  let W: number;
  let start: number;
  if (r === 0) {
    if (sl === 0) {
      W = c - 1;
      start = 0;
    } else {
      W = sameLane ? sl * SL + c - 1 : sl * SL - (c === 0 ? 1 : 0);
      start = 0;
    }
  } else {
    W = sameLane ? q - SL + c - 1 : q - SL - (c === 0 ? 1 : 0);
    start = ((sl + 1) * SL) % q;
  }

  const x = (J1 * J1) >> 32n;
  const Wb = BigInt(W);
  const y = Number((Wb * x) >> 32n);
  const rel = W - 1 - y;

  return (start + rel) % q;
}

// Argon2d fill -- data-dependent addressing in every segment.  J1 and
// J2 always come from the first u64 of the previous block.
function fillSegment(
  memory: bigint[][][],
  r: number,
  lane: number,
  sl: number,
  q: number,
  SL: number,
  p: number,
): void {
  const startingC = r === 0 && sl === 0 ? 2 : 0;

  for (let i = startingC; i < SL; i++) {
    const col = sl * SL + i;
    const prevCol = col > 0 ? col - 1 : q - 1;
    const prevBlock = memory[lane][prevCol];

    const pseudoRand = prevBlock[0];
    const J1 = pseudoRand & MASK32;
    const J2 = Number((pseudoRand >> 32n) & MASK32);

    const lPrime = r === 0 && sl === 0 ? lane : J2 % p;
    const zPrime = indexAlpha(J1, r, sl, i, lPrime === lane, q, SL);
    const refBlock = memory[lPrime][zPrime];

    const newBlock = G(prevBlock, refBlock);
    if (r === 0) {
      memory[lane][col] = newBlock;
    } else {
      const existing = memory[lane][col];
      const merged = new Array<bigint>(BLOCK_WORDS);
      for (let k = 0; k < BLOCK_WORDS; k++) merged[k] = existing[k] ^ newBlock[k];
      memory[lane][col] = merged;
    }
  }
}

function validate(
  password: Uint8Array,
  salt: Uint8Array,
  timeCost: number,
  memoryCost: number,
  parallelism: number,
  tagLength: number,
  key: Uint8Array,
  associatedData: Uint8Array,
  version: number,
): void {
  if (!(password instanceof Uint8Array)) throw new TypeError("password must be Uint8Array");
  if (!(salt instanceof Uint8Array)) throw new TypeError("salt must be Uint8Array");
  if (!(key instanceof Uint8Array)) throw new TypeError("key must be Uint8Array");
  if (!(associatedData instanceof Uint8Array)) throw new TypeError("associatedData must be Uint8Array");

  if (salt.length < 8) throw new RangeError(`salt must be at least 8 bytes, got ${salt.length}`);
  if (tagLength < 4) throw new RangeError(`tagLength must be >= 4, got ${tagLength}`);
  if (tagLength > 0xFFFFFFFF) throw new RangeError(`tagLength must fit in 32 bits, got ${tagLength}`);
  if (parallelism < 1 || parallelism > 0xFFFFFF) {
    throw new RangeError(`parallelism must be in [1, 2^24-1], got ${parallelism}`);
  }
  if (memoryCost < 8 * parallelism) {
    throw new RangeError(
      `memoryCost must be >= 8*parallelism (${8 * parallelism}), got ${memoryCost}`,
    );
  }
  if (memoryCost > 0xFFFFFFFF) throw new RangeError(`memoryCost must fit in 32 bits, got ${memoryCost}`);
  if (timeCost < 1) throw new RangeError(`timeCost must be >= 1, got ${timeCost}`);
  if (version !== VERSION) {
    throw new RangeError(`only Argon2 v1.3 (0x13) is supported; got 0x${version.toString(16)}`);
  }
}

export interface Argon2Options {
  key?: Uint8Array;
  associatedData?: Uint8Array;
  version?: number;
}

// Compute an Argon2d tag (RFC 9106 §3).
export function argon2d(
  password: Uint8Array,
  salt: Uint8Array,
  timeCost: number,
  memoryCost: number,
  parallelism: number,
  tagLength: number,
  options: Argon2Options = {},
): Uint8Array {
  const key = options.key ?? new Uint8Array(0);
  const associatedData = options.associatedData ?? new Uint8Array(0);
  const version = options.version ?? VERSION;

  validate(password, salt, timeCost, memoryCost, parallelism, tagLength, key, associatedData, version);

  const segmentLength = Math.floor(memoryCost / (SYNC_POINTS * parallelism));
  const mPrime = segmentLength * SYNC_POINTS * parallelism;
  const q = mPrime / parallelism;
  const SL = segmentLength;
  const p = parallelism;
  const t = timeCost;

  const h0 = blake2b(
    concatBytes(
      le32(p),
      le32(tagLength),
      le32(memoryCost),
      le32(t),
      le32(version),
      le32(TYPE_D),
      le32(password.length), password,
      le32(salt.length), salt,
      le32(key.length), key,
      le32(associatedData.length), associatedData,
    ),
    { digestSize: 64 },
  );

  const memory: bigint[][][] = new Array(p);
  for (let i = 0; i < p; i++) {
    memory[i] = new Array<bigint[]>(q);
    for (let j = 0; j < q; j++) memory[i][j] = new Array<bigint>(BLOCK_WORDS).fill(0n);
  }

  for (let i = 0; i < p; i++) {
    const b0 = blake2bLong(BLOCK_SIZE, concatBytes(h0, le32(0), le32(i)));
    const b1 = blake2bLong(BLOCK_SIZE, concatBytes(h0, le32(1), le32(i)));
    memory[i][0] = bytesToBlock(b0);
    memory[i][1] = bytesToBlock(b1);
  }

  for (let r = 0; r < t; r++) {
    for (let sl = 0; sl < SYNC_POINTS; sl++) {
      for (let lane = 0; lane < p; lane++) {
        fillSegment(memory, r, lane, sl, q, SL, p);
      }
    }
  }

  const finalBlock = [...memory[0][q - 1]];
  for (let lane = 1; lane < p; lane++) {
    for (let k = 0; k < BLOCK_WORDS; k++) finalBlock[k] ^= memory[lane][q - 1][k];
  }

  return blake2bLong(tagLength, blockToBytes(finalBlock));
}

function bytesToHex(bytes: Uint8Array): string {
  let hex = "";
  for (const byte of bytes) hex += byte.toString(16).padStart(2, "0");
  return hex;
}

// Argon2d returning lowercase hex.
export function argon2dHex(
  password: Uint8Array,
  salt: Uint8Array,
  timeCost: number,
  memoryCost: number,
  parallelism: number,
  tagLength: number,
  options: Argon2Options = {},
): string {
  return bytesToHex(
    argon2d(password, salt, timeCost, memoryCost, parallelism, tagLength, options),
  );
}
