//! Ed25519 digital signatures (RFC 8032) — implemented from scratch.
//!
//! # What Is Ed25519?
//!
//! Ed25519 is an elliptic curve digital signature algorithm (EdDSA) designed by
//! Daniel J. Bernstein et al. It uses the twisted Edwards curve:
//!
//! ```text
//!     -x² + y² = 1 + d·x²·y²    (mod p, where p = 2²⁵⁵ - 19)
//! ```
//!
//! Ed25519 provides:
//!   - Fast signing and verification
//!   - Compact 32-byte keys and 64-byte signatures
//!   - 128-bit security level
//!   - Deterministic signing (no random nonce needed)
//!
//! # Field Representation
//!
//! We represent elements of GF(2²⁵⁵ - 19) as five 64-bit limbs in radix 2⁵¹:
//!
//! ```text
//!     x = x[0] + x[1]·2⁵¹ + x[2]·2¹⁰² + x[3]·2¹⁵³ + x[4]·2²⁰⁴
//! ```
//!
//! Each limb fits in 51 bits, products fit in u128. This avoids external
//! big-integer libraries.
//!
//! # Dependency
//!
//! Uses our from-scratch SHA-512 implementation for all hashing.

use coding_adventures_sha512::sum512;

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 1: CONSTANTS
// ═══════════════════════════════════════════════════════════════════════════════

/// Mask for the low 51 bits.
const MASK51: u64 = (1u64 << 51) - 1;

/// The group order L = 2²⁵² + 27742317777372353535851937790883648493.
/// Encoded as 32 bytes, little-endian.
const L_BYTES: [u8; 32] = [
    0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58,
    0xd6, 0x9c, 0xf7, 0xa2, 0xde, 0xf9, 0xde, 0x14,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10,
];

/// Base point y-coordinate bytes (LE): By = 4/5 mod p.
const BY_BYTES: [u8; 32] = [
    0x58, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66,
    0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66,
    0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66,
    0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66,
];

/// Base point x-coordinate bytes (LE).
const BX_BYTES: [u8; 32] = [
    0x1a, 0xd5, 0x25, 0x8f, 0x60, 0x2d, 0x56, 0xc9,
    0xb2, 0xa7, 0x25, 0x95, 0x60, 0xc7, 0x2c, 0x69,
    0x5c, 0xdc, 0xd6, 0xfd, 0x31, 0xe2, 0xa4, 0xc0,
    0xfe, 0x53, 0x6e, 0xcd, 0xd3, 0x36, 0x69, 0x21,
];

/// d constant bytes (LE): d = -121665/121666 mod p.
const D_BYTES: [u8; 32] = [
    0xa3, 0x78, 0x59, 0x13, 0xca, 0x4d, 0xeb, 0x75,
    0xab, 0xd8, 0x41, 0x41, 0x4d, 0x0a, 0x70, 0x00,
    0x98, 0xe8, 0x79, 0x77, 0x79, 0x40, 0xc7, 0x8c,
    0x73, 0xfe, 0x6f, 0x2b, 0xee, 0x6c, 0x03, 0x52,
];

/// √(-1) mod p, bytes (LE).
const SQRT_M1_BYTES: [u8; 32] = [
    0xb0, 0xa0, 0x0e, 0x4a, 0x27, 0x1b, 0xee, 0xc4,
    0x78, 0xe4, 0x2f, 0xad, 0x06, 0x18, 0x43, 0x2f,
    0xa7, 0xd7, 0xfb, 0x3d, 0x99, 0x00, 0x4d, 0x2b,
    0x0b, 0xdf, 0xc1, 0x4f, 0x80, 0x24, 0x83, 0x2b,
];

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 2: FIELD ELEMENT TYPE AND ARITHMETIC
// ═══════════════════════════════════════════════════════════════════════════════

/// A field element in GF(2²⁵⁵ - 19), stored as 5 limbs in radix 2⁵¹.
#[derive(Clone, Copy, Debug)]
struct Fe([u64; 5]);

impl Fe {
    const ZERO: Fe = Fe([0, 0, 0, 0, 0]);
    const ONE: Fe = Fe([1, 0, 0, 0, 0]);

    /// Decode from 32 little-endian bytes. Masks off bit 255.
    fn from_bytes(b: &[u8; 32]) -> Fe {
        let mut raw = *b;
        raw[31] &= 127;

        // Read 8-byte little-endian chunks and extract 51-bit limbs.
        // load8 reads 8 bytes starting at offset (padded with 0 if near end).
        let load8 = |off: usize| -> u64 {
            let mut buf = [0u8; 8];
            let end = (off + 8).min(32);
            buf[..end - off].copy_from_slice(&raw[off..end]);
            u64::from_le_bytes(buf)
        };

        // Limb 0: bits 0..50  (byte 0, bit 0)
        let l0 = load8(0) & MASK51;
        // Limb 1: bits 51..101 (byte 6, bit 3)
        let l1 = (load8(6) >> 3) & MASK51;
        // Limb 2: bits 102..152 (byte 12, bit 6)
        let l2 = (load8(12) >> 6) & MASK51;
        // Limb 3: bits 153..203 (byte 19, bit 1)
        let l3 = (load8(19) >> 1) & MASK51;
        // Limb 4: bits 204..254 (byte 25, bit 4)
        let l4 = (load8(25) >> 4) & MASK51;

        Fe([l0, l1, l2, l3, l4])
    }

