//! # X25519 — Elliptic Curve Diffie-Hellman on Curve25519 (RFC 7748)
//!
//! X25519 is one of the most widely deployed key-agreement protocols on the
//! internet.  Every TLS 1.3 handshake (HTTPS, SSH, Signal, WireGuard) almost
//! certainly uses it.
//!
//! The beauty of X25519 lies in its simplicity:
//!
//! ```text
//! shared_secret = x25519(my_private_key, your_public_key)
//! ```
//!
//! Both parties compute the same 32-byte shared secret, yet an eavesdropper
//! who sees both public keys cannot derive it.
//!
//! # Implementation strategy
//!
//! We represent field elements in GF(2^255 - 19) using five 64-bit limbs,
//! each carrying 51 bits of the value.  This "radix-2^51" representation
//! allows us to multiply two limbs and accumulate without overflow:
//!
//! ```text
//! max limb value ≈ 2^51
//! product of two limbs ≈ 2^102, fits in u128
//! sum of 5 such products ≈ 5 * 2^102 < 2^105, still fits in u128
//! ```
//!
//! This avoids the need for big-integer libraries entirely.

// ============================================================================
// Field Element Representation
// ============================================================================
//
// A field element in GF(2^255 - 19) is stored as five u64 limbs:
//
//     value = limbs[0] + limbs[1]*2^51 + limbs[2]*2^102 + limbs[3]*2^153 + limbs[4]*2^204
//
// Each limb nominally holds 51 bits, but during computation limbs may
// temporarily exceed 51 bits.  We "carry" (propagate overflow) periodically.
//
// Why 51-bit limbs?
// -----------------
// 255 / 5 = 51.  Five limbs of 51 bits = 255 bits, perfectly matching our
// field size.  The product of two 51-bit numbers is at most 2^102, and
// u128 can hold 2^128 — plenty of room for accumulating partial products.

/// A field element in GF(2^255 - 19), stored as five 51-bit limbs.
#[derive(Clone, Copy, Debug)]
struct Fe([u64; 5]);

/// The prime p = 2^255 - 19.
///
/// We don't store this as a Fe; instead we use it for final reduction.
/// In limb form it would be [2^51 - 19, 2^51 - 1, 2^51 - 1, 2^51 - 1, 2^51 - 1]
/// but we reduce differently.

const MASK51: u64 = (1u64 << 51) - 1;

impl Fe {
    /// The zero element (additive identity).
    const ZERO: Fe = Fe([0, 0, 0, 0, 0]);

    /// The one element (multiplicative identity).
    const ONE: Fe = Fe([1, 0, 0, 0, 0]);

    /// Create a field element from a small integer.
    #[cfg(test)]
    fn from_u64(val: u64) -> Fe {
        Fe([val, 0, 0, 0, 0])
    }

    // ---- Carry propagation ----
    //
    // After addition or multiplication, limbs may exceed 51 bits.
    // We propagate the excess from each limb to the next.  The final
    // limb's overflow wraps around multiplied by 19, because:
    //
    //     2^255 ≡ 19 (mod p)
    //
    // So overflow from limb[4] (which represents 2^204 * excess, and
    // if excess is in the 2^51 position, that's 2^255) becomes 19 * excess
    // added back to limb[0].

    fn carry_propagate(self) -> Fe {
        let mut l = self.0;

        // Propagate from limb 0 → 1 → 2 → 3 → 4
        l[1] += l[0] >> 51;
        l[0] &= MASK51;
        l[2] += l[1] >> 51;
        l[1] &= MASK51;
        l[3] += l[2] >> 51;
        l[2] &= MASK51;
        l[4] += l[3] >> 51;
        l[3] &= MASK51;

        // Wrap overflow from limb 4 back to limb 0, multiplied by 19
        let overflow = l[4] >> 51;
        l[4] &= MASK51;
        l[0] += overflow * 19;

        // One more carry from limb 0 in case the *19 caused overflow
        l[1] += l[0] >> 51;
        l[0] &= MASK51;

        Fe(l)
    }

