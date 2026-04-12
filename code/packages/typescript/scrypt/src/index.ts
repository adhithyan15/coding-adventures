/**
 * @coding-adventures/scrypt
 *
 * scrypt — Memory-Hard Password-Based Key Derivation Function (RFC 7914).
 *
 * What Is scrypt?
 * ===============
 * scrypt is a key derivation function designed by Colin Percival in 2009 and
 * standardized in RFC 7914 (2016). It extends PBKDF2 with a deliberately
 * memory-hard mixing step so that brute-force attacks using GPUs, FPGAs, and
 * ASICs are far more expensive.
 *
 * Why Memory-Hardness Matters
 * ============================
 * SHA-256 or SHA-512 alone are fast: a modern GPU can compute billions of hashes
 * per second. PBKDF2 adds iteration count — increasing CPU time — but each
 * evaluation still fits entirely in a processor's fast L1/L2 cache. An attacker
 * can run thousands of independent PBKDF2 evaluations in parallel because each
 * uses only a handful of kilobytes.
 *
 * scrypt forces each evaluation to:
 *   1. Allocate N × 128 × r bytes of memory (e.g., N=16384, r=8 → 16 MB).
 *   2. Fill it sequentially (so the attacker can't skip ahead).
 *   3. Re-read it in a pseudo-random order (so the attacker can't compress it).
 *
 * The random-access pattern means that if an attacker tries to store only a
 * fraction of the working set, they face cache misses and must recompute the
 * missing pages — trading memory for time at an extremely unfavourable ratio.
 *
 * Parameter Guidance
 * ==================
 * The three tuning parameters are:
 *
 *   N  — CPU/memory cost (power of 2). Higher N → more memory + time.
 *          Typical: N=16384 (2^14) for interactive logins,
 *                   N=1048576 (2^20) for sensitive storage.
 *
 *   r  — Block size multiplier. Each block = 128 × r bytes of working memory.
 *          Typical: r=8 (the NIST recommendation).
 *
 *   p  — Parallelisation factor. Allows splitting work across multiple cores.
 *          Typical: p=1.
 *
 * For a quick sanity check: memory used ≈ N × 128 × r bytes per scrypt call.
 *   N=16384, r=8 → 16,777,216 bytes → 16 MiB.
 *
 * Algorithm Overview (RFC 7914 §5)
 * ==================================
 *   1. Expand password+salt into p "blocks" of 128×r bytes via PBKDF2-HMAC-SHA256.
 *   2. For each block: apply ROMix (the memory-hard mixing step).
 *   3. Compress all p mixed blocks back into the final key via PBKDF2-HMAC-SHA256.
 *
 * ROMix in turn uses BlockMix, which applies Salsa20/8 — a reduced-round variant
 * of Daniel Bernstein's Salsa20 stream cipher — as a pseudorandom permutation.
 *
 * Why Salsa20/8 and Not AES?
 * ===========================
 * At the time scrypt was designed, AES hardware acceleration (AES-NI) was widely
 * available on CPUs but not yet on FPGAs/ASICs. Using Salsa20/8 meant that
 * purpose-built attackers had no hardware advantage over general-purpose CPUs.
 * Salsa20 also has a 32-bit-friendly design that maps well to any processor word
 * size.
 *
 * Important: scrypt Salsa20 uses LITTLE-ENDIAN word ordering throughout, unlike
 * most hash functions (which are big-endian). Every 32-bit word must be read and
 * written in little-endian byte order.
 *
 * RFC 7914 Test Vectors
 * =====================
 *   Vector 1 (trivial, quick sanity check):
 *     password = ""  salt = ""  N=16  r=1  p=1  dkLen=64
 *     expected = "77d6576238657b203b19ca42c18a0497f16b4844e3074ae8dfdffa3fede21442
 *                  fcd0069ded0948f8326a753a0fc81f17e8d3e0fb2e0d3628cf35e20c38d18906"
 *
 *   Vector 2 (more realistic):
 *     password = "password"  salt = "NaCl"  N=1024  r=8  p=16  dkLen=64
 *     expected = "fdbabe1c9d3472007856e7190d01e9fe7c6ad7cbc8237830e77376634b373162
 *                  2eaf30d92e22a3886ff109279d9830dac727afb94a83ee6d8360cbdfa2cc0640"
 *
 * Empty Password Handling
 * =======================
 * RFC 7914 vector 1 uses an empty password (""). The @coding-adventures/pbkdf2
 * package rejects empty passwords by default as a security guard against accidental
 * misuse. scrypt is explicitly designed to accept any password — including empty
 * strings — so it calls pbkdf2HmacSHA256 with allowEmptyPassword=true to bypass
 * that guard. The guard remains in place for all direct PBKDF2 usage.
 */

