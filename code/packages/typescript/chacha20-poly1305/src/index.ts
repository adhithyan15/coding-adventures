// ============================================================================
// ChaCha20-Poly1305 — Authenticated Encryption (RFC 8439)
// ============================================================================
//
// This module implements the ChaCha20-Poly1305 AEAD cipher suite from scratch.
// It combines two primitives designed by Daniel J. Bernstein:
//
//   1. **ChaCha20** — a stream cipher built from ARX (Add, Rotate, XOR)
//      operations on 32-bit words. No lookup tables, no timing side channels.
//
//   2. **Poly1305** — a one-time MAC (Message Authentication Code) that
//      computes a 16-byte tag using arithmetic modulo the prime 2^130 - 5.
//
// Together they form an Authenticated Encryption with Associated Data (AEAD)
// scheme: ChaCha20 encrypts the plaintext while Poly1305 authenticates both
// the ciphertext and any additional data (like packet headers).
//
// Why ChaCha20 instead of AES?
// ----------------------------
// AES relies on hardware AES-NI instructions for speed and resistance to
// cache-timing attacks. ChaCha20 is fast in *pure software* on any CPU,
// making it ideal for mobile devices and software-only implementations.
// It's used in TLS 1.3, WireGuard, SSH, and Chrome/Android.
//
// ============================================================================

// ============================================================================
// Section 1: ChaCha20 Stream Cipher
// ============================================================================
//
// ChaCha20 generates a pseudorandom keystream by scrambling a 4x4 matrix
// of 32-bit words through 20 rounds of quarter-round operations. Each
// quarter round uses only three operations:
//
//   - Addition (mod 2^32)     — diffuses bits upward
//   - XOR                     — combines values without bias
//   - Rotation                — moves bits across word boundaries
//
// These are called ARX operations. They're simple, constant-time on all
// CPUs, and don't need lookup tables (unlike AES S-boxes).
//
// The state matrix layout:
//
//   ┌──────────┬──────────┬──────────┬──────────┐
//   │ const[0] │ const[1] │ const[2] │ const[3] │  "expand 32-byte k"
//   ├──────────┼──────────┼──────────┼──────────┤
//   │ key[0]   │ key[1]   │ key[2]   │ key[3]   │  256-bit key
//   ├──────────┼──────────┼──────────┼──────────┤
//   │ key[4]   │ key[5]   │ key[6]   │ key[7]   │  (continued)
//   ├──────────┼──────────┼──────────┼──────────┤
//   │ counter  │ nonce[0] │ nonce[1] │ nonce[2] │  32-bit counter + 96-bit nonce
//   └──────────┴──────────┴──────────┴──────────┘
//
// ============================================================================

/**
 * The four 32-bit constants that form the first row of the ChaCha20 state.
 * They spell out "expand 32-byte k" in ASCII — a nothing-up-my-sleeve number
 * chosen by Bernstein to fill the state deterministically.
 */
const CONSTANTS = [0x61707865, 0x3320646e, 0x79622d32, 0x6b206574] as const;

/**
 * Read a 32-bit little-endian word from a byte array.
 *
 * ChaCha20 works with 32-bit words stored in little-endian byte order.
 * For example, bytes [0x04, 0x03, 0x02, 0x01] become the word 0x01020304.
 */
function readU32LE(data: Uint8Array, offset: number): number {
  return (
    (data[offset]!) |
    (data[offset + 1]! << 8) |
    (data[offset + 2]! << 16) |
    (data[offset + 3]! << 24)
  ) >>> 0; // >>> 0 forces unsigned 32-bit interpretation
}

/**
 * Write a 32-bit little-endian word into a byte array.
 */
function writeU32LE(data: Uint8Array, offset: number, value: number): void {
  data[offset] = value & 0xff;
  data[offset + 1] = (value >>> 8) & 0xff;
  data[offset + 2] = (value >>> 16) & 0xff;
  data[offset + 3] = (value >>> 24) & 0xff;
}

/**
 * 32-bit left rotation.
 *
 * Rotation moves bits that "fall off" one end back to the other end.
 * For example, rotating 0b1100_0000_..._0000 left by 2 gives
 * 0b0000_0000_..._0011. This is essential for diffusion — it ensures
 * that changes in one bit position affect bits at distant positions
 * after several rounds.
 */
function rotl32(value: number, shift: number): number {
  return ((value << shift) | (value >>> (32 - shift))) >>> 0;
}