    // ---- Addition ----
    //
    // Simply add corresponding limbs.  We don't reduce immediately —
    // the next operation (multiply or carry) will handle it.

    fn add(self, rhs: Fe) -> Fe {
        Fe([
            self.0[0] + rhs.0[0],
            self.0[1] + rhs.0[1],
            self.0[2] + rhs.0[2],
            self.0[3] + rhs.0[3],
            self.0[4] + rhs.0[4],
        ])
    }

    // ---- Subtraction ----
    //
    // To avoid underflow (u64 can't go negative), we add a multiple of p
    // before subtracting.  Adding 2*p ensures every limb stays positive.
    //
    // 2*p in limb form:
    //   limb 0: 2*(2^51 - 19) = 2^52 - 38
    //   limbs 1-4: 2*(2^51 - 1) = 2^52 - 2

    fn sub(self, rhs: Fe) -> Fe {
        // Add 2p to prevent underflow, then subtract
        Fe([
            (self.0[0] + (1u64 << 52) - 38) - rhs.0[0],
            (self.0[1] + (1u64 << 52) - 2) - rhs.0[1],
            (self.0[2] + (1u64 << 52) - 2) - rhs.0[2],
            (self.0[3] + (1u64 << 52) - 2) - rhs.0[3],
            (self.0[4] + (1u64 << 52) - 2) - rhs.0[4],
        ])
        .carry_propagate()
    }

    // ---- Multiplication ----
    //
    // This is the most performance-critical operation.  We compute the
    // full 5×5 schoolbook product using u128 intermediates.
    //
    // The trick: when a partial product would land in limb 5 or higher
    // (i.e., indices i+j >= 5), we multiply by 19 and fold it back into
    // the lower limbs.  This works because 2^255 ≡ 19 (mod p), so
    // limb[5] * 2^(5*51) = limb[5] * 2^255 ≡ limb[5] * 19 (mod p).
    //
    // Schoolbook layout (where * means we multiply by 19 and fold):
    //
    //           a0   a1   a2   a3   a4
    //     ×     b0   b1   b2   b3   b4
    //     ─────────────────────────────
    //     r0:  a0b0 + 19*(a1b4 + a2b3 + a3b2 + a4b1)
    //     r1:  a0b1 + a1b0 + 19*(a2b4 + a3b3 + a4b2)
    //     r2:  a0b2 + a1b1 + a2b0 + 19*(a3b4 + a4b3)
    //     r3:  a0b3 + a1b2 + a2b1 + a3b0 + 19*(a4b4)
    //     r4:  a0b4 + a1b3 + a2b2 + a3b1 + a4b0

    fn mul(self, rhs: Fe) -> Fe {
        let a = self.0;
        let b = rhs.0;

        // Pre-multiply by 19 for the folded terms
        let b19 = [
            0u64, // b[0] * 19 not needed
            b[1] * 19,
            b[2] * 19,
            b[3] * 19,
            b[4] * 19,
        ];

        // Accumulate into u128 to avoid overflow
        let r0 = a[0] as u128 * b[0] as u128
            + a[1] as u128 * b19[4] as u128
            + a[2] as u128 * b19[3] as u128
            + a[3] as u128 * b19[2] as u128
            + a[4] as u128 * b19[1] as u128;

        let r1 = a[0] as u128 * b[1] as u128
            + a[1] as u128 * b[0] as u128
            + a[2] as u128 * b19[4] as u128
            + a[3] as u128 * b19[3] as u128
            + a[4] as u128 * b19[2] as u128;

        let r2 = a[0] as u128 * b[2] as u128
            + a[1] as u128 * b[1] as u128
            + a[2] as u128 * b[0] as u128
            + a[3] as u128 * b19[4] as u128
            + a[4] as u128 * b19[3] as u128;

        let r3 = a[0] as u128 * b[3] as u128
            + a[1] as u128 * b[2] as u128
            + a[2] as u128 * b[1] as u128
            + a[3] as u128 * b[0] as u128
            + a[4] as u128 * b19[4] as u128;

        let r4 = a[0] as u128 * b[4] as u128
            + a[1] as u128 * b[3] as u128
            + a[2] as u128 * b[2] as u128
            + a[3] as u128 * b[1] as u128
            + a[4] as u128 * b[0] as u128;

        // Carry propagation on the u128 results
        Fe::carry_wide([r0, r1, r2, r3, r4])
    }

