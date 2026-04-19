// blake2b -- BLAKE2b cryptographic hash function (RFC 7693) from scratch.
//
// BLAKE2b is a modern hash that is faster than MD5 on 64-bit hardware and as
// secure as SHA-3.  It supports variable output length (1..64 bytes), single-
// pass keyed mode (replacing HMAC), and salt/personalization parameters.
//
// This implementation uses JavaScript BigInt throughout.  A pure-``number``
// two-32-bit-word emulation would be faster but much more code; BigInt keeps
// the implementation a clean one-to-one with the RFC for readability.
//
// Required reading: ``code/specs/HF06-blake2b.md``.
//
// Key invariant (and the classic BLAKE2 off-by-one): the *last real block* is
// the one flagged ``final``.  For messages whose length is an exact multiple
// of 128 bytes, we do NOT add a padding-only final block -- the streaming
// hasher preserves a non-empty tail by flushing only when the buffer strictly
// exceeds the block size.

const MASK64 = 0xFFFFFFFFFFFFFFFFn;
const BLOCK_SIZE = 128;
const MAX_DIGEST = 64;
const MAX_KEY = 64;

// Initial Hash Values -- identical to SHA-512's IVs (fractional parts of the
// square roots of the first eight primes, truncated to 64 bits).  BLAKE2b
// reuses these "nothing up my sleeve" constants.
const IV: readonly bigint[] = [
  0x6A09E667F3BCC908n,
  0xBB67AE8584CAA73Bn,
  0x3C6EF372FE94F82Bn,
  0xA54FF53A5F1D36F1n,
  0x510E527FADE682D1n,
  0x9B05688C2B3E6C1Fn,
  0x1F83D9ABFB41BD6Bn,
  0x5BE0CD19137E2179n,
];

