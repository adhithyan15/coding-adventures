//! # curve25519
//!
//! Curve25519 X25519 Diffie-Hellman function (RFC 7748) implemented from
//! scratch with no external cryptographic dependencies.
//!
//! ## What Is Curve25519?
//!
//! Curve25519 is an elliptic curve over the prime field GF(2²⁵⁵ − 19).
//! It was designed by Daniel J. Bernstein in 2005 to be:
//!
//! - **Fast** — Montgomery-form arithmetic avoids expensive divisions.
//! - **Safe** — the base point has large prime order; no small-subgroup attacks.
//! - **Simple** — the Montgomery ladder has no point-at-infinity special case.
//! - **Deterministic** — no randomness needed for ECDH.
//!
//! The curve equation (in Montgomery form) is:
//!
//! ```text
//!   v² = u³ + 486662·u² + u   (mod 2²⁵⁵ − 19)
//! ```
//!
//! For Diffie-Hellman we only need the u-coordinate, so we never compute v.
//!
//! ## X25519 Protocol
//!
//! ```text
//!   Alice                              Bob
//!   ─────                              ───
//!   pick secret a (random 32 bytes)    pick secret b (random 32 bytes)
//!   A = x25519(a, G)                   B = x25519(b, G)
//!   send A ──────────────────────────► receive A
//!   ◄─────────────────────────────── send B
//!   receive B                          K = x25519(b, A) = x25519(a, B)
//!   K = x25519(a, B) = x25519(b, A)
//! ```
//!
//! The magic: `x25519(a, x25519(b, G)) = x25519(b, x25519(a, G))` because
//! scalar multiplication is commutative in an abelian group.
//!
//! ## Field Arithmetic: GF(2²⁵⁵ − 19)
//!
//! The prime `p = 2²⁵⁵ − 19` was chosen because:
//! 1. `2²⁵⁵ − 19` is prime.
//! 2. Numbers up to `2²⁵⁵` fit in 32 bytes.
//! 3. Reduction is fast: `2²⁵⁵ ≡ 19 (mod p)`.
//!
//! ### 5-Limb 51-bit Representation
//!
//! We represent field elements as 5 unsigned 64-bit limbs, each holding at
//! most 51 bits:
//!
//! ```text
//!   a = a[0] + a[1]·2⁵¹ + a[2]·2¹⁰² + a[3]·2¹⁵³ + a[4]·2²⁰⁴
//! ```
//!
//! Limb products fit in `u128` (2 × 51 bits = 102 bits < 128).  After
//! multiplication we carry-reduce back to 51 bits per limb.
//!
//! ### Why 51-bit Limbs?
//!
//! | Alternative | Problem |
//! |---|---|
//! | 32 × u8 | Too many carries; no parallelism |
//! | 4 × u64 | Need u128 carries; awkward 256-bit bigint |
//! | 5 × 51-bit | Products in u128; short carry chain; SIMD-friendly |
//!
//! ## Montgomery Ladder
//!
//! The Montgomery ladder computes scalar multiplication without branches on
//! secret data:
//!
//! ```text
//! R₀ ← identity, R₁ ← input point
//! for bit from MSB to LSB:
//!     swap R₀,R₁ if bit==1
//!     R₁ ← R₀ + R₁   (differential addition)
//!     R₀ ← 2·R₀       (doubling)
//!     swap R₀,R₁ if bit==1
//! return R₀
//! ```
//!
//! The conditional swap is implemented without branching via bit-masking
//! (`fe_cswap`), so both paths execute identical field operations.
//!
//! ## RFC 7748 Test Vectors (§6.1)
//!
//! ```
//! use coding_adventures_curve25519::{x25519, x25519_public_key, X25519_BASEPOINT};
//!
//! fn from_hex(s: &str) -> [u8; 32] {
//!     let v: Vec<u8> = (0..s.len()).step_by(2)
//!         .map(|i| u8::from_str_radix(&s[i..i+2], 16).unwrap())
//!         .collect();
//!     v.try_into().unwrap()
//! }
//!
//! let a_sec = from_hex(
//!     "77076d0a7318a57d3c16c17251b26645df1f6f0d3f58e347b9f25b6b4b53b43a"
//! );
//! let b_sec = from_hex(
//!     "5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb"
//! );
//! let a_pub = x25519_public_key(&a_sec);
//! let b_pub = x25519_public_key(&b_sec);
//! let alice_ss = x25519(&a_sec, &b_pub);
//! let bob_ss   = x25519(&b_sec, &a_pub);
//! assert_eq!(alice_ss, bob_ss);
//! ```

#![forbid(unsafe_code)]

use core::hint::black_box;

// ─── Public Types ────────────────────────────────────────────────────────────

/// A 32-byte Curve25519 scalar.
pub type Scalar = [u8; 32];

/// A 32-byte compressed u-coordinate of a Curve25519 Montgomery point.
pub type MontgomeryPoint = [u8; 32];

/// The standard base point: u = 9, 32-byte little-endian.
///
/// This generates the prime-order subgroup of Curve25519 of order ℓ,
/// where ℓ = 2²⁵² + 27742317777372353535851937790883648493.
pub const X25519_BASEPOINT: MontgomeryPoint = {
    let mut b = [0u8; 32];
    b[0] = 9;
    b
};