    /// Encode as 32 little-endian bytes (fully reduced).
    fn to_bytes(&self) -> [u8; 32] {
        let t = self.reduce();
        let mut out = [0u8; 32];
        let mut acc: u128 = 0;
        let mut bits: u32 = 0;
        let mut pos = 0;
        for &limb in t.0.iter() {
            acc |= (limb as u128) << bits;
            bits += 51;
            while bits >= 8 && pos < 32 {
                out[pos] = acc as u8;
                acc >>= 8;
                bits -= 8;
                pos += 1;
            }
        }
        while pos < 32 {
            out[pos] = acc as u8;
            acc >>= 8;
            pos += 1;
        }
        out
    }

    /// Full reduction mod p.
    fn reduce(&self) -> Fe {
        let t = self.carry();

        // p limbs
        let p: [u64; 5] = [(1u64 << 51) - 19, MASK51, MASK51, MASK51, MASK51];

        let mut borrow: i64 = 0;
        let mut diff = [0i64; 5];
        for i in 0..5 {
            let d = t.0[i] as i64 - p[i] as i64 + borrow;
            if d < 0 {
                diff[i] = d + (1i64 << 51);
                borrow = -1;
            } else {
                diff[i] = d;
                borrow = 0;
            }
        }

        if borrow == 0 {
            Fe([diff[0] as u64, diff[1] as u64, diff[2] as u64, diff[3] as u64, diff[4] as u64])
        } else {
            t
        }
    }

    /// Carry propagation.
    fn carry(&self) -> Fe {
        let mut t = self.0;
        for i in 0..4 {
            let carry = t[i] >> 51;
            t[i] &= MASK51;
            t[i + 1] += carry;
        }
        let carry = t[4] >> 51;
        t[4] &= MASK51;
        t[0] += carry * 19;
        let carry = t[0] >> 51;
        t[0] &= MASK51;
        t[1] += carry;
        Fe(t)
    }

    fn add(&self, rhs: &Fe) -> Fe {
        Fe([
            self.0[0] + rhs.0[0],
            self.0[1] + rhs.0[1],
            self.0[2] + rhs.0[2],
            self.0[3] + rhs.0[3],
            self.0[4] + rhs.0[4],
        ]).carry()
    }

    fn sub(&self, rhs: &Fe) -> Fe {
        let p: [u64; 5] = [(1u64 << 51) - 19, MASK51, MASK51, MASK51, MASK51];
        Fe([
            self.0[0] + 2 * p[0] - rhs.0[0],
            self.0[1] + 2 * p[1] - rhs.0[1],
            self.0[2] + 2 * p[2] - rhs.0[2],
            self.0[3] + 2 * p[3] - rhs.0[3],
            self.0[4] + 2 * p[4] - rhs.0[4],
        ]).carry()
    }

    fn neg(&self) -> Fe {
        Fe::ZERO.sub(self)
    }

    /// Multiplication using schoolbook with u128 intermediates.
    fn mul(&self, rhs: &Fe) -> Fe {
        let a = self.0;
        let b = rhs.0;

        // When a term a[i]*b[j] has i+j >= 5, it wraps around:
        // 2^(51*5) = 2^255 ≡ 19 (mod p), so multiply by 19.
        let b19 = [b[0], b[1] * 19, b[2] * 19, b[3] * 19, b[4] * 19];

        let mut r = [0u128; 5];
        r[0] = a[0] as u128 * b[0] as u128
             + a[1] as u128 * b19[4] as u128
             + a[2] as u128 * b19[3] as u128
             + a[3] as u128 * b19[2] as u128
             + a[4] as u128 * b19[1] as u128;

        r[1] = a[0] as u128 * b[1] as u128
             + a[1] as u128 * b[0] as u128
             + a[2] as u128 * b19[4] as u128
             + a[3] as u128 * b19[3] as u128
             + a[4] as u128 * b19[2] as u128;

        r[2] = a[0] as u128 * b[2] as u128
             + a[1] as u128 * b[1] as u128
             + a[2] as u128 * b[0] as u128
             + a[3] as u128 * b19[4] as u128
             + a[4] as u128 * b19[3] as u128;

        r[3] = a[0] as u128 * b[3] as u128
             + a[1] as u128 * b[2] as u128
             + a[2] as u128 * b[1] as u128
             + a[3] as u128 * b[0] as u128
             + a[4] as u128 * b19[4] as u128;

        r[4] = a[0] as u128 * b[4] as u128
             + a[1] as u128 * b[3] as u128
             + a[2] as u128 * b[2] as u128
             + a[3] as u128 * b[1] as u128
             + a[4] as u128 * b[0] as u128;

        let mut out = [0u64; 5];
        let mut carry: u128 = 0;
        for i in 0..5 {
            let sum = r[i] + carry;
            out[i] = (sum as u64) & MASK51;
            carry = sum >> 51;
        }
        out[0] += (carry as u64) * 19;
        let c = out[0] >> 51;
        out[0] &= MASK51;
        out[1] += c;

        Fe(out)
    }

    fn square(&self) -> Fe {
        self.mul(self)
    }

    /// Modular inverse via Fermat: a^(p-2) mod p.
    /// Uses an optimized addition chain for p-2 = 2^255 - 21.
    fn invert(&self) -> Fe {
        let a = *self;
        let a2 = a.square();
        let a_2_1 = a2.mul(&a);       // a^3 = a^(2^2-1)
        let a_4_1 = a_2_1.square().square().mul(&a_2_1); // a^(2^4-1) = a^15
        let a_5_1 = a_4_1.square().mul(&a); // a^(2^5-1) = a^31

        let mut t10 = a_5_1;
        for _ in 0..5 { t10 = t10.square(); }
        let t10 = t10.mul(&a_5_1);     // a^(2^10-1)

        let mut t20 = t10;
        for _ in 0..10 { t20 = t20.square(); }
        let t20 = t20.mul(&t10);

        let mut t40 = t20;
        for _ in 0..20 { t40 = t40.square(); }
        let t40 = t40.mul(&t20);

        let mut t50 = t40;
        for _ in 0..10 { t50 = t50.square(); }
        let t50 = t50.mul(&t10);

        let mut t100 = t50;
        for _ in 0..50 { t100 = t100.square(); }
        let t100 = t100.mul(&t50);

        let mut t200 = t100;
        for _ in 0..100 { t200 = t200.square(); }
        let t200 = t200.mul(&t100);

        let mut t250 = t200;
        for _ in 0..50 { t250 = t250.square(); }
        let t250 = t250.mul(&t50);

        // a^(2^255-32) = t250^(2^5)
        let mut r = t250;
        for _ in 0..5 { r = r.square(); }

        // a^(2^255-21) = a^(2^255-32) * a^11
        // a^11 = a^8 * a^3 = (a^2)^4 * a^3
        let a4 = a2.square();
        let a8 = a4.square();
        let a11 = a8.mul(&a_2_1);

        r.mul(&a11)
    }