import { pbkdf2HmacSHA256 } from "@coding-adventures/pbkdf2";

export const VERSION = "0.1.0";

// ─── Rotate Left (32-bit) ─────────────────────────────────────────────────────
//
// Bitwise operations in JavaScript always return SIGNED 32-bit integers, even
// though JavaScript's Number type is a 64-bit IEEE 754 float. The bit pattern
// is correct but sign extension can produce large negative values when the
// high bit is set.
//
// The `>>> 0` (unsigned right shift by zero) idiom forces JavaScript to
// reinterpret the signed 32-bit result as an unsigned 32-bit integer.
// The value is then stored as a positive Number in the range [0, 2^32 − 1].
//
// Rotate left by n bits: move the top n bits around to the bottom.
//
//   Original:  AABBCCDD...  (32 bits)
//   Shift left n:  BBCCDD...000  (top n bits lost, bottom n become 0)
//   Shift right (32-n):  000...AABB  (bottom n become top n, rest zero)
//   OR together:  BBCCDD...AABB  ← correctly rotated
//
// Note: n=0 and n=32 are degenerate (identity). JavaScript shifts are mod 32,
// so n=32 would give shift by 0 — do NOT use n=32.
//
function rotl32(x: number, n: number): number {
  return ((x << n) | (x >>> (32 - n))) >>> 0;
}

// ─── Salsa20/8 Core ──────────────────────────────────────────────────────────
//
// Salsa20 is a stream cipher designed by Daniel Bernstein. It operates on a
// 64-byte (512-bit) block of state, treating it as 16 unsigned 32-bit words
// in LITTLE-ENDIAN byte order.
//
// The "quarter round" QR(a, b, c, d) is the fundamental operation:
//
//   b ^= rotl(a + d, 7)
//   c ^= rotl(b + a, 9)
//   d ^= rotl(c + b, 13)
//   a ^= rotl(d + c, 18)
//
// Notice: each step uses the UPDATED value from the previous line. This is a
// Feistel-like diffusion pattern where each word influences the next in a chain.
// The differing rotation amounts (7, 9, 13, 18) ensure rapid diffusion —
// changing a single input bit affects all 16 output words after 2 rounds.
//
// Salsa20/8 applies 4 "double rounds" (8 rounds total):
//   - One "column round": 4 QRs applied to columns of the 4×4 matrix
//   - One "row round": 4 QRs applied to rows of the 4×4 matrix
//
// After all 8 rounds, the final output is the element-wise sum of the original
// state and the mixed state, modulo 2^32. This "add back" step prevents the
// permutation from being invertible — you cannot recover the input from the
// output without knowing the original state.
//
// The 4×4 matrix layout for indexing (column-major when viewed as a grid):
//
//    0  4  8 12
//    1  5  9 13
//    2  6 10 14
//    3  7 11 15
//
// Column rounds operate on columns: (0,4,8,12), (5,9,13,1), (10,14,2,6), (15,3,7,11)
// Row rounds operate on rows:       (0,1,2,3),  (5,6,7,4),  (10,11,8,9), (15,12,13,14)
//
function salsa20_8(input: Uint8Array): Uint8Array {
  // Read the 64-byte input as 16 little-endian 32-bit words.
  // We use DataView for portable byte-order control — never assume native endianness.
  const view = new DataView(input.buffer, input.byteOffset, 64);
  const x = new Array<number>(16);
  for (let i = 0; i < 16; i++) {
    x[i] = view.getUint32(i * 4, true); // true = little-endian
  }
  // Save the initial state z = x (copy before mixing).
  // The final output will be z[i] + x[i] mod 2^32 for each i.
  const z = [...x];

  // Quarter round QR(a, b, c, d): modifies x[a], x[b], x[c], x[d] in place.
  // All additions are modulo 2^32 via `>>> 0`.
  function qr(a: number, b: number, c: number, d: number) {
    x[b] = (x[b] ^ rotl32((x[a] + x[d]) >>> 0, 7)) >>> 0;
    x[c] = (x[c] ^ rotl32((x[b] + x[a]) >>> 0, 9)) >>> 0;
    x[d] = (x[d] ^ rotl32((x[c] + x[b]) >>> 0, 13)) >>> 0;
    x[a] = (x[a] ^ rotl32((x[d] + x[c]) >>> 0, 18)) >>> 0;
  }

  // 4 double rounds (= 8 rounds total):
  for (let i = 0; i < 4; i++) {
    // Column rounds (mixing words in the same column of the 4×4 matrix):
    qr(0, 4, 8, 12);
    qr(5, 9, 13, 1);
    qr(10, 14, 2, 6);
    qr(15, 3, 7, 11);
    // Row rounds (mixing words in the same row of the 4×4 matrix):
    qr(0, 1, 2, 3);
    qr(5, 6, 7, 4);
    qr(10, 11, 8, 9);
    qr(15, 12, 13, 14);
  }

  // Write output: z[i] + x[i] mod 2^32, little-endian.
  const out = new Uint8Array(64);
  const outView = new DataView(out.buffer);
  for (let i = 0; i < 16; i++) {
    outView.setUint32(i * 4, (x[i] + z[i]) >>> 0, true); // true = little-endian
  }
  return out;
}