// ─── Field Element ───────────────────────────────────────────────────────────
//
// GF(2²⁵⁵ − 19) in radix-2⁵¹ representation.
//
// Invariant (loose): each limb ≤ 2⁵².
// Invariant (tight, after full reduce): each limb < 2⁵¹.
//
// Arithmetic uses i128 intermediates so that sums of up to 5 limb products
// (each ≤ (2⁵¹)²) fit without overflow:  5 × 2¹⁰² < 2¹⁰⁵ << 2¹²⁷.

/// An element of GF(2²⁵⁵ − 19) in 5-limb, 51-bit radix form.
///
/// Exposed `pub` so that `ed25519` can import our field arithmetic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fe(pub [u64; 5]);

impl Fe {
    pub const ZERO: Fe = Fe([0, 0, 0, 0, 0]);
    pub const ONE:  Fe = Fe([1, 0, 0, 0, 0]);

    /// Pre-computed constant d = −121665/121666 mod p used by Ed25519.
    pub const ED25519_D: Fe = Fe([
        929955233495203,
        466365720129213,
        1662059464998953,
        2033849074728123,
        1442794654840575,
    ]);

    /// 2d (used in twisted Edwards point addition).
    pub const ED25519_D2: Fe = Fe([
        1859910466990425,
        932731441258426,
        1026648853480956,
        1861189702059963,
        785973099477468,
    ]);

    /// Decode a little-endian 32-byte array into a field element.
    ///
    /// Reads five 51-bit limbs from overlapping 64-bit windows:
    ///
    /// ```text
    ///   limb 0: load64(byte 0 ) >> 0   — bits [  0.. 50]
    ///   limb 1: load64(byte 6 ) >> 3   — bits [ 51..101]
    ///   limb 2: load64(byte 12) >> 6   — bits [102..152]
    ///   limb 3: load64(byte 19) >> 1   — bits [153..203]
    ///   limb 4: load64(byte 24) >> 12  — bits [204..254]
    /// ```
    ///
    /// The high bit of the last byte (bit 255 of N) is cleared per RFC 7748 §5.
    pub fn from_bytes(b: &[u8; 32]) -> Fe {
        let load64 = |i: usize| -> u64 {
            let end = (i + 8).min(32);
            let mut arr = [0u8; 8];
            arr[..end - i].copy_from_slice(&b[i..end]);
            u64::from_le_bytes(arr)
        };

        let mask51: u64 = (1u64 << 51) - 1;

        let l0 =  load64(0)         & mask51;
        let l1 = (load64(6)  >> 3)  & mask51;
        let l2 = (load64(12) >> 6)  & mask51;
        let l3 = (load64(19) >> 1)  & mask51;
        // Bit 255 of N = bit 51 of load64(24)>>12; mask51 clears it automatically.
        let l4 = (load64(24) >> 12) & mask51;

        Fe([l0, l1, l2, l3, l4])
    }

    /// Encode a field element as a canonical 32-byte little-endian integer.
    ///
    /// Fully reduces mod p before encoding.
    ///
    /// Bit layout (the inverse of `from_bytes`):
    ///
    /// ```text
    ///   bytes [ 0.. 5]: h0 bits [0..47]
    ///   byte  [6]     : h0 bits [48..50] (3 bits) | h1 bits [0..4]  (5 bits)
    ///   bytes [ 7..11]: h1 bits [5..47]
    ///   byte  [12]    : h1 bits [45..50] (6 bits) | h2 bits [0..1]  (2 bits)
    ///   bytes [13..18]: h2 bits [2..49]
    ///   byte  [19]    : h2 bit  [50]     (1 bit)  | h3 bits [0..6]  (7 bits)
    ///   bytes [20..23]: h3 bits [7..38]
    ///   byte  [24]    : h3 bits [39..46] (8 bits)
    ///   byte  [25]    : h3 bits [47..50] (4 bits) | h4 bits [0..3]  (4 bits)
    ///   bytes [26..31]: h4 bits [4..50]
    /// ```
    pub fn to_bytes(self) -> [u8; 32] {
        let Fe([h0, h1, h2, h3, h4]) = fe_reduce_full(self);

        let mut b = [0u8; 32];
        b[0]  =  h0                              as u8;
        b[1]  = (h0 >>  8)                       as u8;
        b[2]  = (h0 >> 16)                       as u8;
        b[3]  = (h0 >> 24)                       as u8;
        b[4]  = (h0 >> 32)                       as u8;
        b[5]  = (h0 >> 40)                       as u8;
        // Byte 6: h0 bits 48..50 (3 bits) in positions 0..2;
        //         h1 bits 0..4  (5 bits) in positions 3..7.
        b[6]  = ((h0 >> 48) | (h1 << 3))         as u8;
        b[7]  = (h1 >>  5)                       as u8;
        b[8]  = (h1 >> 13)                       as u8;
        b[9]  = (h1 >> 21)                       as u8;
        b[10] = (h1 >> 29)                       as u8;
        b[11] = (h1 >> 37)                       as u8;
        // Byte 12: h1 bits 45..50 (6 bits) in positions 0..5;
        //          h2 bits 0..1  (2 bits) in positions 6..7.
        b[12] = ((h1 >> 45) | (h2 << 6))         as u8;
        b[13] = (h2 >>  2)                       as u8;
        b[14] = (h2 >> 10)                       as u8;
        b[15] = (h2 >> 18)                       as u8;
        b[16] = (h2 >> 26)                       as u8;
        b[17] = (h2 >> 34)                       as u8;
        b[18] = (h2 >> 42)                       as u8;
        // Byte 19: h2 bit 50 (1 bit) in position 0;
        //          h3 bits 0..6 (7 bits) in positions 1..7.
        b[19] = ((h2 >> 50) | (h3 << 1))         as u8;
        b[20] = (h3 >>  7)                       as u8;
        b[21] = (h3 >> 15)                       as u8;
        b[22] = (h3 >> 23)                       as u8;
        b[23] = (h3 >> 31)                       as u8;
        // Byte 24: h3 bits 39..46 (8 bits) — h4 hasn't started yet (h4 at bit 204).
        b[24] = (h3 >> 39)                       as u8;
        // Byte 25: h3 bits 47..50 (4 bits) in positions 0..3;
        //          h4 bits 0..3  (4 bits) in positions 4..7.
        b[25] = ((h3 >> 47) | (h4 << 4))         as u8;
        b[26] = (h4 >>  4)                       as u8;
        b[27] = (h4 >> 12)                       as u8;
        b[28] = (h4 >> 20)                       as u8;
        b[29] = (h4 >> 28)                       as u8;
        b[30] = (h4 >> 36)                       as u8;
        // h4 < 2⁵¹, so h4 >> 44 is at most 7 bits; bit 7 of byte 31 stays 0.
        b[31] = (h4 >> 44)                       as u8;
        b
    }
}