    /// Square root: a^((p+3)/8), then check/fix.
    fn sqrt(&self) -> Option<Fe> {
        let candidate = self.pow_p_plus_3_over_8();
        let check = candidate.square();
        let a_mod = *self;

        if fe_eq(&check, &a_mod) {
            return Some(candidate);
        }

        let neg_a = a_mod.neg();
        if fe_eq(&check, &neg_a) {
            let sqrt_m1 = Fe::from_bytes(&SQRT_M1_BYTES);
            return Some(candidate.mul(&sqrt_m1));
        }

        None
    }

    /// Compute self^((p+3)/8) = self^(2^252 - 2).
    fn pow_p_plus_3_over_8(&self) -> Fe {
        let a = *self;
        let a2 = a.square();
        let a_2_1 = a2.mul(&a);
        let a_4_1 = a_2_1.square().square().mul(&a_2_1);
        let a_5_1 = a_4_1.square().mul(&a);

        let mut t10 = a_5_1;
        for _ in 0..5 { t10 = t10.square(); }
        let t10 = t10.mul(&a_5_1);

        let mut t20 = t10;
        for _ in 0..10 { t20 = t20.square(); }
        let t20 = t20.mul(&t10);

        let mut t40 = t20;
        for _ in 0..20 { t40 = t40.square(); }
        let t40 = t40.mul(&t20);

        let mut t50 = t40;
        for _ in 0..10 { t50 = t50.square(); }
        let t50 = t50.mul(&t10);

        let mut t100 = t50;
        for _ in 0..50 { t100 = t100.square(); }
        let t100 = t100.mul(&t50);

        let mut t200 = t100;
        for _ in 0..100 { t200 = t200.square(); }
        let t200 = t200.mul(&t100);

        let mut t250 = t200;
        for _ in 0..50 { t250 = t250.square(); }
        let t250 = t250.mul(&t50);

        // 2^252 - 2 = (2^250-1)*4 + 2
        // t250^4 = a^(4*(2^250-1)) = a^(2^252-4)
        // * a^2 = a^(2^252-2)
        let mut r = t250;
        r = r.square().square();
        r.mul(&a2)
    }
}

/// Check two field elements for equality after full reduction.
fn fe_eq(a: &Fe, b: &Fe) -> bool {
    a.reduce().to_bytes() == b.reduce().to_bytes()
}


// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 3: EXTENDED TWISTED EDWARDS POINT OPERATIONS
// ═══════════════════════════════════════════════════════════════════════════════

/// A point on the Ed25519 curve in extended twisted Edwards coordinates.
#[derive(Clone, Copy)]
struct ExtPoint {
    x: Fe,
    y: Fe,
    z: Fe,
    t: Fe,
}

impl ExtPoint {
    /// The identity element: affine (0, 1).
    fn identity() -> Self {
        ExtPoint { x: Fe::ZERO, y: Fe::ONE, z: Fe::ONE, t: Fe::ZERO }
    }

    /// The base point B.
    fn base() -> Self {
        let bx = Fe::from_bytes(&BX_BYTES);
        let by = Fe::from_bytes(&BY_BYTES);
        let bt = bx.mul(&by);
        ExtPoint { x: bx, y: by, z: Fe::ONE, t: bt }
    }

    /// Point addition (twisted Edwards, a = -1).
    fn add(&self, other: &ExtPoint) -> ExtPoint {
        let d_fe = Fe::from_bytes(&D_BYTES);

        let a = self.x.mul(&other.x);
        let b = self.y.mul(&other.y);
        let c = self.t.mul(&d_fe).mul(&other.t);
        let dd = self.z.mul(&other.z);

        let e = self.x.add(&self.y).mul(&other.x.add(&other.y)).sub(&a).sub(&b);
        let f = dd.sub(&c);
        let g = dd.add(&c);
        let h = b.add(&a);

        ExtPoint {
            x: e.mul(&f),
            y: g.mul(&h),
            z: f.mul(&g),
            t: e.mul(&h),
        }
    }

    /// Point doubling.
    fn double(&self) -> ExtPoint {
        let a = self.x.square();
        let b = self.y.square();
        let c = self.z.square().add(&self.z.square());

        let dd = a.neg();
        let e = self.x.add(&self.y).square().sub(&a).sub(&b);
        let g = dd.add(&b);
        let f = g.sub(&c);
        let h = dd.sub(&b);

        ExtPoint {
            x: e.mul(&f),
            y: g.mul(&h),
            z: f.mul(&g),
            t: e.mul(&h),
        }
    }