/**
 * 32-bit wrapping addition.
 *
 * JavaScript bitwise operators work on signed 32-bit integers, so we
 * use `>>> 0` to reinterpret the result as unsigned. This gives us the
 * same wrapping behavior as hardware 32-bit addition.
 */
function add32(a: number, b: number): number {
  return (a + b) >>> 0;
}

/**
 * The ChaCha20 quarter round — the fundamental mixing operation.
 *
 * It takes four 32-bit words (a, b, c, d) and scrambles them through
 * four ARX steps. Each step combines two words with addition, XORs the
 * result into a third, and rotates to spread the effect across bits:
 *
 *   a += b;  d ^= a;  d <<<= 16    (big rotation — coarse mixing)
 *   c += d;  b ^= c;  b <<<= 12    (medium rotation)
 *   a += b;  d ^= a;  d <<<= 8     (small rotation)
 *   c += d;  b ^= c;  b <<<= 7     (smallest rotation)
 *
 * The rotation amounts (16, 12, 8, 7) were chosen by Bernstein to
 * maximize diffusion: after a few quarter rounds, every output bit
 * depends on every input bit.
 */
function quarterRound(
  state: Uint32Array,
  a: number,
  b: number,
  c: number,
  d: number,
): void {
  state[a] = add32(state[a]!, state[b]!);
  state[d] = rotl32(state[d]! ^ state[a]!, 16);

  state[c] = add32(state[c]!, state[d]!);
  state[b] = rotl32(state[b]! ^ state[c]!, 12);

  state[a] = add32(state[a]!, state[b]!);
  state[d] = rotl32(state[d]! ^ state[a]!, 8);

  state[c] = add32(state[c]!, state[d]!);
  state[b] = rotl32(state[b]! ^ state[c]!, 7);
}

/**
 * Generate one 64-byte ChaCha20 keystream block.
 *
 * The 20 rounds consist of 10 iterations, each performing:
 *   - 4 "column" quarter rounds (down the columns of the 4x4 matrix)
 *   - 4 "diagonal" quarter rounds (along the diagonals)
 *
 * Column rounds mix within columns:        Diagonal rounds mix across columns:
 *   QR(0, 4,  8, 12)                         QR(0, 5, 10, 15)
 *   QR(1, 5,  9, 13)                         QR(1, 6, 11, 12)
 *   QR(2, 6, 10, 14)                         QR(2, 7,  8, 13)
 *   QR(3, 7, 11, 15)                         QR(3, 4,  9, 14)
 *
 * After all rounds, the original state is added back (mod 2^32) to
 * prevent an attacker from inverting the rounds.
 */
function chacha20Block(key: Uint8Array, nonce: Uint8Array, counter: number): Uint8Array {
  // --- Initialize the 4x4 state matrix ---
  const state = new Uint32Array(16);

  // Row 0: constants
  state[0] = CONSTANTS[0];
  state[1] = CONSTANTS[1];
  state[2] = CONSTANTS[2];
  state[3] = CONSTANTS[3];

  // Row 1-2: key (8 words = 32 bytes)
  for (let i = 0; i < 8; i++) {
    state[4 + i] = readU32LE(key, i * 4);
  }

  // Row 3: counter + nonce
  state[12] = counter >>> 0;
  state[13] = readU32LE(nonce, 0);
  state[14] = readU32LE(nonce, 4);
  state[15] = readU32LE(nonce, 8);

  // --- Save original state for the final addition ---
  const original = new Uint32Array(state);

  // --- 20 rounds (10 double-rounds) ---
  for (let i = 0; i < 10; i++) {
    // Column rounds
    quarterRound(state, 0, 4, 8, 12);
    quarterRound(state, 1, 5, 9, 13);
    quarterRound(state, 2, 6, 10, 14);
    quarterRound(state, 3, 7, 11, 15);

    // Diagonal rounds
    quarterRound(state, 0, 5, 10, 15);
    quarterRound(state, 1, 6, 11, 12);
    quarterRound(state, 2, 7, 8, 13);
    quarterRound(state, 3, 4, 9, 14);
  }

  // --- Add original state back ---
  for (let i = 0; i < 16; i++) {
    state[i] = add32(state[i]!, original[i]!);
  }

  // --- Serialize to 64 bytes (little-endian) ---
  const output = new Uint8Array(64);
  for (let i = 0; i < 16; i++) {
    writeU32LE(output, i * 4, state[i]!);
  }

  return output;
}