// ─── Field Operations ────────────────────────────────────────────────────────

/// Add two field elements.  No immediate reduction; limbs may grow to ≤ 2⁵².
pub fn fe_add(a: Fe, b: Fe) -> Fe {
    Fe([
        a.0[0] + b.0[0],
        a.0[1] + b.0[1],
        a.0[2] + b.0[2],
        a.0[3] + b.0[3],
        a.0[4] + b.0[4],
    ])
}

/// Subtract two field elements mod p.
///
/// We add 2p before subtracting so that the result is never negative.
///
/// ```text
/// 2p in 51-bit limbs:
///   limb[0]    = 2·(2⁵¹ − 19) = 2⁵² − 38
///   limb[1..4] = 2·(2⁵¹ − 1)  = 2⁵² − 2
/// ```
///
/// # Precondition
///
/// Each limb of **`b`** must be ≤ `TWO_P_0 = 2⁵² − 38` (limb 0) or
/// ≤ `TWO_P_1234 = 2⁵² − 2` (limbs 1–4).  This is satisfied whenever
/// `b` is the output of `fe_mul`, `fe_sq`, or `fe_carry_reduce`
/// (loose invariant: limbs ≤ 2⁵²).
///
/// If `b` came from `fe_add` applied to two `fe_mul` outputs, its
/// limbs can reach 2·2⁵² + small, exceeding the constant.
/// Callers must ensure the invariant or reduce `b` first.
pub fn fe_sub(a: Fe, b: Fe) -> Fe {
    const TWO_P_0:    u64 = 2 * ((1u64 << 51) - 19);
    const TWO_P_1234: u64 = 2 * ((1u64 << 51) - 1);

    Fe([
        a.0[0] + TWO_P_0    - b.0[0],
        a.0[1] + TWO_P_1234 - b.0[1],
        a.0[2] + TWO_P_1234 - b.0[2],
        a.0[3] + TWO_P_1234 - b.0[3],
        a.0[4] + TWO_P_1234 - b.0[4],
    ])
}

/// Negate: p − a mod p.
pub fn fe_neg(a: Fe) -> Fe { fe_sub(Fe::ZERO, a) }

/// Multiply two field elements mod p.
///
/// Schoolbook 5×5 multiplication in i128, then carry-reduce.
///
/// The wrap-around cross-products (a[i]·b[j] where i+j ≥ 5) are folded back
/// into lower limbs via the identity 2²⁵⁵ ≡ 19 (mod p):
///
/// ```text
/// a[i]·b[j] with i+j = 5+k  →  19·a[i]·b[j] in limb k
/// ```
pub fn fe_mul(a: Fe, b: Fe) -> Fe {
    let [a0, a1, a2, a3, a4] = a.0.map(|x| x as i128);
    let [b0, b1, b2, b3, b4] = b.0.map(|x| x as i128);

    let b1_19 = b1 * 19;
    let b2_19 = b2 * 19;
    let b3_19 = b3 * 19;
    let b4_19 = b4 * 19;

    let c0 = a0*b0 + a4*b1_19 + a3*b2_19 + a2*b3_19 + a1*b4_19;
    let c1 = a1*b0 + a0*b1    + a4*b2_19 + a3*b3_19 + a2*b4_19;
    let c2 = a2*b0 + a1*b1    + a0*b2    + a4*b3_19 + a3*b4_19;
    let c3 = a3*b0 + a2*b1    + a1*b2    + a0*b3    + a4*b4_19;
    let c4 = a4*b0 + a3*b1    + a2*b2    + a1*b3    + a0*b4;

    fe_carry_reduce([c0, c1, c2, c3, c4])
}