// Ten message-schedule permutations.  Round i of the compression function
// uses SIGMA[i % 10]; rounds 10 and 11 reuse SIGMA[0] and SIGMA[1].
const SIGMA: readonly (readonly number[])[] = [
  [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
  [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
  [11, 8, 12, 0, 5, 2, 15, 13, 10, 14, 3, 6, 7, 1, 9, 4],
  [7, 9, 3, 1, 13, 12, 11, 14, 2, 6, 5, 10, 4, 0, 15, 8],
  [9, 0, 5, 7, 2, 4, 10, 15, 14, 1, 11, 12, 6, 8, 3, 13],
  [2, 12, 6, 10, 0, 11, 8, 3, 4, 13, 7, 5, 15, 14, 1, 9],
  [12, 5, 1, 15, 14, 13, 4, 10, 0, 7, 6, 3, 9, 2, 8, 11],
  [13, 11, 7, 14, 12, 1, 3, 9, 5, 0, 15, 4, 8, 6, 2, 10],
  [6, 15, 14, 9, 11, 3, 0, 8, 12, 2, 13, 7, 1, 4, 10, 5],
  [10, 2, 8, 4, 7, 6, 1, 5, 15, 11, 9, 14, 3, 12, 13, 0],
];

// Rotate ``x`` right by ``n`` bits within a 64-bit word, emulated via BigInt.
function rotr64(x: bigint, n: bigint): bigint {
  return ((x >> n) | (x << (64n - n))) & MASK64;
}

// BLAKE2b quarter-round G.  Mutates four entries of the 16-word working
// vector ``v`` using two message words ``x`` and ``y``.  Rotation constants
// (R1, R2, R3, R4) = (32, 24, 16, 63).
function mix(
  v: bigint[],
  a: number,
  b: number,
  c: number,
  d: number,
  x: bigint,
  y: bigint,
): void {
  v[a] = (v[a] + v[b] + x) & MASK64;
  v[d] = rotr64(v[d] ^ v[a], 32n);
  v[c] = (v[c] + v[d]) & MASK64;
  v[b] = rotr64(v[b] ^ v[c], 24n);
  v[a] = (v[a] + v[b] + y) & MASK64;
  v[d] = rotr64(v[d] ^ v[a], 16n);
  v[c] = (v[c] + v[d]) & MASK64;
  v[b] = rotr64(v[b] ^ v[c], 63n);
}

// Parse a 128-byte block as sixteen little-endian 64-bit words.
function parseBlock(block: Uint8Array): bigint[] {
  const view = new DataView(block.buffer, block.byteOffset, block.byteLength);
  const out: bigint[] = new Array(16);
  for (let i = 0; i < 16; i++) {
    out[i] = view.getBigUint64(i * 8, /*littleEndian=*/ true);
  }
  return out;
}

// Compression function F: mix one 128-byte block into the 8-word state.
//
// ``t`` is the total byte count fed into the hash so far, including the bytes
// of the current block.  ``final`` must be true iff this is the last
// compression call; that triggers the v[14] inversion used to domain-separate
// the final block from any intermediate block.
function compress(
  h: bigint[],
  block: Uint8Array,
  t: bigint,
  final: boolean,
): void {
  const m = parseBlock(block);
  const v: bigint[] = [...h, ...IV];

  // Fold the 128-bit byte counter into v[12..13].  Messages > 2^64 bytes are
  // not supported; the upper 64 bits of the counter stay zero in practice.
  v[12] ^= t & MASK64;
  v[13] ^= (t >> 64n) & MASK64;

  if (final) {
    v[14] ^= MASK64;
  }

  // Twelve rounds.  Each round applies G to four columns, then to four
  // diagonals -- the ChaCha20 "double round" pattern.
  for (let i = 0; i < 12; i++) {
    const s = SIGMA[i % 10];
    mix(v, 0, 4, 8, 12, m[s[0]], m[s[1]]);
    mix(v, 1, 5, 9, 13, m[s[2]], m[s[3]]);
    mix(v, 2, 6, 10, 14, m[s[4]], m[s[5]]);
    mix(v, 3, 7, 11, 15, m[s[6]], m[s[7]]);
    mix(v, 0, 5, 10, 15, m[s[8]], m[s[9]]);
    mix(v, 1, 6, 11, 12, m[s[10]], m[s[11]]);
    mix(v, 2, 7, 8, 13, m[s[12]], m[s[13]]);
    mix(v, 3, 4, 9, 14, m[s[14]], m[s[15]]);
  }

  // Feed-forward: XOR both halves of v into the state.
  for (let i = 0; i < 8; i++) {
    h[i] ^= v[i] ^ v[i + 8];
  }
}

// Build the parameter-block-XOR-ed starting state.  Sequential mode only.
function initialState(
  digestSize: number,
  keyLen: number,
  salt: Uint8Array,
  personal: Uint8Array,
): bigint[] {
  const p = new Uint8Array(64);
  p[0] = digestSize;
  p[1] = keyLen;
  p[2] = 1; // fanout = 1
  p[3] = 1; // depth  = 1
  if (salt.length > 0) p.set(salt, 32);
  if (personal.length > 0) p.set(personal, 48);

  const view = new DataView(p.buffer);
  const state: bigint[] = new Array(8);
  for (let i = 0; i < 8; i++) {
    state[i] = IV[i] ^ view.getBigUint64(i * 8, /*littleEndian=*/ true);
  }
  return state;
}

function validate(
  digestSize: number,
  key: Uint8Array,
  salt: Uint8Array,
  personal: Uint8Array,
): void {
  if (!Number.isInteger(digestSize) || digestSize < 1 || digestSize > MAX_DIGEST) {
    throw new RangeError(`digest_size must be in [1, 64], got ${digestSize}`);
  }
  if (key.length > MAX_KEY) {
    throw new RangeError(`key length must be in [0, 64], got ${key.length}`);
  }
  if (salt.length !== 0 && salt.length !== 16) {
    throw new RangeError(
      `salt must be exactly 16 bytes (or empty), got ${salt.length}`,
    );
  }
  if (personal.length !== 0 && personal.length !== 16) {
    throw new RangeError(
      `personal must be exactly 16 bytes (or empty), got ${personal.length}`,
    );
  }
}

export interface Blake2bOptions {
  digestSize?: number;
  key?: Uint8Array;
  salt?: Uint8Array;
  personal?: Uint8Array;
}

// Streaming BLAKE2b hasher.  ``digest()`` is non-destructive; repeated calls
// return the same bytes and the hasher stays usable for further ``update()``.
export class Blake2bHasher {
  private state: bigint[];
  private buffer: Uint8Array;
  private byteCount: bigint;
  private readonly digestSize: number;

  constructor(options: Blake2bOptions = {}) {
    const digestSize = options.digestSize ?? 64;
    const key = options.key ?? new Uint8Array(0);
    const salt = options.salt ?? new Uint8Array(0);
    const personal = options.personal ?? new Uint8Array(0);

    validate(digestSize, key, salt, personal);

    this.digestSize = digestSize;
    this.state = initialState(digestSize, key.length, salt, personal);
    this.byteCount = 0n;

    if (key.length > 0) {
      // Keyed mode: prepend the key, zero-padded to one full block.
      const keyBlock = new Uint8Array(BLOCK_SIZE);
      keyBlock.set(key);
      this.buffer = keyBlock;
    } else {
      this.buffer = new Uint8Array(0);
    }
  }

  update(data: Uint8Array): Blake2bHasher {
    // Append data to the buffer.  Flush every full block except the latest --
    // we must keep at least one byte in the buffer until digest() time so
    // that the final compression is the one flagged final.
    const merged = new Uint8Array(this.buffer.length + data.length);
    merged.set(this.buffer);
    merged.set(data, this.buffer.length);
    this.buffer = merged;

    while (this.buffer.length > BLOCK_SIZE) {
      this.byteCount += BigInt(BLOCK_SIZE);
      compress(this.state, this.buffer.subarray(0, BLOCK_SIZE), this.byteCount, false);
      this.buffer = this.buffer.subarray(BLOCK_SIZE);
    }
    return this;
  }

  digest(): Uint8Array {
    // Copy state so digest() is non-destructive.
    const state = [...this.state];
    const finalBlock = new Uint8Array(BLOCK_SIZE);
    finalBlock.set(this.buffer);
    const byteCount = this.byteCount + BigInt(this.buffer.length);
    compress(state, finalBlock, byteCount, true);

    const out = new Uint8Array(64);
    const view = new DataView(out.buffer);
    for (let i = 0; i < 8; i++) {
      view.setBigUint64(i * 8, state[i], /*littleEndian=*/ true);
    }
    return out.subarray(0, this.digestSize);
  }

  hexDigest(): string {
    return bytesToHex(this.digest());
  }

  copy(): Blake2bHasher {
    const clone = Object.create(Blake2bHasher.prototype) as Blake2bHasher;
    // Casting to any is unavoidable when reaching into private fields of
    // a sibling instance; all assignments remain type-checked.
    (clone as unknown as {
      state: bigint[];
      buffer: Uint8Array;
      byteCount: bigint;
      digestSize: number;
    }).state = [...this.state];
    (clone as unknown as { buffer: Uint8Array }).buffer = new Uint8Array(this.buffer);
    (clone as unknown as { byteCount: bigint }).byteCount = this.byteCount;
    (clone as unknown as { digestSize: number }).digestSize = this.digestSize;
    return clone;
  }
}

function bytesToHex(bytes: Uint8Array): string {
  let hex = "";
  for (const byte of bytes) {
    hex += byte.toString(16).padStart(2, "0");
  }
  return hex;
}

// One-shot BLAKE2b.  Returns raw bytes of length ``digestSize`` (default 64).
export function blake2b(data: Uint8Array, options: Blake2bOptions = {}): Uint8Array {
  const h = new Blake2bHasher(options);
  h.update(data);
  return h.digest();
}

// One-shot BLAKE2b returning lowercase hex.
export function blake2bHex(data: Uint8Array, options: Blake2bOptions = {}): string {
  return bytesToHex(blake2b(data, options));
}