/**
 * ChaCha20 stream cipher encryption (and decryption — they're the same).
 *
 * ChaCha20 is a stream cipher: it generates a pseudorandom keystream
 * and XORs it with the plaintext. Since XOR is its own inverse,
 * encryption and decryption are the same operation.
 *
 * For messages longer than 64 bytes, we generate multiple keystream
 * blocks by incrementing the counter. The last block may be partial —
 * we only use as many keystream bytes as there are plaintext bytes.
 *
 * @param plaintext  — the data to encrypt (or ciphertext to decrypt)
 * @param key        — 32-byte (256-bit) secret key
 * @param nonce      — 12-byte (96-bit) nonce (number used once)
 * @param counter    — starting block counter (usually 0 or 1)
 * @returns the ciphertext (or plaintext if decrypting)
 */
export function chacha20Encrypt(
  plaintext: Uint8Array,
  key: Uint8Array,
  nonce: Uint8Array,
  counter: number,
): Uint8Array {
  if (key.length !== 32) throw new Error("Key must be 32 bytes");
  if (nonce.length !== 12) throw new Error("Nonce must be 12 bytes");

  const output = new Uint8Array(plaintext.length);
  let offset = 0;

  while (offset < plaintext.length) {
    // Generate the next 64-byte keystream block
    const block = chacha20Block(key, nonce, counter);
    counter++;

    // XOR plaintext with keystream (handle partial last block)
    const remaining = plaintext.length - offset;
    const bytesToProcess = Math.min(64, remaining);

    for (let i = 0; i < bytesToProcess; i++) {
      output[offset + i] = plaintext[offset + i]! ^ block[i]!;
    }

    offset += bytesToProcess;
  }

  return output;
}

// ============================================================================
// Section 2: Poly1305 Message Authentication Code
// ============================================================================
//
// Poly1305 is a one-time MAC: given a 32-byte key and a message, it produces
// a 16-byte authentication tag. "One-time" means each key must be used for
// exactly one message — reusing a key completely breaks security.
//
// The math is elegant: treat 16-byte message chunks as numbers, and evaluate
// a polynomial modulo the prime p = 2^130 - 5:
//
//   tag = ((m₁ · r^n + m₂ · r^(n-1) + ... + mₙ · r) mod p) + s  (mod 2^128)
//
// Where:
//   - r is a "clamped" 128-bit value derived from the first 16 key bytes
//   - s is a 128-bit value from the last 16 key bytes
//   - mᵢ are message chunks (with a 0x01 byte appended to each)
//
// The accumulator form is simpler to implement:
//   acc = 0
//   for each chunk c:
//     acc = (acc + c_with_0x01) * r  mod  (2^130 - 5)
//   tag = (acc + s) mod 2^128
//
// We use JavaScript's native BigInt for the modular arithmetic since
// the numbers involved are 130+ bits wide.
//
// ============================================================================

/**
 * The prime modulus for Poly1305: p = 2^130 - 5.
 *
 * This prime was chosen because:
 *   - It's close to a power of 2, making modular reduction fast
 *   - 130 bits accommodates 128-bit chunks with a leading 1 bit
 *   - The small offset (-5) allows efficient reduction tricks
 */
const POLY1305_P = (1n << 130n) - 5n;

/**
 * Compute a Poly1305 MAC tag for a message.
 *
 * The key is split into two halves:
 *   - r (bytes 0-15): the multiplier, with certain bits "clamped" to zero
 *   - s (bytes 16-31): added at the end to hide the internal state
 *
 * Clamping r ensures that it has a specific algebraic structure that
 * makes the MAC provably secure (it limits r to a subset of values
 * where the security proof holds). The clamped bits are:
 *   - Top 4 bits of bytes 3, 7, 11, 15 cleared (& 0x0f)
 *   - Top 2 bits of bytes 4, 8, 12 cleared (& 0xfc)
 *
 * @param message — the data to authenticate
 * @param key     — 32-byte one-time key (NEVER reuse!)
 * @returns 16-byte authentication tag
 */