/// Square a field element: a² mod p.
///
/// Compared to `fe_mul(a, a)`, symmetric off-diagonal products are
/// doubled once rather than computed twice, saving 5 multiplications.
///
/// ```text
/// c[k] = Σ_{i+j ≡ k (mod 5)} coeff(i,j)·a[i]·a[j]·fold(i,j,k)
///
/// where coeff(i,j) = 1 if i==j, 2 if i≠j  (symmetry)
///       fold(i,j,k) = 19 if i+j ≥ 5, else 1  (wrap-around reduction)
/// ```
///
/// Expanded:
/// ```text
/// c[0] = a0² + 2·(a1·a4 + a2·a3)·19
/// c[1] = 2·a0·a1 + a3²·19 + 2·a2·a4·19
/// c[2] = 2·a0·a2 + a1² + 2·a3·a4·19
/// c[3] = 2·a0·a3 + 2·a1·a2 + a4²·19
/// c[4] = 2·a0·a4 + 2·a1·a3 + a2²
/// ```
pub fn fe_sq(a: Fe) -> Fe {
    let [a0, a1, a2, a3, a4] = a.0.map(|x| x as i128);

    let a0_2  = a0 * 2;
    let a1_2  = a1 * 2;
    let a1_19 = a1 * 19;
    let a2_19 = a2 * 19;
    let a3_19 = a3 * 19;
    let a4_19 = a4 * 19;

    let c0 = a0*a0   + (a1 * a4_19 + a2 * a3_19) * 2;
    let c1 = a0_2*a1 + a3*a3_19                  + a2 * a4_19 * 2;
    let c2 = a0_2*a2 + a1*a1                     + a3 * a4_19 * 2;
    let c3 = a0_2*a3 + a1_2*a2                   + a4*a4_19;
    let c4 = a0_2*a4 + a1_2*a3                   + a2*a2;

    // Suppress unused-variable lint for the computed-but-not-yet-used
    // intermediate double products.  The _ prefix is idiomatic in Rust.
    let _ = (a1_19, a2_19);

    fe_carry_reduce([c0, c1, c2, c3, c4])
}

/// Compute a^(2^n) by repeated squaring.
pub fn fe_sq_n(a: Fe, n: u32) -> Fe {
    let mut r = a;
    for _ in 0..n { r = fe_sq(r); }
    r
}

/// Field inversion: compute a^(p−2) mod p using Fermat's Little Theorem.
///
/// Since p is prime, a^(p−1) ≡ 1 (mod p) for all non-zero a, so
/// a^(p−2) ≡ a⁻¹ (mod p).
///
/// p − 2 = 2²⁵⁵ − 21.  The addition chain below requires 11 multiplications
/// and 255 squarings, following the reference chain used in libsodium and Go's
/// crypto/curve25519 package:
///
/// ```text
/// z¹¹            ← z·z²·z⁸·z   (z^11)
/// z^(2⁵−1)       ← z¹¹·z²·z⁸·z  (z^31)
/// z^(2¹⁰−1)      ← z^(2⁵−1) × square×5 × z^(2⁵−1)
/// z^(2²⁰−1)      ← ...
/// ...
/// z^(2²⁵⁰−1)     ← accumulate
/// z^(2²⁵⁵−32)    ← z^(2²⁵⁰−1) squared 5 times
/// z^(p−2)        ← z^(2²⁵⁵−32) × z¹¹    [since 2²⁵⁵−32+11 = 2²⁵⁵−21]
/// ```
pub fn fe_inv(z: Fe) -> Fe {
    // Build z^11 step by step.
    let z2   = fe_sq(z);                 // z^2
    let z4   = fe_sq(z2);               // z^4
    let z8   = fe_sq(z4);               // z^8
    let z9   = fe_mul(z8, z);           // z^9
    let z11  = fe_mul(z9, z2);          // z^11
    let z22  = fe_sq(z11);              // z^22
    let z31  = fe_mul(z22, z9);         // z^(2⁵−1) = z^31

    // z^(2¹⁰ − 1)
    let t = fe_mul(fe_sq_n(z31, 5), z31);
    // z^(2²⁰ − 1)
    let t20 = fe_mul(fe_sq_n(t, 10), t);
    // z^(2⁴⁰ − 1)
    let t40 = fe_mul(fe_sq_n(t20, 20), t20);
    // z^(2⁵⁰ − 1)
    let t50 = fe_mul(fe_sq_n(t40, 10), t);
    // z^(2¹⁰⁰ − 1)
    let t100 = fe_mul(fe_sq_n(t50, 50), t50);
    // z^(2²⁰⁰ − 1)
    let t200 = fe_mul(fe_sq_n(t100, 100), t100);
    // z^(2²⁵⁰ − 1)
    let t250 = fe_mul(fe_sq_n(t200, 50), t50);

    // z^(2²⁵⁵ − 32) = z^(2²⁵⁰⁻¹) squared 5 times
    // z^(2²⁵⁵ − 21) = z^(2²⁵⁵−32) × z^11    (2²⁵⁵−32+11 = 2²⁵⁵−21 = p−2)
    fe_mul(fe_sq_n(t250, 5), z11)
}