// ─── XOR Two 64-byte Blocks ──────────────────────────────────────────────────
//
// XOR each byte at the same position. This is the standard way to "mix"
// two blocks in stream-cipher and hash constructions — it is its own inverse
// (a ^ b ^ b = a), making the operation trivially reversible if you have
// both the key block and the output block.
//
function xor64(a: Uint8Array, b: Uint8Array): Uint8Array {
  const out = new Uint8Array(64);
  for (let i = 0; i < 64; i++) out[i] = a[i] ^ b[i];
  return out;
}

// ─── BlockMix ─────────────────────────────────────────────────────────────────
//
// BlockMix takes an array of 2r 64-byte blocks and produces 2r mixed blocks.
//
// Algorithm (RFC 7914 §3):
//   1. Start with x = the LAST block B[2r-1] (seeds the first mix operation).
//   2. For i = 0 to 2r-1:
//        x = Salsa20/8(x XOR B[i])   ← mix x with each block in turn
//        y[i] = x                     ← save the result
//   3. Interleave the outputs:
//        output = [y[0], y[2], ..., y[2r-2],   ← even-indexed blocks first
//                  y[1], y[3], ..., y[2r-1]]    ← odd-indexed blocks second
//
// The interleaving step increases diffusion: each output block depends on
// multiple input blocks through the chained x variable.
//
function blockMix(blocks: Uint8Array[], r: number): Uint8Array[] {
  const twoR = 2 * r;
  // Seed x from the last block (ensures the entire input influences from the start).
  let x = new Uint8Array(blocks[twoR - 1]);
  const y: Uint8Array[] = new Array(twoR);
  for (let i = 0; i < twoR; i++) {
    x = salsa20_8(xor64(x, blocks[i]));
    y[i] = new Uint8Array(x); // copy — x will be overwritten in the next iteration
  }
  // Interleave: even outputs first, then odd outputs.
  const out: Uint8Array[] = [];
  for (let i = 0; i < r; i++) out.push(y[2 * i]);
  for (let i = 0; i < r; i++) out.push(y[2 * i + 1]);
  return out;
}

// ─── Integerify ──────────────────────────────────────────────────────────────
//
// Integerify extracts a number from the last block of x, used as a pseudo-random
// index into the scratch-pad V during ROMix.
//
// RFC 7914 §4: Integerify(x) = the integer representation of B[2r-1] in
// little-endian order.
//
// The full value is a uint64 (8 bytes), but JavaScript doesn't have native 64-bit
// integers. Since N ≤ 2^20 in our implementation (enforced by validation), the low
// 32 bits are sufficient to compute the index j = Integerify(x) mod N without
// overflow. A 32-bit unsigned integer can represent values up to ~4 billion,
// which is far larger than 2^20 = 1,048,576.
//
function integerify(x: Uint8Array[]): number {
  const lastBlock = x[x.length - 1];
  const view = new DataView(lastBlock.buffer, lastBlock.byteOffset, 8);
  // Low 32 bits of the little-endian uint64 — sufficient for N ≤ 2^20.
  return view.getUint32(0, true);
}