export function poly1305Mac(message: Uint8Array, key: Uint8Array): Uint8Array {
  if (key.length !== 32) throw new Error("Key must be 32 bytes");

  // --- Extract and clamp r ---
  // We copy the first 16 bytes and apply the clamping mask in-place.
  const rBytes = new Uint8Array(key.slice(0, 16));

  // Clamp: clear specific bits to constrain r's algebraic properties
  rBytes[3]! &= 0x0f;
  rBytes[7]! &= 0x0f;
  rBytes[11]! &= 0x0f;
  rBytes[15]! &= 0x0f;
  rBytes[4]! &= 0xfc;
  rBytes[8]! &= 0xfc;
  rBytes[12]! &= 0xfc;

  // Convert r from little-endian bytes to a BigInt
  let r = 0n;
  for (let i = 15; i >= 0; i--) {
    r = (r << 8n) | BigInt(rBytes[i]!);
  }

  // --- Extract s (last 16 bytes, little-endian) ---
  let s = 0n;
  for (let i = 15; i >= 0; i--) {
    s = (s << 8n) | BigInt(key[16 + i]!);
  }

  // --- Process message in 16-byte chunks ---
  let acc = 0n;

  for (let i = 0; i < message.length; i += 16) {
    // Read up to 16 bytes of the message
    const end = Math.min(i + 16, message.length);
    const chunkLen = end - i;

    // Convert chunk to little-endian BigInt and append 0x01 byte.
    //
    // The 0x01 byte goes at position `chunkLen` (just past the chunk data).
    // For a full 16-byte chunk, this creates a 17-byte number with the
    // high byte being 0x01. This ensures that all-zero chunks still
    // contribute to the accumulator (without it, a zero chunk would be 0).
    let n = 0n;
    for (let j = chunkLen - 1; j >= 0; j--) {
      n = (n << 8n) | BigInt(message[i + j]!);
    }
    // Append the 0x01 high byte
    n |= 1n << BigInt(8 * chunkLen);

    // Accumulate: acc = (acc + chunk) * r  mod p
    acc = ((acc + n) * r) % POLY1305_P;
  }

  // --- Finalize: tag = (acc + s) mod 2^128 ---
  // We add s and take only the low 128 bits. This step hides the
  // accumulator value (which is mod p) behind the secret s.
  const tag = (acc + s) & ((1n << 128n) - 1n);

  // --- Convert tag to 16 bytes (little-endian) ---
  const result = new Uint8Array(16);
  let temp = tag;
  for (let i = 0; i < 16; i++) {
    result[i] = Number(temp & 0xffn);
    temp >>= 8n;
  }

  return result;
}

// ============================================================================
// Section 3: AEAD — Authenticated Encryption with Associated Data
// ============================================================================
//
// The AEAD construction (RFC 8439 Section 2.8) ties ChaCha20 and Poly1305
// together. It provides:
//
//   - **Confidentiality**: the plaintext is encrypted with ChaCha20
//   - **Integrity**: any tampering with the ciphertext is detected
//   - **Authenticity**: the tag proves the message came from someone with the key
//   - **Associated Data**: unencrypted data (like headers) is also authenticated
//
// The construction:
//
//   1. Derive a one-time Poly1305 key by encrypting 32 zero bytes with
//      ChaCha20 using counter=0. This ensures each (key, nonce) pair
//      produces a unique Poly1305 key.
//
//   2. Encrypt the plaintext with ChaCha20 starting at counter=1.
//      (Counter 0 was used for the Poly1305 key.)
//
//   3. Build the MAC input:
//        AAD || pad16(AAD) || ciphertext || pad16(CT) || le64(len_AAD) || le64(len_CT)
//
//      The padding ensures each field starts at a 16-byte boundary.
//      The lengths at the end prevent length-extension attacks.
//
//   4. Compute tag = Poly1305(poly_key, mac_input).
//
// ============================================================================

/**
 * Pad data length to a 16-byte boundary.
 * Returns zero bytes needed to reach the next multiple of 16.
 */
function pad16(length: number): Uint8Array {
  const remainder = length % 16;
  if (remainder === 0) return new Uint8Array(0);
  return new Uint8Array(16 - remainder);
}

/**
 * Encode a number as an 8-byte little-endian value (le64).
 */
function le64(value: number): Uint8Array {
  const result = new Uint8Array(8);
  // JavaScript numbers are safe up to 2^53, which is plenty for message lengths
  let v = value;
  for (let i = 0; i < 8; i++) {
    result[i] = v & 0xff;
    v = Math.floor(v / 256);
  }
  return result;
}

/**
 * Concatenate multiple Uint8Arrays into one.
 */
function concat(...arrays: Uint8Array[]): Uint8Array {
  const totalLength = arrays.reduce((sum, arr) => sum + arr.length, 0);
  const result = new Uint8Array(totalLength);
  let offset = 0;
  for (const arr of arrays) {
    result.set(arr, offset);
    offset += arr.length;
  }
  return result;
}