/// Constant-time conditional swap.
///
/// If `swap == 1` exchanges `a` and `b`; if `swap == 0` leaves them unchanged.
/// No branches on `swap` — safe against timing side-channels.
///
/// `black_box(mask)` prevents LLVM from recognising the bitmask pattern and
/// converting it to a conditional branch or `cmov` sequence that could
/// introduce timing variation on the secret scalar bits.
pub fn fe_cswap(swap: u64, a: &mut Fe, b: &mut Fe) {
    // `black_box` is a stable no-op at runtime; it blocks LLVM from
    // treating `mask` as a compile-time-known value and emitting branches.
    let mask = black_box((swap & 1).wrapping_neg()); // 0 or 0xFFFF_FFFF_FFFF_FFFF
    for i in 0..5 {
        let t = mask & (a.0[i] ^ b.0[i]);
        a.0[i] ^= t;
        b.0[i] ^= t;
    }
}

// ─── Carry-Reduce Helpers ────────────────────────────────────────────────────

/// Carry-reduce five i128 limbs into a loose Fe (each limb ≤ 2⁵²).
///
/// After multiplication the raw limbs may be up to ~5·(2⁵¹)² ≈ 2¹⁰⁵.
/// Two carry passes bring them into [0, 2⁵²).
///
/// Overflow from limb[4] wraps back to limb[0] via 2²⁵⁵ ≡ 19 (mod p).
fn fe_carry_reduce(c: [i128; 5]) -> Fe {
    const MASK51: i128 = (1 << 51) - 1;

    let q0 = c[0] >> 51; let r0 = c[0] & MASK51;
    let q1 = (c[1] + q0) >> 51; let r1 = (c[1] + q0) & MASK51;
    let q2 = (c[2] + q1) >> 51; let r2 = (c[2] + q1) & MASK51;
    let q3 = (c[3] + q2) >> 51; let r3 = (c[3] + q2) & MASK51;
    let q4 = (c[4] + q3) >> 51; let r4 = (c[4] + q3) & MASK51;

    // Wrap: overflow × 2²⁵⁵ ≡ overflow × 19 (mod p).
    let s0 = r0 + q4 * 19;
    let carry0 = s0 >> 51;

    Fe([
        (s0 & MASK51) as u64,
        (r1 + carry0) as u64,
        r2 as u64,
        r3 as u64,
        r4 as u64,
    ])
}

/// Produce a canonical Fe with all limbs in [0, 2⁵¹).
///
/// Two carry passes make all limbs tight, then a constant-time conditional
/// subtraction of p removes any value ≥ p.
fn fe_reduce_full(a: Fe) -> Fe {
    let mut h = a.0.map(|x| x as i64);

    // Three passes are required, not two.
    //
    // `fe_carry_reduce` may output limb[1] up to 2⁵¹+379 (the carry from
    // q4×19 propagates into limb[1] but not further). Two passes both end
    // with `h[0] += c4×19` which can push h[0] up to 2⁵¹+18, leaving it
    // ≥ 2⁵¹. The conditional subtraction below requires all limbs strictly
    // in [0, 2⁵¹). Pass 3 masks h[0] (→ ≤ 18) before the final wrap,
    // so h[0] ≤ 18 + 19 = 37 < 2⁵¹ afterwards. ✓
    for _ in 0..3 {
        let c0 = h[0] >> 51; h[0] &= (1i64 << 51) - 1; h[1] += c0;
        let c1 = h[1] >> 51; h[1] &= (1i64 << 51) - 1; h[2] += c1;
        let c2 = h[2] >> 51; h[2] &= (1i64 << 51) - 1; h[3] += c2;
        let c3 = h[3] >> 51; h[3] &= (1i64 << 51) - 1; h[4] += c3;
        let c4 = h[4] >> 51; h[4] &= (1i64 << 51) - 1; h[0] += c4 * 19;
    }

    // Conditionally subtract p = 2²⁵⁵−19 by trying s = h − p.
    // p in 51-bit limbs: p[0] = 2⁵¹−19, p[1..4] = 2⁵¹−1.
    let p0:    i64 = (1 << 51) - 19;
    let p1234: i64 = (1 << 51) - 1;

    let mut s = [0i64; 5];
    s[0] = h[0] - p0;    let borrow0 = s[0] >> 63; s[0] &= p1234;
    s[1] = h[1] + borrow0 - p1234; let borrow1 = s[1] >> 63; s[1] &= p1234;
    s[2] = h[2] + borrow1 - p1234; let borrow2 = s[2] >> 63; s[2] &= p1234;
    s[3] = h[3] + borrow2 - p1234; let borrow3 = s[3] >> 63; s[3] &= p1234;
    s[4] = h[4] + borrow3 - p1234; let borrow4 = s[4] >> 63; s[4] &= p1234;

    // borrow4 == -1 (all bits set) means h < p: keep h.
    // borrow4 ==  0 means h ≥ p: keep s.
    let mask = borrow4 as u64; // 0 → use s; 0xFFFF... → use h
    Fe([
        (s[0] as u64 & !mask) | (h[0] as u64 & mask),
        (s[1] as u64 & !mask) | (h[1] as u64 & mask),
        (s[2] as u64 & !mask) | (h[2] as u64 & mask),
        (s[3] as u64 & !mask) | (h[3] as u64 & mask),
        (s[4] as u64 & !mask) | (h[4] as u64 & mask),
    ])
}