// ─── ROMix ───────────────────────────────────────────────────────────────────
//
// ROMix is the memory-hard core of scrypt. It is the reason scrypt is so much
// harder to attack than plain PBKDF2.
//
// Phase 1 — Sequential Fill (force sequential memory writes):
//   Allocate a scratch-pad V of N blocks. Fill V[0..N-1] sequentially by
//   applying BlockMix N times, storing each intermediate state.
//
//   This forces the attacker to evaluate BlockMix N times in sequence.
//   Because each BlockMix output feeds the next, there is no parallelism.
//
// Phase 2 — Random Read (force random memory reads):
//   Apply BlockMix N more times. On each iteration:
//     j = Integerify(x) mod N       ← pseudo-random index derived from x
//     x = BlockMix(x XOR V[j])      ← XOR in V[j] before mixing
//
//   This forces the attacker to keep ALL of V in memory. If they evict even
//   a single V[j] entry, they must recompute it (which requires re-running
//   Phase 1 from the beginning). The random access pattern makes a 50%
//   memory reduction cost ~50% more time — not useful for attackers.
//
// Together, Phases 1 and 2 require both N sequential writes AND N random reads
// of N × 128 × r bytes. This is the "memory-hard" guarantee: you cannot trade
// memory for time at a favourable ratio.
//
function roMix(bBytes: Uint8Array, n: number, r: number): Uint8Array {
  const twoR = 2 * r;
  // Split the flat byte array into 2r separate 64-byte blocks.
  let x: Uint8Array[] = [];
  for (let i = 0; i < twoR; i++) {
    x.push(new Uint8Array(bBytes.slice(i * 64, (i + 1) * 64)));
  }

  // Phase 1: Fill scratch-pad V sequentially.
  const v: Uint8Array[][] = [];
  for (let i = 0; i < n; i++) {
    // Deep-copy current x into V[i] (x will be mutated in the next iteration).
    v.push(x.map((b) => new Uint8Array(b)));
    x = blockMix(x, r);
  }

  // Phase 2: Random reads from V, mixing into x.
  for (let i = 0; i < n; i++) {
    // Pseudo-random index derived from the current state.
    const j = integerify(x) % n;
    const vj = v[j];
    // XOR each block of x with the corresponding block from V[j].
    x = x.map((blk, idx) => xor64(blk, vj[idx]));
    x = blockMix(x, r);
  }

  // Flatten the 2r blocks back into a single byte array.
  const out = new Uint8Array(twoR * 64);
  for (let i = 0; i < twoR; i++) {
    out.set(x[i], i * 64);
  }
  return out;
}

// ─── Public API ──────────────────────────────────────────────────────────────

/**
 * scrypt — Password-Based Key Derivation Function 2 (RFC 7914).
 *
 * Derives a cryptographic key from a password and salt using the memory-hard
 * scrypt algorithm. Suitable for password hashing and key stretching where
 * resistance to hardware brute-force attacks is required.
 *
 * @param password - The password (may be empty — RFC 7914 vector 1 uses "").
 * @param salt     - A random or unique salt (empty salt is allowed per RFC).
 * @param n        - CPU/memory cost factor. Must be a power of 2 and ≥ 2.
 *                   Higher N → more memory and time. Typical: 16384 for logins.
 * @param r        - Block size parameter. Typical: 8.
 * @param p        - Parallelisation parameter. Typical: 1.
 * @param dkLen    - Desired output key length in bytes.
 * @returns        Derived key as a Uint8Array of length dkLen.
 *
 * @example
 * ```ts
 * const enc = new TextEncoder();
 * const key = scrypt(enc.encode("password"), enc.encode("NaCl"), 1024, 8, 16, 64);
 * ```
 */
