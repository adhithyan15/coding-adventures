// ============================================================================
// ChaCha20-Poly1305 — Authenticated Encryption (RFC 8439)
// ============================================================================
//
// This crate implements ChaCha20-Poly1305 AEAD from scratch in pure Rust.
// It combines two primitives designed by Daniel J. Bernstein:
//
//   1. **ChaCha20** — a stream cipher built from ARX (Add, Rotate, XOR)
//      operations on 32-bit words. No lookup tables, no timing side channels.
//
//   2. **Poly1305** — a one-time MAC that computes a 16-byte authentication
//      tag using polynomial evaluation modulo the prime 2^130 - 5.
//
// Together they form an Authenticated Encryption with Associated Data (AEAD)
// scheme used in TLS 1.3, WireGuard, SSH, and Chrome/Android.
//
// ## Why ChaCha20 instead of AES?
//
// AES relies on hardware AES-NI instructions for speed and resistance to
// cache-timing attacks. ChaCha20 is fast in *pure software* on any CPU —
// it uses only additions, rotations, and XORs (no S-boxes, no lookup tables).
//
// ============================================================================

// ============================================================================
// Section 1: ChaCha20 Stream Cipher
// ============================================================================
//
// ChaCha20 generates a pseudorandom keystream by scrambling a 4x4 matrix
// of 32-bit words through 20 rounds of quarter-round operations.
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

/// The four 32-bit constants that form the first row of the ChaCha20 state.
/// They spell out "expand 32-byte k" in ASCII — a nothing-up-my-sleeve number
/// chosen by Bernstein to fill the state deterministically.
const CONSTANTS: [u32; 4] = [0x61707865, 0x3320646e, 0x79622d32, 0x6b206574];