    // ---- Squaring ----
    //
    // Same as multiplication but exploits symmetry: a[i]*a[j] appears twice
    // (for i != j), so we double those terms.

    fn square(self) -> Fe {
        let a = self.0;

        // Doubled terms (where i != j, we get 2 * a[i] * a[j])
        let a0_2 = a[0] * 2;
        let a1_2 = a[1] * 2;
        let a2_2 = a[2] * 2;
        let a3_2 = a[3] * 2;

        let a3_19 = a[3] * 19;
        let a4_19 = a[4] * 19;

        let r0 = a[0] as u128 * a[0] as u128
            + a1_2 as u128 * a4_19 as u128
            + a2_2 as u128 * a3_19 as u128;

        let r1 = a0_2 as u128 * a[1] as u128
            + a2_2 as u128 * a4_19 as u128
            + a[3] as u128 * a3_19 as u128;

        let r2 = a0_2 as u128 * a[2] as u128
            + a[1] as u128 * a[1] as u128
            + a3_2 as u128 * a4_19 as u128;

        let r3 = a0_2 as u128 * a[3] as u128
            + a1_2 as u128 * a[2] as u128
            + a[4] as u128 * a4_19 as u128;

        let r4 = a0_2 as u128 * a[4] as u128
            + a1_2 as u128 * a[3] as u128
            + a[2] as u128 * a[2] as u128;

        Fe::carry_wide([r0, r1, r2, r3, r4])
    }

    /// Carry propagation from wide (u128) limbs down to u64.
    fn carry_wide(r: [u128; 5]) -> Fe {
        let mut out = [0u64; 5];

        // First pass: extract 51-bit limbs from u128 accumulators
        let mut carry = 0u128;
        for i in 0..5 {
            let sum = r[i] + carry;
            out[i] = (sum as u64) & MASK51;
            carry = sum >> 51;
        }

        // Fold the final carry back into limb 0 (multiply by 19)
        out[0] += (carry as u64) * 19;

        // One more propagation
        out[1] += out[0] >> 51;
        out[0] &= MASK51;

        Fe(out)
    }

    // ---- Scalar multiplication by a small constant ----
    //
    // Used for multiplying by a24 = 121666.

    fn mul_small(self, small: u64) -> Fe {
        let mut r = [0u128; 5];
        for i in 0..5 {
            r[i] = self.0[i] as u128 * small as u128;
        }
        Fe::carry_wide(r)
    }

    // ---- Inversion via Fermat's little theorem ----
    //
    // a^(-1) = a^(p-2) mod p
    //
    // p - 2 = 2^255 - 21
    //
    // We use an addition chain optimized for this specific exponent.
    // The chain computes a^(p-2) using 254 squarings and 11 multiplications.
    //
    // The exponent p - 2 in binary is:
    //   1111...1101011  (250 ones, then 0, 1, 0, 1, 1)
    //
    // We build up powers using repeated squaring and strategic multiplications.