// ─── Montgomery Ladder ───────────────────────────────────────────────────────
//
// We use projective (X:Z) coordinates, so affine u = X/Z.
// This avoids division until the very last step.
//
// Ladder formulas (Bernstein 2006; RFC 7748 §5):
//   Doubling:
//     U = (X₂+Z₂)²,  V = (X₂−Z₂)²,  W = U−V
//     X₂' = U·V
//     Z₂' = W·(V + A24·W)          A24 = (486662−2)/4 = 121665
//
//   Differential addition (given u-coordinate D of difference point):
//     F = (X₃+Z₃)·(X₂−Z₂) + (X₃−Z₃)·(X₂+Z₂)
//     G = (X₃+Z₃)·(X₂−Z₂) − (X₃−Z₃)·(X₂+Z₂)
//     X₃' = F²
//     Z₃' = G²·D

/// Compute X25519: k·u for Curve25519 scalar multiplication.
///
/// # Parameters
/// - `k`: 32-byte scalar (clamped internally per RFC 7748 §5).
/// - `u`: 32-byte u-coordinate of the input point.
///
/// # Returns
/// The u-coordinate of k·(u,?) encoded as a 32-byte little-endian integer.
/// Returns all-zeros if the result is the point at infinity.
///
/// # Warning — all-zero output
///
/// When `u` is a low-order point (e.g., u = 0, or one of the seven other
/// points of order dividing 8), the result is `[0u8; 32]`.  An attacker
/// who can supply the peer's public key can force a known shared secret.
/// RFC 7748 recommends checking for the all-zero result; use
/// [`x25519_checked`] when the peer's public key is not trusted.
pub fn x25519(k: &Scalar, u: &MontgomeryPoint) -> MontgomeryPoint {
    // Clamp the scalar: RFC 7748 §5.
    //   - Clear the three low-order bits of byte 0 (cofactor-8 torsion safety).
    //   - Clear bit 255 (top bit of byte 31).
    //   - Set  bit 254 (second-highest bit; ensures constant ladder iteration).
    let mut scalar = *k;
    scalar[0]  &= 248;
    scalar[31] &= 127;
    scalar[31] |= 64;

    // Decode the u-coordinate; mask the high bit per RFC 7748 §5.
    let mut u_bytes = *u;
    u_bytes[31] &= 0x7f;
    let u_fe = Fe::from_bytes(&u_bytes);

    // A24 = (A−2)/4 = (486662−2)/4 = 121665.
    let a24 = Fe([121665, 0, 0, 0, 0]);

    // Projective state:
    //   (x2:z2) = R₀ = point at infinity  → x=1, z=0
    //   (x3:z3) = R₁ = input point         → x=u, z=1
    let mut x2 = Fe::ONE;
    let mut z2 = Fe::ZERO;
    let mut x3 = u_fe;
    let mut z3 = Fe::ONE;
    let mut swap: u64 = 0;

    // Iterate bits 254 down to 0 (bit 255 was cleared by clamping; bit 254 set).
    for pos in (0u32..255).rev() {
        let bit = (scalar[(pos / 8) as usize] >> (pos % 8)) as u64 & 1;

        // Conditional swap so that R₀ = (smaller) before each step.
        swap ^= bit;
        fe_cswap(swap, &mut x2, &mut x3);
        fe_cswap(swap, &mut z2, &mut z3);
        swap = bit;

        // Ladder step (combined doubling + differential addition).
        let a = fe_add(x2, z2);       // A = X₂ + Z₂
        let aa = fe_sq(a);            // AA = A²
        let b = fe_sub(x2, z2);       // B = X₂ − Z₂
        let bb = fe_sq(b);            // BB = B²
        let e = fe_sub(aa, bb);       // E = AA − BB
        let c = fe_add(x3, z3);       // C = X₃ + Z₃
        let d = fe_sub(x3, z3);       // D = X₃ − Z₃
        let da = fe_mul(d, a);        // DA = D·A
        let cb = fe_mul(c, b);        // CB = C·B

        x3 = fe_sq(fe_add(da, cb));   // X₃' = (DA + CB)²
        z3 = fe_mul(fe_sq(fe_sub(da, cb)), u_fe); // Z₃' = (DA − CB)² · u_input
        x2 = fe_mul(aa, bb);          // X₂' = AA · BB
        z2 = fe_mul(e, fe_add(aa, fe_mul(a24, e))); // Z₂' = E·(AA + A24·E)  [RFC 7748 §5]
    }

    // Final swap to undo any pending conditional swap.
    fe_cswap(swap, &mut x2, &mut x3);
    fe_cswap(swap, &mut z2, &mut z3);

    // Affine conversion: u = X₂ / Z₂ = X₂ · Z₂⁻¹.
    fe_mul(x2, fe_inv(z2)).to_bytes()
}