/// Read a 32-bit little-endian word from a byte slice.
///
/// ChaCha20 works with 32-bit words stored in little-endian byte order.
/// For example, bytes [0x04, 0x03, 0x02, 0x01] become the word 0x01020304.
fn read_u32_le(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

/// Write a 32-bit little-endian word into a byte slice.
fn write_u32_le(data: &mut [u8], offset: usize, value: u32) {
    let bytes = value.to_le_bytes();
    data[offset..offset + 4].copy_from_slice(&bytes);
}

/// The ChaCha20 quarter round — the fundamental mixing operation.
///
/// It takes four 32-bit words (a, b, c, d) from the state and scrambles
/// them through four ARX steps:
///
///   a += b;  d ^= a;  d <<<= 16    (big rotation — coarse mixing)
///   c += d;  b ^= c;  b <<<= 12    (medium rotation)
///   a += b;  d ^= a;  d <<<= 8     (small rotation)
///   c += d;  b ^= c;  b <<<= 7     (smallest rotation)
///
/// The rotation amounts (16, 12, 8, 7) were chosen by Bernstein to
/// maximize diffusion: after a few quarter rounds, every output bit
/// depends on every input bit.
///
/// Rust's `wrapping_add` ensures we get modular 32-bit arithmetic
/// without overflow panics.
fn quarter_round(state: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize) {
    state[a] = state[a].wrapping_add(state[b]);
    state[d] ^= state[a];
    state[d] = state[d].rotate_left(16);

    state[c] = state[c].wrapping_add(state[d]);
    state[b] ^= state[c];
    state[b] = state[b].rotate_left(12);

    state[a] = state[a].wrapping_add(state[b]);
    state[d] ^= state[a];
    state[d] = state[d].rotate_left(8);

    state[c] = state[c].wrapping_add(state[d]);
    state[b] ^= state[c];
    state[b] = state[b].rotate_left(7);
}

/// Generate one 64-byte ChaCha20 keystream block.
///
/// The 20 rounds consist of 10 iterations, each performing:
///   - 4 "column" quarter rounds (down the columns of the 4x4 matrix)
///   - 4 "diagonal" quarter rounds (along the diagonals)
///
/// After all rounds, the original state is added back (mod 2^32) to
/// prevent an attacker from inverting the rounds.
fn chacha20_block(key: &[u8; 32], nonce: &[u8; 12], counter: u32) -> [u8; 64] {
    // --- Initialize the 4x4 state matrix ---
    let mut state: [u32; 16] = [0; 16];

    // Row 0: constants
    state[0] = CONSTANTS[0];
    state[1] = CONSTANTS[1];
    state[2] = CONSTANTS[2];
    state[3] = CONSTANTS[3];

    // Row 1-2: key (8 words = 32 bytes)
    for i in 0..8 {
        state[4 + i] = read_u32_le(key, i * 4);
    }

    // Row 3: counter + nonce
    state[12] = counter;
    state[13] = read_u32_le(nonce, 0);
    state[14] = read_u32_le(nonce, 4);
    state[15] = read_u32_le(nonce, 8);

    // --- Save original state for the final addition ---
    let original = state;

    // --- 20 rounds (10 double-rounds) ---
    for _ in 0..10 {
        // Column rounds
        quarter_round(&mut state, 0, 4, 8, 12);
        quarter_round(&mut state, 1, 5, 9, 13);
        quarter_round(&mut state, 2, 6, 10, 14);
        quarter_round(&mut state, 3, 7, 11, 15);

        // Diagonal rounds
        quarter_round(&mut state, 0, 5, 10, 15);
        quarter_round(&mut state, 1, 6, 11, 12);
        quarter_round(&mut state, 2, 7, 8, 13);
        quarter_round(&mut state, 3, 4, 9, 14);
    }

    // --- Add original state back ---
    for i in 0..16 {
        state[i] = state[i].wrapping_add(original[i]);
    }

    // --- Serialize to 64 bytes (little-endian) ---
    let mut output = [0u8; 64];
    for i in 0..16 {
        write_u32_le(&mut output, i * 4, state[i]);
    }

    output
}

/// ChaCha20 stream cipher encryption (and decryption — they're the same).
///
/// ChaCha20 is a stream cipher: it generates a pseudorandom keystream
/// and XORs it with the plaintext. Since XOR is its own inverse,
/// encryption and decryption are the same operation.
///
/// For messages longer than 64 bytes, we generate multiple keystream
/// blocks by incrementing the counter. The last block may be partial.
pub fn chacha20_encrypt(
    plaintext: &[u8],
    key: &[u8; 32],
    nonce: &[u8; 12],
    counter: u32,
) -> Vec<u8> {
    let mut output = vec![0u8; plaintext.len()];
    let mut offset = 0;
    let mut current_counter = counter;

    while offset < plaintext.len() {
        let block = chacha20_block(key, nonce, current_counter);
        current_counter = current_counter.wrapping_add(1);

        let remaining = plaintext.len() - offset;
        let bytes_to_process = remaining.min(64);

        for i in 0..bytes_to_process {
            output[offset + i] = plaintext[offset + i] ^ block[i];
        }

        offset += bytes_to_process;
    }

    output
}

// ============================================================================
// Section 2: Poly1305 Message Authentication Code
// ============================================================================
//
// Poly1305 is a one-time MAC: given a 32-byte key and a message, it produces
// a 16-byte authentication tag. "One-time" means each key must be used for
// exactly one message — reusing a key completely breaks security.
//
// The math: treat 16-byte message chunks as numbers and evaluate a polynomial
// modulo p = 2^130 - 5:
//
//   acc = 0
//   for each chunk c:
//     acc = (acc + c_with_0x01) * r  mod  p
//   tag = (acc + s) mod 2^128
//
// For the arithmetic, we need numbers up to about 130 bits. Rust's u128 can
// hold 128 bits, but we need 130+ bits for intermediate values. We use a
// pair of u128 values to represent a 256-bit intermediate during multiplication,
// or we can use a simpler approach: since r is clamped to have certain bits
// zero, the product fits within manageable bounds.
//
// Our approach: represent the accumulator as five 26-bit limbs, which allows
// us to use u64 arithmetic for multiplication without overflow. However, for
// clarity and correctness, we'll use a simpler approach with u128 and careful
// modular reduction.
//
// ============================================================================

/// The prime modulus for Poly1305: p = 2^130 - 5.
///
/// Since this doesn't fit in a u128 (which holds up to 2^128 - 1), we
/// represent it conceptually and handle the modular arithmetic carefully.

/// Read a 128-bit little-endian value from a byte slice (up to 17 bytes).
/// Returns the value as a (high, low) pair where high holds overflow bits.
fn read_le_bytes_to_u128(data: &[u8]) -> u128 {
    let mut result: u128 = 0;
    for (i, &byte) in data.iter().enumerate() {
        result |= (byte as u128) << (i * 8);
    }
    result
}

/// Multiply two 130-bit values and reduce modulo 2^130 - 5.
///
/// Since r is at most 124 bits (after clamping) and acc is at most 131 bits,
/// their product can be up to 255 bits. We need to handle this carefully.
///
/// We split the accumulator into 5 limbs of 26 bits each and use u64
/// multiplication to avoid overflow. This is the standard approach used
/// in production Poly1305 implementations.
///
/// However, for educational clarity, we use a different approach: we represent
/// numbers as (high_2_bits, low_128_bits) and perform modular arithmetic
/// using the identity: 2^130 ≡ 5 (mod p).
///
/// Any number n can be written as n = n_high * 2^130 + n_low, and then
/// n mod p = n_low + n_high * 5 (mod p).
fn mod_multiply(a_lo: u128, a_hi: u8, r: u128) -> (u128, u8) {
    // We need to compute (a * r) mod (2^130 - 5).
    // a = a_hi * 2^128 + a_lo, where a_hi is at most 3 (2 bits).
    // r is at most 124 bits (after clamping).
    //
    // a * r = a_lo * r + a_hi * r * 2^128
    //
    // We compute a_lo * r using 128-bit multiplication with carry.

    // Split r and a_lo into 64-bit halves for multiplication
    let r_lo = r as u64 as u128;
    let r_hi = (r >> 64) as u64 as u128;

    let al_lo = a_lo as u64 as u128;
    let al_hi = (a_lo >> 64) as u64 as u128;

    // Compute a_lo * r in four 64x64->128 multiplications
    let t0 = al_lo * r_lo;
    let t1 = al_lo * r_hi;
    let t2 = al_hi * r_lo;
    let t3 = al_hi * r_hi;

    // Combine:
    // result = t0 + (t1 + t2) << 64 + t3 << 128
    let (mid, carry1) = t1.overflowing_add(t2);
    let carry1 = if carry1 { 1u128 << 64 } else { 0 };

    let (lo, carry2) = t0.overflowing_add(mid << 64);
    let hi = t3 + (mid >> 64) + carry1 + if carry2 { 1u128 } else { 0 };

    // Now add a_hi * r * 2^128
    let a_hi_r = (a_hi as u128) * r;
    let (hi, _) = hi.overflowing_add(a_hi_r);

    // We have a 256-bit result in (hi, lo).
    // Reduce modulo 2^130 - 5 using the identity: 2^130 ≡ 5 (mod p).
    //
    // Split into: value = bits[0..130) + bits[130..256) * 2^130
    // reduced = bits[0..130) + bits[130..256) * 5
    reduce_mod_p(lo, hi)
}

/// Reduce a 256-bit value (lo, hi) modulo 2^130 - 5.
///
/// The key insight: since p = 2^130 - 5, we have 2^130 ≡ 5 (mod p).
/// So any value can be split at the 130-bit boundary and the upper part
/// is multiplied by 5 and added to the lower part.
fn reduce_mod_p(lo: u128, hi: u128) -> (u128, u8) {
    // Extract bits [0..128) and [128..130) from lo
    let lo_128 = lo;
    // Extract bits [128..130) from lo and all of hi to form the upper part
    // The 256-bit number is: hi * 2^128 + lo
    // We want to split at bit 130:
    //   lower_130 = (lo & ((1<<130)-1))      -- but 130 bits doesn't fit in u128
    //   upper = (hi * 2^128 + lo) >> 130
    //
    // lower_130 = lo[0..128] + hi[0..2] * 2^128 (the low 130 bits)
    // upper_126 = hi >> 2 (the bits above 130)
    //
    // Wait, let me reconsider. The 256-bit value is hi:lo where:
    //   bit 0..127 = lo
    //   bit 128..255 = hi
    //
    // Split at bit 130:
    //   lower = bits 0..129 = lo[0..127] | hi[0..1] << 128
    //   upper = bits 130..255 = hi >> 2

    let lower_128 = lo_128;
    let lower_hi2 = (hi & 0x3) as u8; // bits 128-129
    // lower_130 = lower_hi2 * 2^128 + lower_128

    let upper = hi >> 2; // bits 130 and above

    // reduced = lower_130 + upper * 5
    let product = upper * 5;
    let (new_lo, carry) = lower_128.overflowing_add(product);
    let mut new_hi = lower_hi2 + if carry { 1 } else { 0 };

    // If new_hi >= 4, we have bits above 130 again — reduce once more
    // (at most one more reduction needed since upper*5 is much smaller)
    if new_hi >= 4 {
        let overflow = (new_hi >> 2) as u128;
        new_hi &= 0x3;
        let (new_lo2, carry2) = new_lo.overflowing_add(overflow * 5);
        if carry2 {
            new_hi += 1;
        }
        return (new_lo2, new_hi);
    }

    (new_lo, new_hi)
}

/// Add two 130-bit values represented as (lo: u128, hi: u8).
fn add_130(a_lo: u128, a_hi: u8, b_lo: u128, b_hi: u8) -> (u128, u8) {
    let (lo, carry) = a_lo.overflowing_add(b_lo);
    let hi = a_hi + b_hi + if carry { 1 } else { 0 };
    (lo, hi)
}

/// Compute a Poly1305 MAC tag for a message.
///
/// The key is split into two halves:
///   - r (bytes 0-15): the multiplier, with certain bits "clamped" to zero
///   - s (bytes 16-31): added at the end to hide the internal state
///
/// Clamping r ensures it has a specific algebraic structure that makes
/// the MAC provably secure.
pub fn poly1305_mac(message: &[u8], key: &[u8; 32]) -> [u8; 16] {
    // --- Extract and clamp r ---
    let mut r_bytes = [0u8; 16];
    r_bytes.copy_from_slice(&key[0..16]);

    // Clamp: clear specific bits to constrain r's algebraic properties
    r_bytes[3] &= 0x0f;
    r_bytes[7] &= 0x0f;
    r_bytes[11] &= 0x0f;
    r_bytes[15] &= 0x0f;
    r_bytes[4] &= 0xfc;
    r_bytes[8] &= 0xfc;
    r_bytes[12] &= 0xfc;

    let r = read_le_bytes_to_u128(&r_bytes);

    // --- Extract s (last 16 bytes, little-endian) ---
    let s = read_le_bytes_to_u128(&key[16..32]);

    // --- Process message in 16-byte chunks ---
    let mut acc_lo: u128 = 0;
    let mut acc_hi: u8 = 0;

    let mut i = 0;
    while i < message.len() {
        let end = (i + 16).min(message.len());
        let chunk = &message[i..end];
        let chunk_len = chunk.len();

        // Convert chunk to little-endian number and append 0x01 byte.
        // The 0x01 byte ensures all-zero chunks still contribute.
        let mut n_lo = read_le_bytes_to_u128(chunk);
        let mut n_hi: u8 = 0;

        // Place the 0x01 bit at position (chunk_len * 8)
        if chunk_len < 16 {
            n_lo |= 1u128 << (chunk_len * 8);
        } else {
            // chunk_len == 16, so bit 128
            n_hi = 1;
        }

        // acc = (acc + n) * r mod p
        let (sum_lo, sum_hi) = add_130(acc_lo, acc_hi, n_lo, n_hi);
        let (new_lo, new_hi) = mod_multiply(sum_lo, sum_hi, r);
        acc_lo = new_lo;
        acc_hi = new_hi;

        i += 16;
    }

    // --- Finalize: tag = (acc + s) mod 2^128 ---
    // We add s (a 128-bit value) and take only the low 128 bits.
    let tag = acc_lo.wrapping_add(s);

    // Convert tag to 16 bytes (little-endian)
    tag.to_le_bytes()
}

// ============================================================================
// Section 3: AEAD — Authenticated Encryption with Associated Data
// ============================================================================
//
// The AEAD construction (RFC 8439 Section 2.8) ties ChaCha20 and Poly1305
// together:
//
//   1. Derive a one-time Poly1305 key from ChaCha20(key, nonce, counter=0)
//   2. Encrypt plaintext with ChaCha20 starting at counter=1
//   3. Build MAC input: AAD || pad || ciphertext || pad || lengths
//   4. Compute tag = Poly1305(poly_key, mac_input)
//
// ============================================================================

/// Pad data length to a 16-byte boundary.
fn pad16(length: usize) -> Vec<u8> {
    let remainder = length % 16;
    if remainder == 0 {
        Vec::new()
    } else {
        vec![0u8; 16 - remainder]
    }
}

/// Encode a number as 8 bytes little-endian (le64).
fn le64(value: u64) -> [u8; 8] {
    value.to_le_bytes()
}

/// Build the Poly1305 MAC input for AEAD (RFC 8439 Section 2.8).
///
///   AAD || pad16(AAD) || ciphertext || pad16(CT) || le64(len_AAD) || le64(len_CT)
fn build_mac_data(aad: &[u8], ciphertext: &[u8]) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(aad);
    data.extend(pad16(aad.len()));
    data.extend_from_slice(ciphertext);
    data.extend(pad16(ciphertext.len()));
    data.extend_from_slice(&le64(aad.len() as u64));
    data.extend_from_slice(&le64(ciphertext.len() as u64));
    data
}