    fn invert(self) -> Fe {
        // t0 = a^2
        let t0 = self.square();
        // t1 = t0^(2^2) = a^8
        let t1 = t0.square().square();
        // t1 = a^9 = a^8 * a
        let t1 = self.mul(t1);
        // t0 = a^11 = a^9 * a^2
        let t0 = t0.mul(t1);
        // t2 = a^22 = (a^11)^2
        let t2 = t0.square();
        // t1 = a^(2^5 - 1) = 31 = a^22 * a^9
        let t1 = t1.mul(t2);

        // t2 = a^(2^10 - 1)
        let mut t2 = t1.square();
        for _ in 1..5 {
            t2 = t2.square();
        }
        let t1 = t2.mul(t1);

        // t2 = a^(2^20 - 1)
        let mut t2 = t1.square();
        for _ in 1..10 {
            t2 = t2.square();
        }
        let t2 = t2.mul(t1);

        // t3 = a^(2^40 - 1)
        let mut t3 = t2.square();
        for _ in 1..20 {
            t3 = t3.square();
        }
        let t2 = t3.mul(t2);

        // t2 = a^(2^50 - 1)
        let mut t2 = t2.square();
        for _ in 1..10 {
            t2 = t2.square();
        }
        let t1 = t2.mul(t1);

        // t2 = a^(2^100 - 1)
        let mut t2 = t1.square();
        for _ in 1..50 {
            t2 = t2.square();
        }
        let t2 = t2.mul(t1);

        // t3 = a^(2^200 - 1)
        let mut t3 = t2.square();
        for _ in 1..100 {
            t3 = t3.square();
        }
        let t2 = t3.mul(t2);

        // t2 = a^(2^250 - 1)
        let mut t2 = t2.square();
        for _ in 1..50 {
            t2 = t2.square();
        }
        let t1 = t2.mul(t1);

        // t1 = a^(2^255 - 21) = a^(p-2)
        let t1 = t1.square();
        let t1 = t1.square();
        let t1 = t1.square();
        let t1 = t1.square();
        let t1 = t1.square();
        t1.mul(t0)
    }

    // ---- Canonical reduction ----
    //
    // After all computation, we need to reduce the result to [0, p).
    // A field element might be in [p, 2p) even after carry propagation.
    // We subtract p and check if the result underflowed.

    fn to_bytes(self) -> [u8; 32] {
        let t = self.carry_propagate().carry_propagate();
        let mut l = t.0;

        // Conditional subtraction of p:
        // p = 2^255 - 19, in limbs: [2^51 - 19, 2^51 - 1, 2^51 - 1, 2^51 - 1, 2^51 - 1]
        //
        // We check if our value >= p by trying to subtract p and seeing if
        // the result is non-negative.
        //
        // Add 19 to limb 0 (equivalent to subtracting p and adding 2^255)
        let mut q = (l[0] + 19) >> 51;
        q = (l[1] + q) >> 51;
        q = (l[2] + q) >> 51;
        q = (l[3] + q) >> 51;
        q = (l[4] + q) >> 51;
        // q is 1 if value >= p, 0 otherwise

        l[0] += 19 * q;
        // Propagate
        l[1] += l[0] >> 51;
        l[0] &= MASK51;
        l[2] += l[1] >> 51;
        l[1] &= MASK51;
        l[3] += l[2] >> 51;
        l[2] &= MASK51;
        l[4] += l[3] >> 51;
        l[3] &= MASK51;
        l[4] &= MASK51;

        // ---- Pack 5 x 51-bit limbs into 32 bytes (little-endian) ----
        //
        // The value is: v0 + v1*2^51 + v2*2^102 + v3*2^153 + v4*2^204
        //
        // We split into two u128 halves at the byte boundary (bit 128):
        //   lo = bits 0-127   (bytes 0-15)
        //   hi = bits 128-255 (bytes 16-31)
        //
        // v2 spans the boundary: its bits 0-25 are in lo (at positions 102-127),
        // and its bits 26-50 are in hi (at positions 128-152).

        let v0 = l[0] as u128;
        let v1 = l[1] as u128;
        let v2 = l[2] as u128;
        let v3 = l[3] as u128;
        let v4 = l[4] as u128;

        // lo captures bits 0-127: v0 at 0, v1 at 51, lower 26 bits of v2 at 102
        let lo: u128 = v0 | (v1 << 51) | (v2 << 102);

        // hi captures bits 128-254: upper 25 bits of v2, v3 at 25, v4 at 76
        let hi: u128 = (v2 >> 26) | (v3 << 25) | (v4 << 76);

        let mut out = [0u8; 32];
        let lo_le = lo.to_le_bytes();
        let hi_le = hi.to_le_bytes();
        out[..16].copy_from_slice(&lo_le);
        out[16..32].copy_from_slice(&hi_le);
        out
    }