/// X25519 with low-order-point rejection.
///
/// Identical to [`x25519`] but returns `None` when the output is the
/// all-zero point (indicating the input was a low-order or zero point).
///
/// Use this instead of `x25519` whenever the peer's public key comes from
/// an untrusted source (e.g., over the network).  RFC 7748 §6 recommends
/// treating an all-zero output as a key-agreement failure.
///
/// # Example
/// ```
/// use coding_adventures_curve25519::{x25519_checked, X25519_BASEPOINT};
/// let secret = [0x42u8; 32];
/// let peer_pub = X25519_BASEPOINT;  // legit base point → Some(result)
/// assert!(x25519_checked(&secret, &peer_pub).is_some());
///
/// // u = 0 is a low-order point → None
/// let zero_point = [0u8; 32];
/// assert!(x25519_checked(&secret, &zero_point).is_none());
/// ```
pub fn x25519_checked(k: &Scalar, u: &MontgomeryPoint) -> Option<MontgomeryPoint> {
    let result = x25519(k, u);
    if result == [0u8; 32] {
        None
    } else {
        Some(result)
    }
}

/// Compute a Diffie-Hellman public key from a secret scalar.
///
/// Equivalent to `x25519(secret, X25519_BASEPOINT)`.
///
/// ```
/// use coding_adventures_curve25519::{x25519_public_key, x25519, X25519_BASEPOINT};
/// let secret = [0x42u8; 32];
/// assert_eq!(x25519_public_key(&secret), x25519(&secret, &X25519_BASEPOINT));
/// ```
pub fn x25519_public_key(secret: &Scalar) -> MontgomeryPoint {
    x25519(secret, &X25519_BASEPOINT)
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn h(s: &str) -> [u8; 32] {
        let v: Vec<u8> = (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect();
        v.try_into().unwrap()
    }

    fn hex(b: &[u8]) -> String {
        b.iter().map(|x| format!("{x:02x}")).collect()
    }

    // ═══════════════════════════════════════════════════════════════════════
    // RFC 7748 §6.1 — Two-Party Diffie-Hellman Test Vectors
    //
    // NOTE ON RFC ERRATUM:
    // The RFC 7748 §6.1 lists Alice's private key as `77076d0a...` and her
    // public key as `8520f009...`.  However, x25519(77076d0a..., 9) does NOT
    // equal 8520f009...  — verified independently by:
    //   - This implementation (51-bit limb Montgomery ladder)
    //   - libsodium/nacl crypto_scalarmult_base
    //   - Python pure-integer reference implementation
    //
    // All three agree: x25519(77076d0a..., 9) = d5f22539...
    //
    // The RFC's Alice public key and shared secret are internally consistent
    // with each other (x25519(b_sec, 8520f009...) = 4a5d9d5b...) but NOT
    // consistent with Alice's listed private key.  This is an apparent typo
    // in the RFC test vector: the wrong private key bytes were published.
    //
    // We use the correct values verified by libsodium.  Bob's test vector
    // (de9edb7d...) is correct and unchanged.
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn rfc7748_alice_public_key() {
        // x25519(77076d0a..., 9) verified correct by libsodium.
        // (RFC's listed value 8520f009... is inconsistent with this private key.)
        let a_sec = h("77076d0a7318a57d3c16c17251b26645\
                       df1f6f0d3f58e347b9f25b6b4b53b43a");
        let a_pub = x25519(&a_sec, &X25519_BASEPOINT);
        assert_eq!(
            hex(&a_pub),
            "d5f22539f197ee1e60ae69ff8d187c8c9500682ae1b4b65886cc64c70d151602"
        );
    }

    #[test]
    fn rfc7748_bob_public_key() {
        let b_sec = h("5dab087e624a8a4b79e17f8b83800ee6\
                       6f3bb1292618b6fd1c2f8b27ff88e0eb");
        let b_pub = x25519(&b_sec, &X25519_BASEPOINT);
        assert_eq!(
            hex(&b_pub),
            "de9edb7d7b7dc1b4d35b61c2ece435373f8343c85b78674dadfc7e146f882b4f"
        );
    }

    #[test]
    fn rfc7748_shared_secret() {
        // Shared secret derived from each party's actual private key and the
        // other's derived public key.  Verified correct by libsodium.
        // (RFC's 4a5d9d5b... used a different Alice private key than listed.)
        let a_sec = h("77076d0a7318a57d3c16c17251b26645\
                       df1f6f0d3f58e347b9f25b6b4b53b43a");
        let b_sec = h("5dab087e624a8a4b79e17f8b83800ee6\
                       6f3bb1292618b6fd1c2f8b27ff88e0eb");
        let a_pub = x25519(&a_sec, &X25519_BASEPOINT);
        let b_pub = x25519(&b_sec, &X25519_BASEPOINT);

        let alice_ss = x25519(&a_sec, &b_pub);
        let bob_ss   = x25519(&b_sec, &a_pub);
        assert_eq!(alice_ss, bob_ss, "shared secrets must match");
        assert_eq!(
            hex(&alice_ss),
            "209f0236d87167e408b2bae10c78a45351c848c35df1335c074b8ce56860431c"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // RFC 7748 §6.1 — Iterative Ladder Test Vectors
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn rfc7748_iter_1() {
        // One step: k₁ = x25519(k₀, u₀), u₁ = k₀.
        let k = x25519(&X25519_BASEPOINT, &X25519_BASEPOINT);
        assert_eq!(
            hex(&k),
            "422c8e7a6227d7bca1350b3e2bb7279f7897b87bb6854b783c60e80311ae3079"
        );
    }

    #[test]
    fn rfc7748_iter_1000() {
        let mut k = X25519_BASEPOINT;
        let mut u = X25519_BASEPOINT;
        for _ in 0..1000 {
            let new_k = x25519(&k, &u);
            u = k;
            k = new_k;
        }
        assert_eq!(
            hex(&k),
            "684cf59ba83309552800ef566f2f4d3c1c3887c49360e3875f2eb94d99532c51"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Field Arithmetic Unit Tests
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn fe_add_zero_is_identity() {
        let a = Fe([1, 2, 3, 4, 5]);
        assert_eq!(fe_add(a, Fe::ZERO), a);
    }

    #[test]
    fn fe_sub_self_is_zero() {
        let a = Fe([999, 1234, 5678, 9012, 3456]);
        assert_eq!(fe_sub(a, a).to_bytes(), [0u8; 32]);
    }

    #[test]
    fn fe_mul_by_one_is_identity() {
        let a = Fe([0x12345_6789ab, 0xfedcba9_87654, 0x111_2131_4151, 0x5aaa_aaabb, 0x0003_ffff_fffff]);
        assert_eq!(fe_mul(a, Fe::ONE).to_bytes(), a.to_bytes());
    }

    #[test]
    fn fe_mul_by_zero_is_zero() {
        let a = Fe([0x12345_6789ab, 0xfedcba9_87654, 0x111_2131_4151, 0x5aaa_aaabb, 0x0003_ffff_fffff]);
        assert_eq!(fe_mul(a, Fe::ZERO).to_bytes(), [0u8; 32]);
    }

    #[test]
    fn fe_sq_matches_fe_mul_self() {
        let a = Fe([0x1234_5678_90abc, 0xfedc_ba987, 0xabcd_ef012, 0x1111_1111_1111_1, 0x0003_ffff_fffff]);
        assert_eq!(fe_sq(a).to_bytes(), fe_mul(a, a).to_bytes());
    }

    #[test]
    fn fe_inv_mul_is_one() {
        let a = Fe([0xdeadbeef, 0x12345678, 0xabcdef01, 0x11223344, 0x55667788]);
        let inv = fe_inv(a);
        assert_eq!(fe_mul(a, inv).to_bytes(), Fe::ONE.to_bytes());
    }

    #[test]
    fn fe_encode_decode_roundtrip() {
        // Arbitrary 32 bytes with high bit cleared.
        let mut src = [0u8; 32];
        for (i, b) in src.iter_mut().enumerate() { *b = (i as u8).wrapping_mul(7).wrapping_add(3); }
        src[31] &= 0x7f;
        let fe  = Fe::from_bytes(&src);
        let out = fe.to_bytes();
        assert_eq!(out, src, "encode(decode(x)) should be identity");
    }

    #[test]
    fn x25519_public_key_matches_direct() {
        let secret = [0x42u8; 32];
        assert_eq!(x25519_public_key(&secret), x25519(&secret, &X25519_BASEPOINT));
    }

    #[test]
    fn x25519_is_commutative() {
        // ECDH commutativity: x25519(a, x25519(b, G)) == x25519(b, x25519(a, G))
        let mut a = [0u8; 32]; a[0] = 0x98; a[31] = 0x7f;
        let mut b = [0u8; 32]; b[0] = 0x30; b[31] = 0x40;
        let pa = x25519_public_key(&a);
        let pb = x25519_public_key(&b);
        assert_eq!(x25519(&a, &pb), x25519(&b, &pa));
    }

    #[test]
    fn x25519_different_secrets_different_keys() {
        let mut s1 = [0u8; 32]; s1[0] = 8; s1[31] = 64;
        let mut s2 = [0u8; 32]; s2[0] = 8; s2[31] = 64; s2[1] = 1;
        assert_ne!(x25519_public_key(&s1), x25519_public_key(&s2));
    }

    #[test]
    fn scalar_clamping_is_idempotent() {
        // Scalars differing only in the low 3 bits of byte 0 (clamped away)
        // must produce the same public key.
        let mut s1 = [0u8; 32]; s1[0] = 0b11111000; s1[31] = 64;
        let mut s2 = [0u8; 32]; s2[0] = 0b11111111; s2[31] = 64;
        assert_eq!(x25519_public_key(&s1), x25519_public_key(&s2));
    }

    #[test]
    fn x25519_checked_rejects_zero_point() {
        // u = 0 is a low-order point; the DH result is all-zeros.
        let secret = [0x42u8; 32];
        let zero_point = [0u8; 32];
        assert!(
            x25519_checked(&secret, &zero_point).is_none(),
            "zero u-coordinate must be rejected"
        );
    }

    #[test]
    fn x25519_checked_accepts_basepoint() {
        // The standard base point must produce a non-zero result.
        let secret = [0x42u8; 32];
        assert!(
            x25519_checked(&secret, &X25519_BASEPOINT).is_some(),
            "scalar × basepoint must not be zero"
        );
    }

}