    /// Scalar multiplication: double-and-add from MSB to LSB.
    fn scalar_mul(&self, scalar: &[u8; 32]) -> ExtPoint {
        let mut result = ExtPoint::identity();
        let mut started = false;

        // Iterate bytes from MSB (byte 31) to LSB (byte 0)
        for &byte in scalar.iter().rev() {
            for bit in (0..8).rev() {
                if started {
                    result = result.double();
                }
                if (byte >> bit) & 1 == 1 {
                    if !started {
                        result = *self;
                        started = true;
                    } else {
                        result = result.add(self);
                    }
                }
            }
        }

        if !started {
            return ExtPoint::identity();
        }

        result
    }

    /// Encode point as 32 bytes per RFC 8032.
    fn encode(&self) -> [u8; 32] {
        let z_inv = self.z.invert();
        let x_aff = self.x.mul(&z_inv);
        let y_aff = self.y.mul(&z_inv);

        let mut encoded = y_aff.to_bytes();
        let x_bytes = x_aff.to_bytes();
        encoded[31] |= (x_bytes[0] & 1) << 7;

        encoded
    }

    /// Decode a 32-byte compressed point.
    fn decode(data: &[u8; 32]) -> Option<ExtPoint> {
        let sign = (data[31] >> 7) & 1;

        let mut y_bytes = *data;
        y_bytes[31] &= 0x7F;

        let y = Fe::from_bytes(&y_bytes);

        // Check y < p: encode the reduced value and compare
        let y_reduced = y.reduce().to_bytes();
        if y_bytes != y_reduced {
            return None;
        }

        let d_fe = Fe::from_bytes(&D_BYTES);
        let y_sq = y.square();
        let num = y_sq.sub(&Fe::ONE);
        let den = d_fe.mul(&y_sq).add(&Fe::ONE);
        let x_sq = num.mul(&den.invert());

        let x_sq_bytes = x_sq.reduce().to_bytes();
        if x_sq_bytes == [0u8; 32] {
            if sign == 1 {
                return None;
            }
            return Some(ExtPoint { x: Fe::ZERO, y, z: Fe::ONE, t: Fe::ZERO });
        }

        let x = x_sq.sqrt()?;

        let x_bytes = x.reduce().to_bytes();
        let x_final = if (x_bytes[0] & 1) != sign {
            x.neg()
        } else {
            x
        };

        let t = x_final.mul(&y);
        Some(ExtPoint { x: x_final, y, z: Fe::ONE, t })
    }
}


// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 4: SCALAR ARITHMETIC MOD L
// ═══════════════════════════════════════════════════════════════════════════════
//
// We need to reduce 512-bit SHA-512 outputs mod L and compute (r + k*a) mod L.
// We use a simple multi-precision arithmetic approach with u64 words.

/// Load a little-endian byte array as an array of u64 words.
fn bytes_to_words(b: &[u8]) -> Vec<u64> {
    let mut words = Vec::new();
    let chunks = b.chunks(8);
    for chunk in chunks {
        let mut arr = [0u8; 8];
        arr[..chunk.len()].copy_from_slice(chunk);
        words.push(u64::from_le_bytes(arr));
    }
    words
}

/// Encode a multi-word number as 32 little-endian bytes.
fn words_to_32bytes(words: &[u64]) -> [u8; 32] {
    let mut out = [0u8; 32];
    for (i, &w) in words.iter().enumerate().take(4) {
        let bytes = w.to_le_bytes();
        let start = i * 8;
        let end = (start + 8).min(32);
        out[start..end].copy_from_slice(&bytes[..end - start]);
    }
    out
}

/// Compare two multi-word numbers. Returns Ordering.
fn words_cmp(a: &[u64], b: &[u64]) -> std::cmp::Ordering {
    let len = a.len().max(b.len());
    for i in (0..len).rev() {
        let aw = if i < a.len() { a[i] } else { 0 };
        let bw = if i < b.len() { b[i] } else { 0 };
        match aw.cmp(&bw) {
            std::cmp::Ordering::Equal => continue,
            other => return other,
        }
    }
    std::cmp::Ordering::Equal
}

/// Subtract b from a (multi-word), assuming a >= b. Returns (result, borrow).
fn words_sub(a: &[u64], b: &[u64]) -> Vec<u64> {
    let len = a.len().max(b.len());
    let mut result = vec![0u64; len];
    let mut borrow: u128 = 0;
    for i in 0..len {
        let aw = if i < a.len() { a[i] as u128 } else { 0 };
        let bw = if i < b.len() { b[i] as u128 } else { 0 };
        let diff = aw.wrapping_sub(bw).wrapping_sub(borrow);
        result[i] = diff as u64;
        borrow = if aw < bw + borrow { 1 } else { 0 };
    }
    // Trim leading zeros
    while result.len() > 1 && *result.last().unwrap() == 0 {
        result.pop();
    }
    result
}

/// Multiply two multi-word numbers.
fn words_mul(a: &[u64], b: &[u64]) -> Vec<u64> {
    let mut result = vec![0u128; a.len() + b.len()];
    for i in 0..a.len() {
        let mut carry: u128 = 0;
        for j in 0..b.len() {
            let prod = a[i] as u128 * b[j] as u128 + result[i + j] + carry;
            result[i + j] = prod & 0xFFFFFFFFFFFFFFFF;
            carry = prod >> 64;
        }
        result[i + b.len()] += carry;
    }
    // Convert to u64 and trim
    let mut out: Vec<u64> = result.iter().map(|&v| v as u64).collect();
    while out.len() > 1 && *out.last().unwrap() == 0 {
        out.pop();
    }
    out
}

/// Add two multi-word numbers.
fn words_add(a: &[u64], b: &[u64]) -> Vec<u64> {
    let len = a.len().max(b.len());
    let mut result = vec![0u64; len + 1];
    let mut carry: u128 = 0;
    for i in 0..=len {
        let aw = if i < a.len() { a[i] as u128 } else { 0 };
        let bw = if i < b.len() { b[i] as u128 } else { 0 };
        let sum = aw + bw + carry;
        result[i] = sum as u64;
        carry = sum >> 64;
    }
    while result.len() > 1 && *result.last().unwrap() == 0 {
        result.pop();
    }
    result
}