    /// Decode a field element from 32 bytes (little-endian).
    fn from_bytes(bytes: &[u8; 32]) -> Fe {
        // Read as two u128 values (little-endian)
        let mut lo_bytes = [0u8; 16];
        let mut hi_bytes = [0u8; 16];
        lo_bytes.copy_from_slice(&bytes[0..16]);
        hi_bytes.copy_from_slice(&bytes[16..32]);

        let lo = u128::from_le_bytes(lo_bytes);
        let hi = u128::from_le_bytes(hi_bytes);

        // Extract 51-bit limbs
        // limb[0] = bits 0-50
        // limb[1] = bits 51-101
        // limb[2] = bits 102-152 (spans lo and hi boundary at bit 128)
        // limb[3] = bits 153-203
        // limb[4] = bits 204-254

        let l0 = (lo as u64) & MASK51;
        let l1 = ((lo >> 51) as u64) & MASK51;

        // Bits 102-127 from lo, bits 128-152 from hi
        let l2 = ((lo >> 102) as u64 | ((hi as u64) << 26)) & MASK51;

        // Bits 153-203 from hi (153 - 128 = 25)
        let l3 = ((hi >> 25) as u64) & MASK51;

        // Bits 204-254 from hi (204 - 128 = 76)
        let l4 = ((hi >> 76) as u64) & MASK51;

        Fe([l0, l1, l2, l3, l4])
    }
}

// ============================================================================
// Constant-Time Conditional Swap
// ============================================================================
//
// If swap == 1, exchange a and b.  If swap == 0, do nothing.
// This must not branch on `swap` — we use bitwise masking instead.

fn cswap(swap: u64, a: &mut Fe, b: &mut Fe) {
    // mask = 0xFFFF...FFFF if swap == 1, 0x0000...0000 if swap == 0
    let mask = (-(swap as i64)) as u64;

    for i in 0..5 {
        let dummy = mask & (a.0[i] ^ b.0[i]);
        a.0[i] ^= dummy;
        b.0[i] ^= dummy;
    }
}

// ============================================================================
// Scalar Clamping
// ============================================================================

fn clamp_scalar(k: &[u8; 32]) -> [u8; 32] {
    let mut clamped = *k;

    // Clear the three lowest bits — make k a multiple of 8 (cofactor clearing).
    // Curve25519 has cofactor h = 8.  Multiplying by a multiple of 8
    // ensures we land in the prime-order subgroup, preventing small
    // subgroup attacks.
    clamped[0] &= 248;

    // Clear bit 255 — keep k below 2^255
    clamped[31] &= 127;

    // Set bit 254 — ensure constant bit-length for constant-time execution
    clamped[31] |= 64;

    clamped
}

// ============================================================================
// The Montgomery Ladder
// ============================================================================
//
// This is the heart of X25519.  See the module-level documentation and the
// Python implementation for a detailed walkthrough of the algorithm.

fn montgomery_ladder(k_bytes: &[u8; 32], u_bytes: &[u8; 32]) -> [u8; 32] {
    let k = clamp_scalar(k_bytes);
    let mut u_masked = *u_bytes;
    u_masked[31] &= 0x7F; // Mask high bit of u-coordinate
    let u = Fe::from_bytes(&u_masked);

    let x_1 = u;
    let mut x_2 = Fe::ONE;
    let mut z_2 = Fe::ZERO;
    let mut x_3 = u;
    let mut z_3 = Fe::ONE;

    let mut swap: u64 = 0;

    // Iterate from bit 254 down to bit 0
    for t in (0..=254).rev() {
        // Extract bit t from the scalar
        let byte_idx = t / 8;
        let bit_idx = t % 8;
        let k_t = ((k[byte_idx] >> bit_idx) & 1) as u64;

        swap ^= k_t;
        cswap(swap, &mut x_2, &mut x_3);
        cswap(swap, &mut z_2, &mut z_3);
        swap = k_t;

        // ---- Montgomery ladder step ----
        let a = x_2.add(z_2);
        let aa = a.square();
        let b = x_2.sub(z_2);
        let bb = b.square();
        let e = aa.sub(bb);

        let c = x_3.add(z_3);
        let d = x_3.sub(z_3);
        let da = d.mul(a);
        let cb = c.mul(b);

        x_3 = da.add(cb).square();
        z_3 = x_1.mul(da.sub(cb).square());
        x_2 = aa.mul(bb);
        // z_2 = E * (BB + a24 * E), where a24 = (A+2)/4 = 121666
        // This comes from: Z_{2n} = 4xz * (x^2 + Axz + z^2)
        //                         = E * ((x-z)^2 + (A+2)/4 * E)
        //                         = E * (BB + 121666 * E)
        z_2 = e.mul(bb.add(e.mul_small(121666)));
    }

    // Final conditional swap
    cswap(swap, &mut x_2, &mut x_3);
    cswap(swap, &mut z_2, &mut z_3);

    // Convert from projective to affine: result = x_2 * z_2^(-1)
    let result = x_2.mul(z_2.invert());
    result.to_bytes()
}