/**
 * Construct the Poly1305 MAC input for AEAD.
 *
 * The format (RFC 8439 Section 2.8):
 *
 *   ┌──────────────┐
 *   │ AAD          │  associated data
 *   ├──────────────┤
 *   │ pad to 16    │  zero padding
 *   ├──────────────┤
 *   │ ciphertext   │  encrypted data
 *   ├──────────────┤
 *   │ pad to 16    │  zero padding
 *   ├──────────────┤
 *   │ le64(aadLen) │  8-byte little-endian length of AAD
 *   ├──────────────┤
 *   │ le64(ctLen)  │  8-byte little-endian length of ciphertext
 *   └──────────────┘
 */
function buildMacData(
  aad: Uint8Array,
  ciphertext: Uint8Array,
): Uint8Array {
  return concat(
    aad,
    pad16(aad.length),
    ciphertext,
    pad16(ciphertext.length),
    le64(aad.length),
    le64(ciphertext.length),
  );
}

/**
 * AEAD encryption: encrypt plaintext and produce an authentication tag.
 *
 * @param plaintext — data to encrypt
 * @param key       — 32-byte secret key
 * @param nonce     — 12-byte nonce (must be unique per encryption with same key)
 * @param aad       — associated data to authenticate but not encrypt
 * @returns [ciphertext, tag] — the encrypted data and 16-byte auth tag
 */
export function aeadEncrypt(
  plaintext: Uint8Array,
  key: Uint8Array,
  nonce: Uint8Array,
  aad: Uint8Array,
): [Uint8Array, Uint8Array] {
  if (key.length !== 32) throw new Error("Key must be 32 bytes");
  if (nonce.length !== 12) throw new Error("Nonce must be 12 bytes");

  // Step 1: Generate one-time Poly1305 key using counter=0
  // We encrypt 32 zero bytes to get the key material (first 32 bytes of
  // the 64-byte keystream block).
  const polyKey = chacha20Block(key, nonce, 0).slice(0, 32);

  // Step 2: Encrypt plaintext with ChaCha20 starting at counter=1
  const ciphertext = chacha20Encrypt(plaintext, key, nonce, 1);

  // Step 3: Build MAC input and compute tag
  const macData = buildMacData(aad, ciphertext);
  const tag = poly1305Mac(macData, polyKey);

  return [ciphertext, tag];
}

/**
 * AEAD decryption: verify the tag and decrypt the ciphertext.
 *
 * This is the inverse of aeadEncrypt. Critically, we verify the tag
 * BEFORE returning any plaintext. If the tag doesn't match, the
 * ciphertext has been tampered with and we throw an error.
 *
 * @param ciphertext — encrypted data
 * @param key        — 32-byte secret key
 * @param nonce      — 12-byte nonce (same as used for encryption)
 * @param aad        — associated data (same as used for encryption)
 * @param tag        — 16-byte authentication tag to verify
 * @returns the decrypted plaintext
 * @throws Error if the tag is invalid (tampering detected)
 */
export function aeadDecrypt(
  ciphertext: Uint8Array,
  key: Uint8Array,
  nonce: Uint8Array,
  aad: Uint8Array,
  tag: Uint8Array,
): Uint8Array {
  if (key.length !== 32) throw new Error("Key must be 32 bytes");
  if (nonce.length !== 12) throw new Error("Nonce must be 12 bytes");
  if (tag.length !== 16) throw new Error("Tag must be 16 bytes");

  // Step 1: Generate one-time Poly1305 key
  const polyKey = chacha20Block(key, nonce, 0).slice(0, 32);

  // Step 2: Recompute the tag over the ciphertext
  const macData = buildMacData(aad, ciphertext);
  const computedTag = poly1305Mac(macData, polyKey);

  // Step 3: Constant-time tag comparison
  // We compare all 16 bytes regardless of where a mismatch occurs.
  // This prevents timing attacks that could reveal partial tag info.
  let diff = 0;
  for (let i = 0; i < 16; i++) {
    diff |= computedTag[i]! ^ tag[i]!;
  }
  if (diff !== 0) {
    throw new Error("Authentication failed: tag mismatch");
  }

  // Step 4: Decrypt (ChaCha20 is symmetric — encryption = decryption)
  return chacha20Encrypt(ciphertext, key, nonce, 1);
}