/// AEAD encryption: encrypt plaintext and produce an authentication tag.
///
/// Returns (ciphertext, tag) where tag is 16 bytes.
pub fn aead_encrypt(
    plaintext: &[u8],
    key: &[u8; 32],
    nonce: &[u8; 12],
    aad: &[u8],
) -> (Vec<u8>, [u8; 16]) {
    // Step 1: Generate one-time Poly1305 key using counter=0
    let poly_block = chacha20_block(key, nonce, 0);
    let mut poly_key = [0u8; 32];
    poly_key.copy_from_slice(&poly_block[0..32]);

    // Step 2: Encrypt plaintext with ChaCha20 starting at counter=1
    let ciphertext = chacha20_encrypt(plaintext, key, nonce, 1);

    // Step 3: Build MAC input and compute tag
    let mac_data = build_mac_data(aad, &ciphertext);
    let tag = poly1305_mac(&mac_data, &poly_key);

    (ciphertext, tag)
}

/// AEAD decryption: verify the tag and decrypt the ciphertext.
///
/// Returns `Some(plaintext)` if the tag is valid, or `None` if
/// authentication fails (ciphertext was tampered with).
pub fn aead_decrypt(
    ciphertext: &[u8],
    key: &[u8; 32],
    nonce: &[u8; 12],
    aad: &[u8],
    tag: &[u8; 16],
) -> Option<Vec<u8>> {
    // Step 1: Generate one-time Poly1305 key
    let poly_block = chacha20_block(key, nonce, 0);
    let mut poly_key = [0u8; 32];
    poly_key.copy_from_slice(&poly_block[0..32]);

    // Step 2: Recompute the tag
    let mac_data = build_mac_data(aad, ciphertext);
    let computed_tag = poly1305_mac(&mac_data, &poly_key);

    // Step 3: Constant-time tag comparison
    let mut diff = 0u8;
    for i in 0..16 {
        diff |= computed_tag[i] ^ tag[i];
    }
    if diff != 0 {
        return None;
    }

    // Step 4: Decrypt
    Some(chacha20_encrypt(ciphertext, key, nonce, 1))
}