// ============================================================================
// Public API
// ============================================================================

/// Compute X25519: scalar multiplication on Curve25519.
///
/// Given a 32-byte scalar (private key) and a 32-byte u-coordinate
/// (public key or base point), returns the 32-byte u-coordinate of
/// the resulting point.
///
/// # Errors
///
/// Returns `Err` if the result is all zeros (indicating a low-order point
/// input, which would be a security issue).
///
/// # Example
///
/// ```
/// use coding_adventures_x25519::{x25519, x25519_base};
///
/// let alice_private = [0x77, 0x07, 0x6d, 0x0a, 0x73, 0x18, 0xa5, 0x7d,
///                      0x3c, 0x16, 0xc1, 0x72, 0x51, 0xb2, 0x66, 0x45,
///                      0xdf, 0x4c, 0x2f, 0x87, 0xeb, 0xc0, 0x99, 0x2a,
///                      0xb1, 0x77, 0xfb, 0xa5, 0x1d, 0xb9, 0x2c, 0x2a];
/// let alice_public = x25519_base(&alice_private).unwrap();
/// assert_eq!(alice_public.len(), 32);
/// ```
pub fn x25519(scalar: &[u8; 32], u_coordinate: &[u8; 32]) -> Result<[u8; 32], &'static str> {
    let result = montgomery_ladder(scalar, u_coordinate);

    // Check for all-zeros result (point at infinity)
    if result == [0u8; 32] {
        return Err("X25519 produced the all-zeros output (low-order point)");
    }

    Ok(result)
}

/// The standard base point for Curve25519: u = 9.
///
/// This is the generator of the prime-order subgroup, encoded as
/// 32 bytes in little-endian.  Bernstein chose 9 as the smallest
/// valid generator — it's arbitrary but conventional.
pub const BASE_POINT: [u8; 32] = {
    let mut bp = [0u8; 32];
    bp[0] = 9;
    bp
};

/// Compute scalar multiplication with the standard base point (u = 9).
///
/// This is the standard way to derive a public key from a private key.
pub fn x25519_base(scalar: &[u8; 32]) -> Result<[u8; 32], &'static str> {
    x25519(scalar, &BASE_POINT)
}

