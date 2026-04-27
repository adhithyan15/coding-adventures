// argon2id -- hybrid memory-hard password hashing (RFC 9106).
//
// Argon2id is the recommended default Argon2 variant: it uses the
// side-channel-resistant *data-independent* addressing (Argon2i) for the
// first half of the first pass, and the GPU/ASIC-resistant
// *data-dependent* addressing (Argon2d) for everything after.
//
// This implementation uses JavaScript BigInt throughout so the compression
// function maps one-to-one with the RFC.  A pure-Number two-u32-words
// emulation would be faster but much more code; correctness-first is the
// right call for an educational port.
//
// Required reading: ``code/specs/KD03-argon2.md``.

import { blake2b } from "@coding-adventures/blake2b";

const MASK64 = 0xFFFFFFFFFFFFFFFFn;
const MASK32 = 0xFFFFFFFFn;

const BLOCK_SIZE = 1024;                 // bytes per Argon2 block
const BLOCK_WORDS = BLOCK_SIZE / 8;      // 128 × u64
const SYNC_POINTS = 4;                   // slices per pass
const ADDRESSES_PER_BLOCK = BLOCK_WORDS; // 128

const VERSION = 0x13;                    // only Argon2 v1.3 is supported

const TYPE_ID = 2;

// ---------------------------------------------------------------------------
// BLAKE2b round (Argon2 flavour) -- no SIGMA, with 2*trunc32(a)*trunc32(b).
// ---------------------------------------------------------------------------

function rotr64(x: bigint, n: bigint): bigint {
  return ((x >> n) | (x << (64n - n))) & MASK64;
}

// Argon2's G_B quarter-round.  Mutates four entries of ``v`` in place.
// The ``2 * trunc32(a) * trunc32(b)`` term is Argon2's only addition on
// top of BLAKE2b; it is what makes the mix non-linear in a way that
// isn't invertible from one operand.
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

// Permutation P -- one BLAKE2b round over 16 × u64.  The 8-call pattern
// is identical to BLAKE2b: four columns, then four diagonals.
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

// ---------------------------------------------------------------------------
// Compression function G(X, Y) -- blocks as 8x8 of 128-bit registers.
// ---------------------------------------------------------------------------
//
// P is applied to each of the 8 rows (16 contiguous words each), then to
// each of the 8 columns (word pairs gathered at stride 16 starting at
// offset 2c).  The feed-forward ``R XOR Q`` mirrors BLAKE2b's
// Davies-Meyer construction and is what keeps G non-invertible.

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

// ---------------------------------------------------------------------------
// Block serialisation.
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// H' -- variable-length BLAKE2b (RFC 9106 §3.3).
// ---------------------------------------------------------------------------
//
// For T <= 64 bytes, H' is just BLAKE2b-T(LE32(T) || X).
// For T > 64, emit T bytes by chaining 64-byte BLAKE2b outputs, keeping
// the first 32 bytes of each, then finishing with a variable-size
// BLAKE2b producing the tail (T - 32r) bytes, where r = ceil(T/32) - 2.

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

// ---------------------------------------------------------------------------
// index_alpha -- map J1 to a reference column (RFC 9106 §3.4.1.1).
// ---------------------------------------------------------------------------

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

  // J1^2 biasing, done in BigInt to avoid 53-bit Number overflow.
  const x = (J1 * J1) >> 32n;
  const Wb = BigInt(W);
  const y = Number((Wb * x) >> 32n);
  const rel = W - 1 - y;

  return (start + rel) % q;
}

// ---------------------------------------------------------------------------
// Segment fill -- Argon2id-specific: data-independent for r==0, sl<2.
// ---------------------------------------------------------------------------