// ============================================================================
// Section 4: HChaCha20 + XChaCha20-Poly1305 AEAD
// ============================================================================
//
// The standard ChaCha20-Poly1305 AEAD uses a 96-bit (12-byte) nonce, which is
// dangerously short for random-nonce systems: the birthday bound on accidental
// reuse is only 2^48 messages.  XChaCha20-Poly1305 extends the nonce to 192
// bits (24 bytes) via a two-step construction:
//
//   1. HChaCha20(key, nonce[0..16]) -> 32-byte subkey
//   2. Run standard ChaCha20-Poly1305 with
//        chacha_key   = subkey
//        chacha_nonce = 0x00000000 || nonce[16..24]      (12 bytes)
//
// HChaCha20 is the ChaCha20 round function over a modified initial state:
// the 32-bit counter and 96-bit nonce are replaced by a single 128-bit nonce.
// After 20 rounds, the output is *NOT* added back to the original state;
// instead, we take state[0..4] and state[12..15] (32 bytes total) as the
// subkey.  That "no feed-forward" property is what makes HChaCha20 a PRF
// under the assumption that ChaCha20's round function is a pseudorandom
// permutation.
//
// Reference: draft-irtf-cfrg-xchacha (Arciszewski),
//            §2.2 (HChaCha20), §2.3 (XChaCha20-Poly1305), §A (test vectors).