/// Generate a public key from a private key.
///
/// This is simply `x25519_base` — included for API clarity.
/// The private key should be 32 bytes of cryptographically secure random data.
pub fn generate_keypair(private_key: &[u8; 32]) -> Result<[u8; 32], &'static str> {
    x25519_base(private_key)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Field Arithmetic Tests
    // ========================================================================

    #[test]
    fn test_fe_roundtrip() {
        // Encode a small value and decode it back
        let original = Fe::from_u64(42);
        let bytes = original.to_bytes();
        let recovered = Fe::from_bytes(&bytes);
        let recovered_bytes = recovered.to_bytes();
        assert_eq!(bytes, recovered_bytes);
        assert_eq!(bytes[0], 42);
        for b in &bytes[1..] {
            assert_eq!(*b, 0);
        }
    }

    #[test]
    fn test_fe_add() {
        let a = Fe::from_u64(3);
        let b = Fe::from_u64(5);
        let c = a.add(b).carry_propagate();
        assert_eq!(c.to_bytes()[0], 8);
    }

    #[test]
    fn test_fe_mul() {
        let a = Fe::from_u64(7);
        let b = Fe::from_u64(6);
        let c = a.mul(b);
        assert_eq!(c.to_bytes()[0], 42);
    }

    #[test]
    fn test_fe_square() {
        let a = Fe::from_u64(7);
        let sq = a.square();
        let mul = a.mul(a);
        assert_eq!(sq.to_bytes(), mul.to_bytes());
    }

    #[test]
    fn test_fe_invert() {
        // a * a^(-1) should equal 1
        let a = Fe::from_u64(42);
        let a_inv = a.invert();
        let product = a.mul(a_inv);
        assert_eq!(product.to_bytes(), Fe::ONE.to_bytes());
    }

    #[test]
    fn test_fe_invert_various() {
        for val in [1u64, 2, 3, 7, 42, 121666] {
            let a = Fe::from_u64(val);
            let a_inv = a.invert();
            let product = a.mul(a_inv);
            assert_eq!(product.to_bytes(), Fe::ONE.to_bytes(), "invert failed for {}", val);
        }
    }

    #[test]
    fn test_fe_sub() {
        let a = Fe::from_u64(10);
        let b = Fe::from_u64(3);
        let c = a.sub(b);
        assert_eq!(c.to_bytes()[0], 7);
    }

    // ========================================================================
    // Cswap Tests
    // ========================================================================

    #[test]
    fn test_cswap_no_swap() {
        let mut a = Fe::from_u64(10);
        let mut b = Fe::from_u64(20);
        cswap(0, &mut a, &mut b);
        assert_eq!(a.to_bytes()[0], 10);
        assert_eq!(b.to_bytes()[0], 20);
    }

    #[test]
    fn test_cswap_swap() {
        let mut a = Fe::from_u64(10);
        let mut b = Fe::from_u64(20);
        cswap(1, &mut a, &mut b);
        assert_eq!(a.to_bytes()[0], 20);
        assert_eq!(b.to_bytes()[0], 10);
    }

    // ========================================================================
    // Scalar Clamping Tests
    // ========================================================================

    #[test]
    fn test_clamp_scalar() {
        let k = [0xFFu8; 32];
        let clamped = clamp_scalar(&k);
        assert_eq!(clamped[0] & 0x07, 0);    // low 3 bits cleared
        assert_eq!(clamped[31] >> 7, 0);       // bit 255 cleared
        assert_eq!((clamped[31] >> 6) & 1, 1); // bit 254 set
    }

    #[test]
    fn test_clamp_sets_bit_254() {
        let k = [0u8; 32];
        let clamped = clamp_scalar(&k);
        assert_eq!((clamped[31] >> 6) & 1, 1);
    }

    // ========================================================================
    // RFC 7748 Test Vectors
    // ========================================================================

    fn hex_to_32(hex_str: &str) -> [u8; 32] {
        let mut out = [0u8; 32];
        for i in 0..32 {
            out[i] = u8::from_str_radix(&hex_str[2 * i..2 * i + 2], 16).unwrap();
        }
        out
    }

    #[test]
    fn test_vector_1() {
        let scalar = hex_to_32("a546e36bf0527c9d3b16154b82465edd62144c0ac1fc5a18506a2244ba449ac4");
        let u = hex_to_32("e6db6867583030db3594c1a424b15f7c726624ec26b3353b10a903a6d0ab1c4c");
        let expected = hex_to_32("c3da55379de9c6908e94ea4df28d084f32eccf03491c71f754b4075577a28552");
        assert_eq!(x25519(&scalar, &u).unwrap(), expected);
    }

    #[test]
    fn test_vector_2() {
        let scalar = hex_to_32("4b66e9d4d1b4673c5ad22691957d6af5c11b6421e0ea01d42ca4169e7918ba0d");
        let u = hex_to_32("e5210f12786811d3f4b7959d0538ae2c31dbe7106fc03c3efc4cd549c715a493");
        let expected = hex_to_32("95cbde9476e8907d7aade45cb4b873f88b595a68799fa152e6f8f7647aac7957");
        assert_eq!(x25519(&scalar, &u).unwrap(), expected);
    }

    #[test]
    fn test_alice_public_key() {
        let alice_private = hex_to_32("77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a");
        let expected = hex_to_32("8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a");
        assert_eq!(x25519_base(&alice_private).unwrap(), expected);
    }

    #[test]
    fn test_bob_public_key() {
        let bob_private = hex_to_32("5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb");
        let expected = hex_to_32("de9edb7d7b7dc1b4d35b61c2ece435373f8343c85b78674dadfc7e146f882b4f");
        assert_eq!(x25519_base(&bob_private).unwrap(), expected);
    }

    #[test]
    fn test_shared_secret() {
        let alice_private = hex_to_32("77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a");
        let bob_private = hex_to_32("5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb");

        let alice_public = x25519_base(&alice_private).unwrap();
        let bob_public = x25519_base(&bob_private).unwrap();

        let shared_ab = x25519(&alice_private, &bob_public).unwrap();
        let shared_ba = x25519(&bob_private, &alice_public).unwrap();

        let expected = hex_to_32("4a5d9d5ba4ce2de1728e3bf480350f25e07e21c947d19e3376f09b3c1e161742");
        assert_eq!(shared_ab, expected);
        assert_eq!(shared_ba, expected);
    }

    #[test]
    fn test_generate_keypair_is_x25519_base() {
        let private = hex_to_32("77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a");
        assert_eq!(generate_keypair(&private).unwrap(), x25519_base(&private).unwrap());
    }

    // ========================================================================
    // Iterated Tests
    // ========================================================================

    #[test]
    fn test_1_iteration() {
        let mut k = [0u8; 32];
        k[0] = 9;
        let mut u = [0u8; 32];
        u[0] = 9;

        let new_k = x25519(&k, &u).unwrap();
        let expected = hex_to_32("422c8e7a6227d7bca1350b3e2bb7279f7897b87bb6854b783c60e80311ae3079");
        assert_eq!(new_k, expected);
    }

    #[test]
    fn test_1000_iterations() {
        let mut k = [0u8; 32];
        k[0] = 9;
        let mut u = [0u8; 32];
        u[0] = 9;

        for _ in 0..1000 {
            let new_k = x25519(&k, &u).unwrap();
            u = k;
            k = new_k;
        }

        let expected = hex_to_32("684cf59ba83309552800ef566f2f4d3c1c3887c49360e3875f2eb94d99532c51");
        assert_eq!(k, expected);
    }

    // #[test]
    // fn test_1000000_iterations() {
    //     let mut k = [0u8; 32];
    //     k[0] = 9;
    //     let mut u = [0u8; 32];
    //     u[0] = 9;
    //     for _ in 0..1_000_000 {
    //         let new_k = x25519(&k, &u).unwrap();
    //         u = k;
    //         k = new_k;
    //     }
    //     let expected = hex_to_32("7c3911e0ab2586fd864497297e575e6f3bc601c0883c30df5f4dd2d24f665424");
    //     assert_eq!(k, expected);
    // }

    // ========================================================================
    // Edge Cases
    // ========================================================================

    #[test]
    fn test_base_point_is_nine() {
        let mut expected = [0u8; 32];
        expected[0] = 9;
        assert_eq!(BASE_POINT, expected);
    }

    #[test]
    fn test_fe_from_bytes_to_bytes_roundtrip() {
        // Test with various byte patterns
        let mut bytes = [0u8; 32];
        bytes[0] = 0xFF;
        bytes[15] = 0xAB;
        bytes[31] = 0x7F; // high bit already clear

        let fe = Fe::from_bytes(&bytes);
        let out = fe.to_bytes();
        assert_eq!(bytes, out);
    }
}