export function scrypt(
  password: Uint8Array,
  salt: Uint8Array,
  n: number,
  r: number,
  p: number,
  dkLen: number,
): Uint8Array {
  // ── Validate parameters ──────────────────────────────────────────────────
  //
  // These checks are derived from RFC 7914 §2 and the mathematical constraints
  // of the algorithm. Violating them causes undefined behaviour or silently
  // wrong results.

  if (!Number.isInteger(n) || !Number.isInteger(r) || !Number.isInteger(p) || !Number.isInteger(dkLen)) {
    throw new Error("scrypt: N, r, p, and dkLen must be integers");
  }
  if (n < 2 || (n & (n - 1)) !== 0) {
    throw new Error("scrypt: N must be a power of 2 and >= 2");
  }
  if (n > 1 << 20) {
    // Limit N to 2^20 (≈ 1 million) so that integerify's low 32-bit truncation
    // remains safe and memory usage stays bounded to reasonable values.
    throw new Error("scrypt: N must not exceed 2^20");
  }
  if (r < 1) {
    throw new Error("scrypt: r must be a positive integer");
  }
  if (p < 1) {
    throw new Error("scrypt: p must be a positive integer");
  }
  if (dkLen < 1) {
    throw new Error("scrypt: dk_len must be a positive integer");
  }
  if (dkLen > 1 << 20) {
    // Sanity cap: 1 MiB of output is ample for any real-world use.
    throw new Error("scrypt: dk_len must not exceed 2^20");
  }
  if (p * r > 1 << 30) {
    // RFC 7914 requires p ≤ ((2^32 − 1) × hLen) / (128 × r).
    // The simpler p*r ≤ 2^30 guard covers all practical cases.
    throw new Error("scrypt: p * r exceeds limit");
  }
  // p * 128 * r is the actual byte count allocated in Step 1. Even when
  // p*r ≤ 2^30, p*128*r can reach 128 GiB (e.g. p=2^15, r=2^15). Cap at
  // 2^30 bytes (1 GiB) and ensure it is a safe integer.
  const bLen = p * 128 * r;
  if (!Number.isSafeInteger(bLen) || bLen > (1 << 30)) {
    throw new Error("scrypt: p * 128 * r exceeds memory limit (2^30 bytes = 1 GiB)");
  }

  // ── Step 1: Expand password + salt into p blocks via PBKDF2 ─────────────
  //
  // PBKDF2 with 1 iteration is used here as a pure pseudo-random function to
  // stretch the password into a large block buffer. The memory-hardness comes
  // from ROMix, not from PBKDF2 iteration count (which is fixed at 1 here).
  let b = pbkdf2HmacSHA256(password, salt, 1, bLen, true);

  // ── Step 2: Apply ROMix independently to each p block ───────────────────
  //
  // Each block of 128×r bytes undergoes the memory-hard ROMix transformation
  // independently. The parallelism factor p allows these to run on separate
  // threads in environments that support it — in this single-threaded JS
  // implementation they run sequentially.
  for (let i = 0; i < p; i++) {
    const chunk = b.slice(i * 128 * r, (i + 1) * 128 * r);
    const mixed = roMix(chunk, n, r);
    b.set(mixed, i * 128 * r);
  }

  // ── Step 3: Compress all p mixed blocks into the final key via PBKDF2 ───
  //
  // Another PBKDF2 call (again 1 iteration) maps the large mixed buffer back
  // down to the requested dkLen bytes. The password is used as the PBKDF2
  // password and b is used as the salt — ensuring both inputs influence the
  // final output.
  return pbkdf2HmacSHA256(password, b, 1, dkLen, true);
}

/**
 * scryptHex — scrypt output as a lowercase hexadecimal string.
 *
 * Convenience wrapper that encodes the raw scrypt output as hex.
 *
 * @param password - The password bytes.
 * @param salt     - The salt bytes.
 * @param n        - CPU/memory cost. Power of 2, ≥ 2.
 * @param r        - Block size parameter.
 * @param p        - Parallelisation parameter.
 * @param dkLen    - Desired key length in bytes.
 * @returns Lowercase hex string of length 2 × dkLen.
 *
 * @example
 * ```ts
 * const enc = new TextEncoder();
 * scryptHex(new Uint8Array(0), new Uint8Array(0), 16, 1, 1, 64);
 * // "77d6576238657b203b19ca42c18a0497..."
 * ```
 */
export function scryptHex(
  password: Uint8Array,
  salt: Uint8Array,
  n: number,
  r: number,
  p: number,
  dkLen: number,
): string {
  return Array.from(scrypt(password, salt, n, r, p, dkLen))
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}