/// Reduce a multi-word number mod L using repeated subtraction.
/// This works because our numbers are at most 512+256 bits ≈ 2^768,
/// and L is ~2^252, so at most ~2^516/2^252 = 2^264 iterations in the
/// worst case. That's way too many.
///
/// Instead, we use schoolbook division.
fn words_mod_l(n: &[u64]) -> Vec<u64> {
    let l_words = bytes_to_words(&L_BYTES);

    // If n < L, return n
    let mut remainder = n.to_vec();

    // Schoolbook long division approach:
    // We find the highest bit of remainder and L, then shift L up to align,
    // subtract, repeat.
    let l_bits = word_bit_len(&l_words);

    loop {
        let r_bits = word_bit_len(&remainder);
        if r_bits < l_bits {
            break;
        }
        if r_bits == l_bits && words_cmp(&remainder, &l_words) == std::cmp::Ordering::Less {
            break;
        }

        let shift = if r_bits > l_bits { r_bits - l_bits } else { 0 };

        // Compute L << shift
        let shifted = words_shl(&l_words, shift);

        if words_cmp(&remainder, &shifted) != std::cmp::Ordering::Less {
            remainder = words_sub(&remainder, &shifted);
        } else if shift > 0 {
            let shifted = words_shl(&l_words, shift - 1);
            remainder = words_sub(&remainder, &shifted);
        } else {
            break;
        }
    }

    remainder
}

/// Bit length of a multi-word number.
fn word_bit_len(words: &[u64]) -> usize {
    for i in (0..words.len()).rev() {
        if words[i] != 0 {
            return i * 64 + (64 - words[i].leading_zeros() as usize);
        }
    }
    0
}

/// Left-shift a multi-word number by `shift` bits.
fn words_shl(words: &[u64], shift: usize) -> Vec<u64> {
    if shift == 0 {
        return words.to_vec();
    }
    let word_shift = shift / 64;
    let bit_shift = shift % 64;

    let mut result = vec![0u64; words.len() + word_shift + 1];
    for i in 0..words.len() {
        result[i + word_shift] |= if bit_shift == 0 { words[i] } else { words[i] << bit_shift };
        if bit_shift > 0 && i + word_shift + 1 < result.len() {
            result[i + word_shift + 1] |= words[i] >> (64 - bit_shift);
        }
    }
    // Trim
    while result.len() > 1 && *result.last().unwrap() == 0 {
        result.pop();
    }
    result
}

/// Reduce a 64-byte SHA-512 output mod L, returning 32-byte LE scalar.
fn sc_reduce(hash: &[u8; 64]) -> [u8; 32] {
    let words = bytes_to_words(hash);
    let reduced = words_mod_l(&words);
    words_to_32bytes(&reduced)
}

/// Compute (r + k * a) mod L.
fn sc_muladd(r: &[u8; 32], k: &[u8; 32], a: &[u8; 32]) -> [u8; 32] {
    let r_words = bytes_to_words(r);
    let k_words = bytes_to_words(k);
    let a_words = bytes_to_words(a);

    let product = words_mul(&k_words, &a_words);
    let sum = words_add(&product, &r_words);
    let reduced = words_mod_l(&sum);
    words_to_32bytes(&reduced)
}

/// Check if a 32-byte LE scalar is < L (canonical).
fn sc_is_canonical(s: &[u8; 32]) -> bool {
    for i in (0..32).rev() {
        if s[i] < L_BYTES[i] { return true; }
        if s[i] > L_BYTES[i] { return false; }
    }
    false // s == L
}


// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 5: CLAMPING
// ═══════════════════════════════════════════════════════════════════════════════

fn clamp(h: &[u8; 64]) -> [u8; 32] {
    let mut clamped = [0u8; 32];
    clamped.copy_from_slice(&h[..32]);
    clamped[0] &= 248;
    clamped[31] &= 127;
    clamped[31] |= 64;
    clamped
}


// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 6: PUBLIC API
// ═══════════════════════════════════════════════════════════════════════════════

/// Generate an Ed25519 key pair from a 32-byte seed.
///
/// Returns `(public_key, secret_key)` where public_key is 32 bytes
/// and secret_key is 64 bytes (seed || public_key).
///
/// # Example
///
/// ```
/// use coding_adventures_ed25519::generate_keypair;
/// let seed = [0u8; 32];
/// let (public_key, secret_key) = generate_keypair(&seed);
/// assert_eq!(public_key.len(), 32);
/// assert_eq!(secret_key.len(), 64);
/// ```
pub fn generate_keypair(seed: &[u8; 32]) -> ([u8; 32], [u8; 64]) {
    let h = sum512(seed);
    let a = clamp(&h);

    let big_a = ExtPoint::base().scalar_mul(&a);
    let public_key = big_a.encode();

    let mut secret_key = [0u8; 64];
    secret_key[..32].copy_from_slice(seed);
    secret_key[32..].copy_from_slice(&public_key);

    (public_key, secret_key)
}