function fillSegment(
  memory: bigint[][][],
  r: number,
  lane: number,
  sl: number,
  q: number,
  SL: number,
  p: number,
  mPrime: number,
  t: number,
): void {
  const dataIndependent = r === 0 && sl < 2;

  const inputBlock = new Array<bigint>(BLOCK_WORDS).fill(0n);
  const addressBlock = new Array<bigint>(BLOCK_WORDS).fill(0n);
  const zeroBlock = new Array<bigint>(BLOCK_WORDS).fill(0n);
  if (dataIndependent) {
    inputBlock[0] = BigInt(r);
    inputBlock[1] = BigInt(lane);
    inputBlock[2] = BigInt(sl);
    inputBlock[3] = BigInt(mPrime);
    inputBlock[4] = BigInt(t);
    inputBlock[5] = BigInt(TYPE_ID);
  }

  function nextAddresses(): void {
    inputBlock[6] = (inputBlock[6] + 1n) & MASK64;
    const Z = G(zeroBlock, inputBlock);
    const addr = G(zeroBlock, Z);
    for (let k = 0; k < BLOCK_WORDS; k++) addressBlock[k] = addr[k];
  }

  const startingC = r === 0 && sl === 0 ? 2 : 0;
  if (dataIndependent && startingC !== 0) nextAddresses();

  for (let i = startingC; i < SL; i++) {
    if (
      dataIndependent &&
      i % ADDRESSES_PER_BLOCK === 0 &&
      !(r === 0 && sl === 0 && i === 2)
    ) {
      nextAddresses();
    }

    const col = sl * SL + i;
    const prevCol = col > 0 ? col - 1 : q - 1;
    const prevBlock = memory[lane][prevCol];

    const pseudoRand = dataIndependent
      ? addressBlock[i % ADDRESSES_PER_BLOCK]
      : prevBlock[0];
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

// ---------------------------------------------------------------------------
// Parameter validation.
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Public API.
// ---------------------------------------------------------------------------

export interface Argon2Options {
  key?: Uint8Array;
  associatedData?: Uint8Array;
  version?: number;
}

// Compute an Argon2id tag (RFC 9106 §3).
export function argon2id(
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

  // Step 1 -- H0 (64 bytes, RFC 9106 §3.2).
  const h0 = blake2b(
    concatBytes(
      le32(p),
      le32(tagLength),
      le32(memoryCost),
      le32(t),
      le32(version),
      le32(TYPE_ID),
      le32(password.length), password,
      le32(salt.length), salt,
      le32(key.length), key,
      le32(associatedData.length), associatedData,
    ),
    { digestSize: 64 },
  );

  // Step 2 -- allocate B[p][q] (each slot 128 × u64).
  const memory: bigint[][][] = new Array(p);
  for (let i = 0; i < p; i++) {
    memory[i] = new Array<bigint[]>(q);
    for (let j = 0; j < q; j++) memory[i][j] = new Array<bigint>(BLOCK_WORDS).fill(0n);
  }

  // Step 3 -- first two blocks of every lane via H'.
  for (let i = 0; i < p; i++) {
    const b0 = blake2bLong(BLOCK_SIZE, concatBytes(h0, le32(0), le32(i)));
    const b1 = blake2bLong(BLOCK_SIZE, concatBytes(h0, le32(1), le32(i)));
    memory[i][0] = bytesToBlock(b0);
    memory[i][1] = bytesToBlock(b1);
  }

  // Step 4 -- fill the rest of memory, pass by pass, slice by slice.
  for (let r = 0; r < t; r++) {
    for (let sl = 0; sl < SYNC_POINTS; sl++) {
      for (let lane = 0; lane < p; lane++) {
        fillSegment(memory, r, lane, sl, q, SL, p, mPrime, t);
      }
    }
  }

  // Step 5 -- XOR the final column across lanes, then H' to T bytes.
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

// Argon2id returning lowercase hex.
export function argon2idHex(
  password: Uint8Array,
  salt: Uint8Array,
  timeCost: number,
  memoryCost: number,
  parallelism: number,
  tagLength: number,
  options: Argon2Options = {},
): string {
  return bytesToHex(
    argon2id(password, salt, timeCost, memoryCost, parallelism, tagLength, options),
  );
}