/// HChaCha20 -- derive a 32-byte subkey from (key, 16-byte nonce).
///
/// Used as the first stage of XChaCha20-Poly1305 and of XChaCha20 on its
/// own.  Unlike `chacha20_block`, HChaCha20 does *not* add the original
/// state back after the 20 rounds -- instead it takes four words from
/// row 0 and four words from row 3 of the post-round state.  Those are
/// the rows that do NOT contain the key, which is what breaks any
/// trivial relationship between input and output.
pub fn hchacha20_subkey(key: &[u8; 32], nonce16: &[u8; 16]) -> [u8; 32] {
    let mut state: [u32; 16] = [0; 16];

    // Row 0: the four ChaCha20 constants ("expand 32-byte k").
    state[0] = CONSTANTS[0];
    state[1] = CONSTANTS[1];
    state[2] = CONSTANTS[2];
    state[3] = CONSTANTS[3];

    // Rows 1-2: 256-bit key.
    for i in 0..8 {
        state[4 + i] = read_u32_le(key, i * 4);
    }

    // Row 3: the full 128-bit input nonce (no counter in HChaCha20).
    for i in 0..4 {
        state[12 + i] = read_u32_le(nonce16, i * 4);
    }

    // 20 rounds, identical to ChaCha20's column/diagonal double-rounds.
    for _ in 0..10 {
        quarter_round(&mut state, 0, 4, 8, 12);
        quarter_round(&mut state, 1, 5, 9, 13);
        quarter_round(&mut state, 2, 6, 10, 14);
        quarter_round(&mut state, 3, 7, 11, 15);

        quarter_round(&mut state, 0, 5, 10, 15);
        quarter_round(&mut state, 1, 6, 11, 12);
        quarter_round(&mut state, 2, 7, 8, 13);
        quarter_round(&mut state, 3, 4, 9, 14);
    }

    // NO feed-forward.  Take state[0..4] (row 0, post-round) plus
    // state[12..16] (row 3, post-round) as the 32-byte subkey.
    let mut subkey = [0u8; 32];
    for i in 0..4 {
        write_u32_le(&mut subkey, i * 4, state[i]);
        write_u32_le(&mut subkey, 16 + i * 4, state[12 + i]);
    }
    subkey
}

/// XChaCha20 stream cipher -- 256-bit key + 192-bit (24-byte) nonce.
///
/// Derives a subkey via HChaCha20 using the first 16 bytes of the nonce,
/// then runs standard ChaCha20 with subkey and the 12-byte nonce
/// `[0, 0, 0, 0] || nonce[16..24]`.  Starting counter is caller-chosen.
pub fn xchacha20_encrypt(
    plaintext: &[u8],
    key: &[u8; 32],
    nonce24: &[u8; 24],
    counter: u32,
) -> Vec<u8> {
    let mut n16 = [0u8; 16];
    n16.copy_from_slice(&nonce24[0..16]);
    let subkey = hchacha20_subkey(key, &n16);

    let mut n12 = [0u8; 12];
    // First 4 bytes of the ChaCha20 nonce are zeroed; last 8 bytes come
    // from the second half of the 24-byte XChaCha20 nonce.
    n12[4..12].copy_from_slice(&nonce24[16..24]);

    chacha20_encrypt(plaintext, &subkey, &n12, counter)
}

/// XChaCha20-Poly1305 AEAD encryption.
///
/// Returns `(ciphertext, 16-byte tag)`.  Matches the construction in
/// draft-irtf-cfrg-xchacha §2.3: derive a subkey with HChaCha20, then
/// run the RFC 8439 AEAD with that subkey and a 12-byte chacha nonce
/// of `[0,0,0,0] || nonce24[16..24]`.
pub fn xchacha20_poly1305_aead_encrypt(
    plaintext: &[u8],
    key: &[u8; 32],
    nonce24: &[u8; 24],
    aad: &[u8],
) -> (Vec<u8>, [u8; 16]) {
    let mut n16 = [0u8; 16];
    n16.copy_from_slice(&nonce24[0..16]);
    let subkey = hchacha20_subkey(key, &n16);

    let mut n12 = [0u8; 12];
    n12[4..12].copy_from_slice(&nonce24[16..24]);

    aead_encrypt(plaintext, &subkey, &n12, aad)
}