/// Sign a message with an Ed25519 secret key (64 bytes).
///
/// Returns a 64-byte signature (R || S). Signing is deterministic.
///
/// # Example
///
/// ```
/// use coding_adventures_ed25519::{generate_keypair, sign};
/// let seed = [0u8; 32];
/// let (_, sec_key) = generate_keypair(&seed);
/// let sig = sign(b"hello", &sec_key);
/// assert_eq!(sig.len(), 64);
/// ```
pub fn sign(message: &[u8], secret_key: &[u8; 64]) -> [u8; 64] {
    let seed: [u8; 32] = secret_key[..32].try_into().unwrap();
    let public_key: &[u8; 32] = secret_key[32..].try_into().unwrap();

    let h = sum512(&seed);
    let a = clamp(&h);
    let prefix = &h[32..64];

    // r = SHA-512(prefix || message) mod L
    let mut r_input = Vec::with_capacity(32 + message.len());
    r_input.extend_from_slice(prefix);
    r_input.extend_from_slice(message);
    let r_hash = sum512(&r_input);
    let r = sc_reduce(&r_hash);

    // R = r · B
    let big_r = ExtPoint::base().scalar_mul(&r);
    let r_bytes = big_r.encode();

    // k = SHA-512(R || public_key || message) mod L
    let mut k_input = Vec::with_capacity(64 + message.len());
    k_input.extend_from_slice(&r_bytes);
    k_input.extend_from_slice(public_key);
    k_input.extend_from_slice(message);
    let k_hash = sum512(&k_input);
    let k = sc_reduce(&k_hash);

    // S = (r + k·a) mod L
    let s = sc_muladd(&r, &k, &a);

    let mut sig = [0u8; 64];
    sig[..32].copy_from_slice(&r_bytes);
    sig[32..].copy_from_slice(&s);
    sig
}

/// Verify an Ed25519 signature.
///
/// Returns `true` if the signature is valid. Never panics on invalid input.
///
/// # Example
///
/// ```
/// use coding_adventures_ed25519::{generate_keypair, sign, verify};
/// let seed = [0u8; 32];
/// let (pub_key, sec_key) = generate_keypair(&seed);
/// let sig = sign(b"hello", &sec_key);
/// assert!(verify(b"hello", &sig, &pub_key));
/// ```
pub fn verify(message: &[u8], signature: &[u8; 64], public_key: &[u8; 32]) -> bool {
    let r_bytes: [u8; 32] = signature[..32].try_into().unwrap();
    let s_bytes: [u8; 32] = signature[32..].try_into().unwrap();

    if !sc_is_canonical(&s_bytes) {
        return false;
    }

    let big_r = match ExtPoint::decode(&r_bytes) {
        Some(pt) => pt,
        None => return false,
    };

    let big_a = match ExtPoint::decode(public_key) {
        Some(pt) => pt,
        None => return false,
    };

    let mut k_input = Vec::with_capacity(64 + message.len());
    k_input.extend_from_slice(&r_bytes);
    k_input.extend_from_slice(public_key);
    k_input.extend_from_slice(message);
    let k_hash = sum512(&k_input);
    let k = sc_reduce(&k_hash);

    let lhs = ExtPoint::base().scalar_mul(&s_bytes);
    let rhs = big_r.add(&big_a.scalar_mul(&k));

    let lx_rz = lhs.x.mul(&rhs.z);
    let rx_lz = rhs.x.mul(&lhs.z);
    let ly_rz = lhs.y.mul(&rhs.z);
    let ry_lz = rhs.y.mul(&lhs.z);

    fe_eq(&lx_rz, &rx_lz) && fe_eq(&ly_rz, &ry_lz)
}


// ═══════════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn hex_decode(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i+2], 16).unwrap())
            .collect()
    }

    fn hex_to_32(s: &str) -> [u8; 32] {
        let b = hex_decode(s);
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&b);
        arr
    }

    fn hex_to_64(s: &str) -> [u8; 64] {
        let b = hex_decode(s);
        let mut arr = [0u8; 64];
        arr.copy_from_slice(&b);
        arr
    }

    // ── Field Arithmetic ──

    #[test]
    fn test_field_inv_basic() {
        let three = Fe([3, 0, 0, 0, 0]);
        let inv = three.invert();
        let product = three.mul(&inv);
        assert!(fe_eq(&product, &Fe::ONE), "3 * inv(3) should be 1");
    }

    #[test]
    fn test_field_inv_one() {
        let inv = Fe::ONE.invert();
        assert!(fe_eq(&inv, &Fe::ONE), "inv(1) should be 1");
    }

    #[test]
    fn test_sqrt_m1_squared() {
        let sm1 = Fe::from_bytes(&SQRT_M1_BYTES);
        let sq = sm1.square();
        let neg_one = Fe::ZERO.sub(&Fe::ONE);
        assert!(fe_eq(&sq, &neg_one), "sqrt(-1)^2 should be -1");
    }

    #[test]
    fn test_field_sqrt_perfect_square() {
        let val = Fe([42, 0, 0, 0, 0]);
        let sq = val.square();
        let root = sq.sqrt().expect("42^2 should have a sqrt");
        let check = root.square();
        assert!(fe_eq(&check, &sq));
    }

    #[test]
    fn test_field_sqrt_no_root() {
        let two = Fe([2, 0, 0, 0, 0]);
        assert!(two.sqrt().is_none(), "2 has no sqrt mod p");
    }

    // ── Point Operations ──

    #[test]
    fn test_identity_add() {
        let bp = ExtPoint::base();
        let result = ExtPoint::identity().add(&bp);
        assert_eq!(result.encode(), bp.encode());
    }

    #[test]
    fn test_double_equals_add() {
        let bp = ExtPoint::base();
        let doubled = bp.double();
        let added = bp.add(&bp);
        assert_eq!(doubled.encode(), added.encode());
    }

    #[test]
    fn test_scalar_mult_zero() {
        let bp = ExtPoint::base();
        let result = bp.scalar_mul(&[0u8; 32]);
        assert_eq!(result.encode(), ExtPoint::identity().encode());
    }

    #[test]
    fn test_scalar_mult_one() {
        let bp = ExtPoint::base();
        let mut one = [0u8; 32];
        one[0] = 1;
        let result = bp.scalar_mul(&one);
        assert_eq!(result.encode(), bp.encode());
    }

    #[test]
    fn test_scalar_mult_two() {
        let bp = ExtPoint::base();
        let mut two = [0u8; 32];
        two[0] = 2;
        let result = bp.scalar_mul(&two);
        assert_eq!(result.encode(), bp.add(&bp).encode());
    }

    #[test]
    fn test_scalar_mult_order() {
        let bp = ExtPoint::base();
        let result = bp.scalar_mul(&L_BYTES);
        assert_eq!(result.encode(), ExtPoint::identity().encode());
    }

    #[test]
    fn test_base_point_on_curve() {
        let bx = Fe::from_bytes(&BX_BYTES);
        let by = Fe::from_bytes(&BY_BYTES);
        let d_fe = Fe::from_bytes(&D_BYTES);

        let x_sq = bx.square();
        let y_sq = by.square();
        let lhs = y_sq.sub(&x_sq);
        let rhs = Fe::ONE.add(&d_fe.mul(&x_sq).mul(&y_sq));
        assert!(fe_eq(&lhs, &rhs), "base point should be on curve");
    }

    // ── Point Encoding/Decoding ──

    #[test]
    fn test_encode_decode_base_point() {
        let bp = ExtPoint::base();
        let encoded = bp.encode();
        let decoded = ExtPoint::decode(&encoded).expect("should decode");
        assert_eq!(decoded.encode(), encoded);
    }

    #[test]
    fn test_encode_decode_identity() {
        let id = ExtPoint::identity();
        let encoded = id.encode();
        let decoded = ExtPoint::decode(&encoded).expect("should decode");
        assert_eq!(decoded.encode(), encoded);
    }

    #[test]
    fn test_encode_decode_double_base() {
        let bp = ExtPoint::base();
        let double = bp.double();
        let encoded = double.encode();
        let decoded = ExtPoint::decode(&encoded).expect("should decode");
        assert_eq!(decoded.encode(), encoded);
    }

    // ── RFC 8032 Test Vectors ──

    #[test]
    fn test_vector_1_empty_message() {
        let seed = hex_to_32("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60");
        let expected_pub = hex_to_32("d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a");
        let expected_sig = hex_to_64(
            "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b"
        );

        let (pub_key, sec_key) = generate_keypair(&seed);
        assert_eq!(pub_key, expected_pub, "public key mismatch");

        let sig = sign(b"", &sec_key);
        assert_eq!(sig, expected_sig, "signature mismatch");

        assert!(verify(b"", &sig, &pub_key));
    }

    #[test]
    fn test_vector_2_one_byte() {
        let seed = hex_to_32("4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb");
        let expected_pub = hex_to_32("3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c");
        let expected_sig = hex_to_64(
            "92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00"
        );

        let (pub_key, sec_key) = generate_keypair(&seed);
        assert_eq!(pub_key, expected_pub);

        let sig = sign(&hex_decode("72"), &sec_key);
        assert_eq!(sig, expected_sig);

        assert!(verify(&hex_decode("72"), &sig, &pub_key));
    }

    #[test]
    fn test_vector_3_two_bytes() {
        let seed = hex_to_32("c5aa8df43f9f837bedb7442f31dcb7b166d38535076f094b85ce3a2e0b4458f7");
        let expected_pub = hex_to_32("fc51cd8e6218a1a38da47ed00230f0580816ed13ba3303ac5deb911548908025");
        let expected_sig = hex_to_64(
            "6291d657deec24024827e69c3abe01a30ce548a284743a445e3680d7db5ac3ac18ff9b538d16f290ae67f760984dc6594a7c15e9716ed28dc027beceea1ec40a"
        );

        let (pub_key, sec_key) = generate_keypair(&seed);
        assert_eq!(pub_key, expected_pub);

        let sig = sign(&hex_decode("af82"), &sec_key);
        assert_eq!(sig, expected_sig);

        assert!(verify(&hex_decode("af82"), &sig, &pub_key));
    }

    #[test]
    fn test_vector_4_1023_bytes() {
        let seed = hex_to_32("f5e5767cf153319517630f226876b86c8160cc583bc013744c6bf255f5cc0ee5");
        let expected_pub = hex_to_32("278117fc144c72340f67d0f2316e8386ceffbf2b2428c9c51fef7c597f1d426e");
        let expected_sig = hex_to_64(
            "d686294b743c6760c6a78a2c4c2fc76115c2600b8f083acde59e7cee32578c0f59ea4219ab9b5896795e4e2b87a30270aa0e3099eee944e9e67a1b22df41ff07"
        );

        let message = hex_decode(concat!(
            "08b8b2b733424243760fe426a4b54908632110a66c2f6591eabd3345e3e4eb98",
            "fa6e264bf09efe12ee50f8f54e9f77b1e355f6c50544e23fb1433ddf73be84d8",
            "79de7c0046dc4996d9e773f4bc9efe5738829adb26c81b37c93a1b270b20329d",
            "658675fc6ea534e0810a4432826bf58c941efb65d57a338bbd2e26640f89ffbc",
            "1a858efcb8550ee3a5e1998bd177e93a7363c344fe6b199ee5d02e82d522c4fe",
            "ba15452f80288a821a579116ec6dad2b3b310da903401aa62100ab5d1a36553e",
            "06203b33890cc9b832f79ef80560ccb9a39ce767967ed628c6ad573cb116dbef",
            "fefd75499da96bd68a8a97b928a8bbc103b6621fcde2beca1231d206be6cd9ec",
            "7aff6f6c94fcd7204ed3455c68c83f4a41da4af2b74ef5c53f1d8ac70bdcb7ed",
            "185ce81bd84359d44254d95629e9855a94a7c1958d1f8ada5d0532ed8a5aa3fb",
            "2d17ba70eb6248e594e1a2297acbbb39d502f1a8c6eb6f1ce22b3de1a1f40cc2",
            "4554119a831a9aad6079cad88425de6bde1a9187ebb6092cf67bf2b13fd65f27",
            "088d78b7e883c8759d2c4f5c65adb7553878ad575f9fad878e80a0c9ba63bcbc",
            "c2732e69485bbc9c90bfbd62481d9089beccf80cfe2df16a2cf65bd92dd597b0",
            "7e0917af48bbb75fed413d238f5555a7a569d80c3414a8d0859dc65a46128bab",
            "27af87a71314f318c782b23ebfe808b82b0ce26401d2e22f04d83d1255dc51ad",
            "dd3b75a2b1ae0784504df543af8969be3ea7082ff7fc9888c144da2af58429ec",
            "96031dbcad3dad9af0dcbaaaf268cb8fcffead94f3c7ca495e056a9b47acdb75",
            "1fb73e666c6c655ade8297297d07ad1ba5e43f1bca32301651339e22904cc8c4",
            "2f58c30c04aafdb038dda0847dd988dcda6f3bfd15c4b4c4525004aa06eeff8c",
            "a61783aacec57fb3d1f92b0fe2fd1a85f6724517b65e614ad6808d6f6ee34dff",
            "7310fdc82aebfd904b01e1dc54b2927094b2db68d6f903b68401adebf5a7e08d",
            "78ff4ef5d63653a65040cf9bfd4aca7984a74d37145986780fc0b16ac451649d",
            "e6188a7dbdf191f64b5fc5e2ab47b57f7f7276cd419c17a3ca8e1b939ae49e48",
            "8acba6b965610b5480109c8b17b80e1b7b750dfc7598d5d5011fd2dcc5600a32",
            "ef5b52a1ecc820e308aa342721aac0943bf6686b64b2579376504ccc493d97e6",
            "aed3fb0f9cd71a43dd497f01f17c0e2cb3797aa2a2f256656168e6c496afc5fb",
            "93246f6b1116398a346f1a641f3b041e989f7914f90cc2c7fff357876e506b50",
            "d334ba77c225bc307ba537152f3f1610e4eafe595f6d9d90d11faa933a15ef13",
            "69546868a7f3a45a96768d40fd9d03412c091c6315cf4fde7cb68606937380db",
            "2eaaa707b4c4185c32eddcdd306705e4dc1ffc872eeee475a64dfac86aba41c0",
            "618983f8741c5ef68d3a101e8a3b8cac60c905c15fc910840b94c00a0b9d00"
        ));

        let (pub_key, sec_key) = generate_keypair(&seed);
        assert_eq!(pub_key, expected_pub);

        let sig = sign(&message, &sec_key);
        assert_eq!(sig, expected_sig);

        assert!(verify(&message, &sig, &pub_key));
    }

    // ── Verification Edge Cases ──

    #[test]
    fn test_wrong_message() {
        let seed = hex_to_32("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60");
        let (pub_key, sec_key) = generate_keypair(&seed);
        let sig = sign(b"hello", &sec_key);
        assert!(!verify(b"world", &sig, &pub_key));
    }

    #[test]
    fn test_wrong_public_key() {
        let seed1 = hex_to_32("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60");
        let seed2 = hex_to_32("4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb");
        let (_, sec1) = generate_keypair(&seed1);
        let (pub2, _) = generate_keypair(&seed2);
        let sig = sign(b"hello", &sec1);
        assert!(!verify(b"hello", &sig, &pub2));
    }

    #[test]
    fn test_tampered_signature_r() {
        let seed = hex_to_32("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60");
        let (pub_key, sec_key) = generate_keypair(&seed);
        let mut sig = sign(b"hello", &sec_key);
        sig[0] ^= 1;
        assert!(!verify(b"hello", &sig, &pub_key));
    }

    #[test]
    fn test_tampered_signature_s() {
        let seed = hex_to_32("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60");
        let (pub_key, sec_key) = generate_keypair(&seed);
        let mut sig = sign(b"hello", &sec_key);
        sig[32] ^= 1;
        assert!(!verify(b"hello", &sig, &pub_key));
    }

    #[test]
    fn test_s_out_of_range() {
        let seed = hex_to_32("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60");
        let (pub_key, sec_key) = generate_keypair(&seed);
        let mut sig = sign(b"hello", &sec_key);
        sig[32..].copy_from_slice(&L_BYTES);
        assert!(!verify(b"hello", &sig, &pub_key));
    }

    // ── Key Generation ──

    #[test]
    fn test_deterministic() {
        let mut seed = [0u8; 32];
        for i in 0..32 { seed[i] = i as u8; }
        let (pub1, sec1) = generate_keypair(&seed);
        let (pub2, sec2) = generate_keypair(&seed);
        assert_eq!(pub1, pub2);
        assert_eq!(sec1, sec2);
    }

    #[test]
    fn test_sign_deterministic() {
        let mut seed = [0u8; 32];
        for i in 0..32 { seed[i] = i as u8; }
        let (_, sec) = generate_keypair(&seed);
        let sig1 = sign(b"hello", &sec);
        let sig2 = sign(b"hello", &sec);
        assert_eq!(sig1, sig2);
    }

    #[test]
    fn test_secret_key_format() {
        let mut seed = [0u8; 32];
        for i in 0..32 { seed[i] = i as u8; }
        let (pub_key, sec_key) = generate_keypair(&seed);
        assert_eq!(&sec_key[..32], &seed);
        assert_eq!(&sec_key[32..], &pub_key);
    }
}