/// XChaCha20-Poly1305 AEAD decryption.
///
/// Returns `Some(plaintext)` on a valid tag, or `None` on any
/// authentication failure.  Tag comparison is constant-time (delegated
/// to `aead_decrypt`).
pub fn xchacha20_poly1305_aead_decrypt(
    ciphertext: &[u8],
    key: &[u8; 32],
    nonce24: &[u8; 24],
    aad: &[u8],
    tag: &[u8; 16],
) -> Option<Vec<u8>> {
    let mut n16 = [0u8; 16];
    n16.copy_from_slice(&nonce24[0..16]);
    let subkey = hchacha20_subkey(key, &n16);

    let mut n12 = [0u8; 12];
    n12[4..12].copy_from_slice(&nonce24[16..24]);

    aead_decrypt(ciphertext, &subkey, &n12, aad, tag)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---- ChaCha20 Tests ----

    #[test]
    fn test_chacha20_rfc8439_section_2_4_2() {
        // RFC 8439 Section 2.4.2 — the canonical "sunscreen" test vector.
        let key: [u8; 32] = [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a,
            0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15,
            0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f,
        ];
        let nonce: [u8; 12] = [
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x4a, 0x00, 0x00, 0x00,
            0x00,
        ];
        let plaintext = b"Ladies and Gentlemen of the class of '99: If I could offer you only one tip for the future, sunscreen would be it.";

        let expected_ct = hex::decode(
            "6e2e359a2568f98041ba0728dd0d6981\
             e97e7aec1d4360c20a27afccfd9fae0b\
             f91b65c5524733ab8f593dabcd62b357\
             1639d624e65152ab8f530c359f0861d8\
             07ca0dbf500d6a6156a38e088a22b65e\
             52bc514d16ccf806818ce91ab7793736\
             5af90bbf74a35be6b40b8eedf2785e42\
             874d",
        )
        .unwrap();

        let ciphertext = chacha20_encrypt(plaintext, &key, &nonce, 1);
        assert_eq!(ciphertext, expected_ct);

        // Verify round-trip (XOR is its own inverse)
        let decrypted = chacha20_encrypt(&ciphertext, &key, &nonce, 1);
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_chacha20_empty() {
        let key = [0u8; 32];
        let nonce = [0u8; 12];
        let result = chacha20_encrypt(&[], &key, &nonce, 0);
        assert!(result.is_empty());
    }

    #[test]
    fn test_chacha20_single_byte() {
        let mut key = [0u8; 32];
        key[0] = 1;
        let nonce = [0u8; 12];
        let ct = chacha20_encrypt(&[0x42], &key, &nonce, 0);
        assert_eq!(ct.len(), 1);
        let pt = chacha20_encrypt(&ct, &key, &nonce, 0);
        assert_eq!(pt, vec![0x42]);
    }

    #[test]
    fn test_chacha20_multiblock() {
        let mut key = [0u8; 32];
        for i in 0..32 {
            key[i] = i as u8;
        }
        let mut nonce = [0u8; 12];
        nonce[0] = 0x09;
        let mut plaintext = vec![0u8; 200];
        for i in 0..200 {
            plaintext[i] = (i % 256) as u8;
        }

        let ct = chacha20_encrypt(&plaintext, &key, &nonce, 0);
        assert_eq!(ct.len(), 200);
        let pt = chacha20_encrypt(&ct, &key, &nonce, 0);
        assert_eq!(pt, plaintext);
    }

    #[test]
    fn test_chacha20_different_keys() {
        let mut key1 = [0u8; 32];
        key1[0] = 1;
        let mut key2 = [0u8; 32];
        key2[0] = 2;
        let nonce = [0u8; 12];
        let plaintext = b"Hello, World!";
        let ct1 = chacha20_encrypt(plaintext, &key1, &nonce, 0);
        let ct2 = chacha20_encrypt(plaintext, &key2, &nonce, 0);
        assert_ne!(ct1, ct2);
    }

    // ---- Poly1305 Tests ----

    #[test]
    fn test_poly1305_rfc8439_section_2_5_2() {
        let key: [u8; 32] = hex::decode(
            "85d6be7857556d337f4452fe42d506a80103808afb0db2fd4abff6af4149f51b",
        )
        .unwrap()
        .try_into()
        .unwrap();
        let message = b"Cryptographic Forum Research Group";
        let expected_tag: [u8; 16] =
            hex::decode("a8061dc1305136c6c22b8baf0c0127a9")
                .unwrap()
                .try_into()
                .unwrap();

        let tag = poly1305_mac(message, &key);
        assert_eq!(tag, expected_tag);
    }

    #[test]
    fn test_poly1305_empty() {
        let mut key = [0u8; 32];
        for i in 0..32 {
            key[i] = i as u8;
        }
        let tag = poly1305_mac(&[], &key);
        assert_eq!(tag.len(), 16);
    }

    #[test]
    fn test_poly1305_single_byte() {
        let mut key = [0u8; 32];
        for i in 0..32 {
            key[i] = i as u8;
        }
        let tag = poly1305_mac(&[0x42], &key);
        assert_eq!(tag.len(), 16);
    }

    #[test]
    fn test_poly1305_different_messages() {
        let key: [u8; 32] = hex::decode(
            "85d6be7857556d337f4452fe42d506a80103808afb0db2fd4abff6af4149f51b",
        )
        .unwrap()
        .try_into()
        .unwrap();
        let tag1 = poly1305_mac(b"Message A", &key);
        let tag2 = poly1305_mac(b"Message B", &key);
        assert_ne!(tag1, tag2);
    }

    #[test]
    fn test_poly1305_exactly_16_bytes() {
        let mut key = [0u8; 32];
        for i in 0..32 {
            key[i] = i as u8;
        }
        let mut msg = [0u8; 16];
        for i in 0..16 {
            msg[i] = i as u8;
        }
        let tag = poly1305_mac(&msg, &key);
        assert_eq!(tag.len(), 16);
    }

    // ---- AEAD Tests ----

    #[test]
    fn test_aead_rfc8439_section_2_8_2() {
        let key: [u8; 32] = hex::decode(
            "808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9f",
        )
        .unwrap()
        .try_into()
        .unwrap();
        let nonce: [u8; 12] = hex::decode("070000004041424344454647")
            .unwrap()
            .try_into()
            .unwrap();
        let aad = hex::decode("50515253c0c1c2c3c4c5c6c7").unwrap();
        let plaintext = b"Ladies and Gentlemen of the class of '99: If I could offer you only one tip for the future, sunscreen would be it.";

        let expected_ct = hex::decode(
            "d31a8d34648e60db7b86afbc53ef7ec2\
             a4aded51296e08fea9e2b5a736ee62d6\
             3dbea45e8ca9671282fafb69da92728b\
             1a71de0a9e060b2905d6a5b67ecd3b36\
             92ddbd7f2d778b8c9803aee328091b58\
             fab324e4fad675945585808b4831d7bc\
             3ff4def08e4b7a9de576d26586cec64b\
             6116",
        )
        .unwrap();
        let expected_tag: [u8; 16] =
            hex::decode("1ae10b594f09e26a7e902ecbd0600691")
                .unwrap()
                .try_into()
                .unwrap();

        let (ciphertext, tag) = aead_encrypt(plaintext, &key, &nonce, &aad);
        assert_eq!(ciphertext, expected_ct);
        assert_eq!(tag, expected_tag);
    }

    #[test]
    fn test_aead_round_trip() {
        let key: [u8; 32] = hex::decode(
            "808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9f",
        )
        .unwrap()
        .try_into()
        .unwrap();
        let nonce: [u8; 12] = hex::decode("070000004041424344454647")
            .unwrap()
            .try_into()
            .unwrap();
        let aad = hex::decode("50515253c0c1c2c3c4c5c6c7").unwrap();
        let plaintext = b"Ladies and Gentlemen of the class of '99: If I could offer you only one tip for the future, sunscreen would be it.";

        let (ciphertext, tag) = aead_encrypt(plaintext, &key, &nonce, &aad);
        let decrypted = aead_decrypt(&ciphertext, &key, &nonce, &aad, &tag);
        assert_eq!(decrypted, Some(plaintext.to_vec()));
    }

    #[test]
    fn test_aead_tampered_ciphertext() {
        let key: [u8; 32] = hex::decode(
            "808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9f",
        )
        .unwrap()
        .try_into()
        .unwrap();
        let nonce: [u8; 12] = hex::decode("070000004041424344454647")
            .unwrap()
            .try_into()
            .unwrap();
        let aad = hex::decode("50515253c0c1c2c3c4c5c6c7").unwrap();
        let plaintext = b"Secret message";

        let (mut ciphertext, tag) = aead_encrypt(plaintext, &key, &nonce, &aad);
        ciphertext[0] ^= 0x01;
        assert_eq!(aead_decrypt(&ciphertext, &key, &nonce, &aad, &tag), None);
    }

    #[test]
    fn test_aead_tampered_aad() {
        let key: [u8; 32] = hex::decode(
            "808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9f",
        )
        .unwrap()
        .try_into()
        .unwrap();
        let nonce: [u8; 12] = hex::decode("070000004041424344454647")
            .unwrap()
            .try_into()
            .unwrap();
        let mut aad = hex::decode("50515253c0c1c2c3c4c5c6c7").unwrap();
        let plaintext = b"Secret message";

        let (ciphertext, tag) = aead_encrypt(plaintext, &key, &nonce, &aad);
        aad[0] ^= 0x01;
        assert_eq!(aead_decrypt(&ciphertext, &key, &nonce, &aad, &tag), None);
    }

    #[test]
    fn test_aead_empty_plaintext() {
        let mut key = [0u8; 32];
        for i in 0..32 {
            key[i] = i as u8;
        }
        let mut nonce = [0u8; 12];
        nonce[0] = 7;
        let aad = b"header data";

        let (ct, tag) = aead_encrypt(&[], &key, &nonce, aad);
        assert!(ct.is_empty());
        assert_eq!(tag.len(), 16);

        let pt = aead_decrypt(&ct, &key, &nonce, aad, &tag);
        assert_eq!(pt, Some(vec![]));
    }

    #[test]
    fn test_aead_empty_aad() {
        let mut key = [0u8; 32];
        for i in 0..32 {
            key[i] = i as u8;
        }
        let nonce = [0u8; 12];
        let plaintext = b"Hello, World!";

        let (ct, tag) = aead_encrypt(plaintext, &key, &nonce, &[]);
        let pt = aead_decrypt(&ct, &key, &nonce, &[], &tag);
        assert_eq!(pt, Some(plaintext.to_vec()));
    }

    #[test]
    fn test_aead_large_plaintext() {
        let mut key = [0u8; 32];
        for i in 0..32 {
            key[i] = i as u8;
        }
        let mut nonce = [0u8; 12];
        nonce[4] = 0xab;
        let aad = b"extra data";
        let mut plaintext = vec![0u8; 500];
        for i in 0..500 {
            plaintext[i] = (i % 256) as u8;
        }

        let (ct, tag) = aead_encrypt(&plaintext, &key, &nonce, aad);
        assert_eq!(ct.len(), 500);
        let pt = aead_decrypt(&ct, &key, &nonce, aad, &tag);
        assert_eq!(pt, Some(plaintext));
    }

    #[test]
    fn test_aead_wrong_tag() {
        let key = [0u8; 32];
        let nonce = [0u8; 12];
        let plaintext = b"test";

        let (ciphertext, _tag) = aead_encrypt(plaintext, &key, &nonce, &[]);
        let wrong_tag = [0u8; 16];
        assert_eq!(
            aead_decrypt(&ciphertext, &key, &nonce, &[], &wrong_tag),
            None
        );
    }

    // ------------------------------------------------------------------
    // HChaCha20 + XChaCha20-Poly1305 (draft-irtf-cfrg-xchacha)
    // ------------------------------------------------------------------

    #[test]
    fn test_hchacha20_draft_section_2_2_1() {
        // From draft-irtf-cfrg-xchacha §2.2.1 -- the canonical HChaCha20
        // test vector.
        let key = hex::decode(
            "000102030405060708090a0b0c0d0e0f\
             101112131415161718191a1b1c1d1e1f",
        )
        .unwrap();
        let nonce = hex::decode("000000090000004a0000000031415927").unwrap();
        let expected = hex::decode(
            "82413b4227b27bfed30e42508a877d73\
             a0f9e4d58a74a853c12ec41326d3ecdc",
        )
        .unwrap();

        let mut k = [0u8; 32];
        k.copy_from_slice(&key);
        let mut n = [0u8; 16];
        n.copy_from_slice(&nonce);

        let subkey = hchacha20_subkey(&k, &n);
        assert_eq!(subkey.to_vec(), expected);
    }

    #[test]
    fn test_xchacha20_poly1305_draft_appendix_a3() {
        // draft-irtf-cfrg-xchacha §A.3 -- the gold-standard AEAD vector.
        let key = hex::decode(
            "808182838485868788898a8b8c8d8e8f\
             909192939495969798999a9b9c9d9e9f",
        )
        .unwrap();
        let nonce = hex::decode(
            "404142434445464748494a4b4c4d4e4f5051525354555657",
        )
        .unwrap();
        let aad = hex::decode("50515253c0c1c2c3c4c5c6c7").unwrap();
        let plaintext = b"Ladies and Gentlemen of the class of '99: If I could offer you only one tip for the future, sunscreen would be it.";

        let expected_ct = hex::decode(
            "bd6d179d3e83d43b9576579493c0e939\
             572a1700252bfaccbed2902c21396cbb\
             731c7f1b0b4aa6440bf3a82f4eda7e39\
             ae64c6708c54c216cb96b72e1213b452\
             2f8c9ba40db5d945b11b69b982c1bb9e\
             3f3fac2bc369488f76b2383565d3fff9\
             21f9664c97637da9768812f615c68b13\
             b52e",
        )
        .unwrap();
        let expected_tag =
            hex::decode("c0875924c1c7987947deafd8780acf49").unwrap();

        let mut k = [0u8; 32];
        k.copy_from_slice(&key);
        let mut n = [0u8; 24];
        n.copy_from_slice(&nonce);
        let mut t_expected = [0u8; 16];
        t_expected.copy_from_slice(&expected_tag);

        let (ct, tag) =
            xchacha20_poly1305_aead_encrypt(plaintext, &k, &n, &aad);
        assert_eq!(ct, expected_ct);
        assert_eq!(tag, t_expected);

        // Round-trip: a correct tag must decrypt.
        let pt = xchacha20_poly1305_aead_decrypt(&ct, &k, &n, &aad, &tag);
        assert_eq!(pt.as_deref(), Some(&plaintext[..]));
    }

    #[test]
    fn test_xchacha20_poly1305_wrong_tag_rejected() {
        let key = [0u8; 32];
        let nonce = [0u8; 24];
        let plaintext = b"hello";
        let (ct, _tag) =
            xchacha20_poly1305_aead_encrypt(plaintext, &key, &nonce, &[]);
        let wrong_tag = [0u8; 16];
        assert_eq!(
            xchacha20_poly1305_aead_decrypt(
                &ct,
                &key,
                &nonce,
                &[],
                &wrong_tag
            ),
            None,
        );
    }

    #[test]
    fn test_xchacha20_poly1305_aad_binding() {
        let key = [7u8; 32];
        let nonce = [9u8; 24];
        let plaintext = b"secret";

        let (ct, tag) =
            xchacha20_poly1305_aead_encrypt(plaintext, &key, &nonce, b"ctx-a");

        // Decrypt with the *wrong* AAD must fail.
        assert_eq!(
            xchacha20_poly1305_aead_decrypt(
                &ct,
                &key,
                &nonce,
                b"ctx-b",
                &tag
            ),
            None,
        );
        // Decrypt with the *right* AAD must succeed.
        assert_eq!(
            xchacha20_poly1305_aead_decrypt(
                &ct,
                &key,
                &nonce,
                b"ctx-a",
                &tag
            )
            .as_deref(),
            Some(&plaintext[..]),
        );
    }

    #[test]
    fn test_xchacha20_poly1305_tampered_ciphertext() {
        let key = [0x42u8; 32];
        let nonce = [0x24u8; 24];
        let plaintext = b"do not flip this byte";
        let (mut ct, tag) =
            xchacha20_poly1305_aead_encrypt(plaintext, &key, &nonce, &[]);

        // Flip one bit in the middle of the ciphertext.
        ct[5] ^= 0x01;
        assert_eq!(
            xchacha20_poly1305_aead_decrypt(&ct, &key, &nonce, &[], &tag),
            None,
        );
    }

    #[test]
    fn test_xchacha20_poly1305_long_message() {
        // Exercise the multi-block path through ChaCha20.
        let key = [0x11u8; 32];
        let nonce = [0x22u8; 24];
        let mut plaintext = vec![0u8; 4096];
        for (i, b) in plaintext.iter_mut().enumerate() {
            *b = (i % 251) as u8;
        }

        let (ct, tag) =
            xchacha20_poly1305_aead_encrypt(&plaintext, &key, &nonce, b"ctx");
        assert_eq!(ct.len(), plaintext.len());
        let pt = xchacha20_poly1305_aead_decrypt(
            &ct,
            &key,
            &nonce,
            b"ctx",
            &tag,
        );
        assert_eq!(pt, Some(plaintext));
    }

    #[test]
    fn test_xchacha20_encrypt_round_trip() {
        // XChaCha20 (no Poly1305 -- stream cipher on its own) round-trip.
        let key = [0x5au8; 32];
        let nonce = [0xa5u8; 24];
        let plaintext = b"raw stream cipher test, no AEAD";
        let ct = xchacha20_encrypt(plaintext, &key, &nonce, 1);
        assert_ne!(ct, plaintext.to_vec());
        let pt = xchacha20_encrypt(&ct, &key, &nonce, 1);
        assert_eq!(pt, plaintext);
    }
}
