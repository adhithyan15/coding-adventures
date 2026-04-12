//! # gf256 — Galois Field GF(2^8) arithmetic.
//!
//! **GF(2^8)** — read "GF of 256" — is a finite field with exactly 256 elements.
//! The elements are the integers 0 through 255, but the *arithmetic* is very
//! different from ordinary integer arithmetic.
//!
//! ## Why Finite Fields?
//!
//! Three important algorithms in modern computing rely on GF(256):
//!
//! 1. **Reed-Solomon error correction** — Used in QR codes, CDs, DVDs, hard drives,
//!    and deep-space communication. RS codes treat data bytes as GF(256) elements
//!    and perform polynomial arithmetic over GF(256) to add redundancy capable of
//!    detecting and correcting burst errors.
//!
//! 2. **QR codes** — The error correction codewords are an RS code over GF(256).
//!    A QR code can survive up to 30% damage thanks to this.
//!
//! 3. **AES encryption** — The AES SubBytes (S-box) and MixColumns steps use
//!    arithmetic in GF(2^8). (Note: AES uses a *different* primitive polynomial,
//!    `0x11B`, than this crate's `0x11D`.)
//!
//! ## Characteristic 2: Add = XOR = Subtract
//!
//! In **characteristic 2** fields, `1 + 1 = 0`. This means `-1 = 1`, so
//! subtraction equals addition. For bytes (GF(2^8) elements):
//!
//! ```text
//! add(a, b)      = a XOR b
//! subtract(a, b) = a XOR b   ← same operation!
//! ```
//!
//! Each bit of a byte is a GF(2) = {0, 1} coefficient. GF(2) addition is
//! `1 + 1 = 0 mod 2`, which is bitwise XOR. No carry is ever needed.
//!
//! ## The Primitive Polynomial
//!
//! Elements of GF(2^8) are polynomials over GF(2) with degree ≤ 7:
//!
//! ```text
//! a₇x⁷ + a₆x⁶ + … + a₁x + a₀,   each aᵢ ∈ {0, 1}
//! ```
//!
//! This gives 2^8 = 256 elements. To multiply two such polynomials (product
//! degree up to 14), we reduce modulo an **irreducible polynomial of degree 8**,
//! just as integers modulo a prime give a finite field.
//!
//! We use:
//!
//! ```text
//! p(x) = x^8 + x^4 + x^3 + x^2 + 1   =   0x11D   =   285
//! ```
//!
//! This polynomial is both *irreducible* (cannot be factored) and *primitive*
//! (the element `g = 2` generates all 255 non-zero elements of GF(256)).
//!
//! ## Log/Antilog Tables
//!
//! Because `g = 2` generates all non-zero elements, we can compute:
//!
//! ```text
//! ALOG[i] = g^i mod p(x)           antilogarithm table
//! LOG[x]  = i  such that g^i = x   logarithm table
//! ```
//!
//! This turns multiplication into two table lookups and one addition:
//!
//! ```text
//! a × b = ALOG[(LOG[a] + LOG[b]) mod 255]
//! ```
//!
//! Tables are precomputed once at first use via [`std::sync::OnceLock`].

use std::sync::OnceLock;

/// The additive identity element.
pub const ZERO: u8 = 0;

/// The multiplicative identity element.
pub const ONE: u8 = 1;

/// The primitive (irreducible) polynomial used for modular reduction.
///
/// `p(x) = x^8 + x^4 + x^3 + x^2 + 1`
///
/// Binary representation: bit 8 = x^8, bit 4 = x^4, bit 3 = x^3,
/// bit 2 = x^2, bit 0 = x^0 (the constant term 1):
///
/// ```text
/// 0b1_0001_1101 = 0x11D = 285
/// ```
///
/// We store this as `u16` because the value 285 does not fit in a `u8`.
/// When checking overflow during table construction, we test `val >= 256`
/// (i.e., bit 8 is set), then XOR with this constant to reduce.
pub const PRIMITIVE_POLYNOMIAL: u16 = 0x11d;

// =============================================================================
// Table construction
// =============================================================================

/// Holds both lookup tables as a single heap allocation.
///
/// - `log[x]` — the discrete logarithm of `x` base 2 in GF(256).
///   `log[0]` is defined as 0 but must never be accessed for valid computations,
///   since 0 has no logarithm.
/// - `alog[i]` — the antilogarithm: 2^i mod p(x) in GF(256).
///   `alog[255] = 1` to support `inverse(1)` correctly.
struct Tables {
    log: [u16; 256],
    alog: [u8; 256],
}

/// Build the logarithm and antilogarithm tables for GF(256).
///
/// ## Algorithm
///
/// Start with `val = 1` (which is g^0). For each step i from 0 to 254:
/// 1. Record `ALOG[i] = val` and `LOG[val] = i`.
/// 2. Multiply `val` by 2 (the generator `g`), which in GF(2^8) is a left
///    bit-shift by 1.
/// 3. If the result overflows a byte (bit 8 is set, i.e., val ≥ 256), reduce
///    modulo the primitive polynomial by XOR-ing with 0x11D.
///
/// ## Why shift-left equals multiply by 2
///
/// The element `2` in GF(2^8) is the polynomial `x` (bit 1 set, all else zero).
/// Multiplying any polynomial `f(x)` by `x` shifts all coefficients up by one
/// degree — which is exactly a left bit-shift of the byte representation.
/// When degree 8 appears (bit 8 set), we reduce modulo `p(x)` by subtracting
/// `p(x)` — which in GF(2) arithmetic is XOR.
///
/// ## First Few ALOG Values
///
/// | i  | ALOG[i] | hex  | note |
/// |----|---------|------|------|
/// | 0  | 1       | 0x01 | 2^0 = 1 |
/// | 1  | 2       | 0x02 | 2^1 = x |
/// | 7  | 128     | 0x80 | 2^7 = x^7 |
/// | 8  | 29      | 0x1D | 256 XOR 0x11D: first reduction |
/// | 9  | 58      | 0x3A | 29 * 2, no overflow |
///
/// At i=8: `128 << 1 = 256`. Since 256 ≥ 256, XOR with 285 (0x11D):
/// `256 XOR 285 = 0x100 XOR 0x11D = 0x01D = 29`. So `ALOG[8] = 29`.
///
/// ## The 256th Entry
///
/// `ALOG[255]` is set to 1. The multiplicative group has order 255, so
/// `g^255 = g^0 = 1`. This is needed for `inverse(1)`:
///   `ALOG[255 - LOG[1]] = ALOG[255 - 0] = ALOG[255] = 1 ✓`
fn build_tables() -> Tables {
    let mut log = [0u16; 256];
    let mut alog = [0u8; 256];

    let mut val: u16 = 1; // Start at g^0 = 1.
    for i in 0..255u16 {
        // Record the forward (antilog) and inverse (log) mappings.
        alog[i as usize] = val as u8;
        log[val as usize] = i;

        // Multiply val by g = 2 (left-shift by 1 in polynomial terms).
        val <<= 1;

        // If bit 8 is set (val overflowed a byte), reduce modulo p(x).
        // In GF(2) arithmetic, subtraction is XOR, so:
        //   val mod p(x) = val XOR p(x)
        if val >= 256 {
            val ^= PRIMITIVE_POLYNOMIAL;
        }
    }

    // g^255 = 1 (the cyclic group wraps around after 255 steps).
    alog[255] = 1;
    // LOG[0] is undefined but left as 0 (a safe default; callers must guard on 0).

    Tables { log, alog }
}

/// Lazily-initialized lookup tables, computed at most once.
///
/// [`OnceLock`] provides thread-safe one-time initialization without any
/// external crate dependencies. The first call to any arithmetic function
/// triggers table construction; subsequent calls return the cached result.
static TABLES: OnceLock<Tables> = OnceLock::new();

/// Get a reference to the global lookup tables, initializing if needed.
#[inline]
fn tables() -> &'static Tables {
    TABLES.get_or_init(build_tables)
}

// =============================================================================
// Field Operations
// =============================================================================

/// Add two GF(256) elements.
///
/// In a characteristic-2 field, addition is XOR. Each bit is an independent
/// GF(2) coefficient, and GF(2) addition is `1 + 1 = 0 (mod 2)` — i.e., XOR.
///
/// No overflow, no carry, no tables needed.
///
/// ```text
/// add(0x53, 0xCA)   =   0x53 XOR 0xCA   =   0x99
/// add(x, x)         =   0   for all x     (every element is its own inverse)
/// ```
#[inline]
pub fn add(a: u8, b: u8) -> u8 {
    a ^ b
}

/// Subtract two GF(256) elements.
///
/// In characteristic 2, negation is the identity: `-x = x` for all `x`.
/// Therefore subtraction equals addition:
///
/// ```text
/// subtract(a, b) = a + (-b) = a + b = a XOR b
/// ```
///
/// This is not a coincidence — it is a fundamental consequence of working in
/// a field where every element satisfies `x + x = 0`.
#[inline]
pub fn subtract(a: u8, b: u8) -> u8 {
    a ^ b
}

/// Multiply two GF(256) elements using logarithm/antilogarithm tables.
///
/// The mathematical identity: `a × b = g^(log(a) + log(b))`
///
/// The modular addition `(LOG[a] + LOG[b]) % 255` keeps the exponent within
/// the cyclic group of order 255. (The group has exactly 255 elements, and
/// the exponents wrap around: `g^255 = g^0 = 1`.)
///
/// ## Special Case: Zero
///
/// Zero has no logarithm — it is not reachable as a power of the generator.
/// We handle it explicitly: `0 × anything = 0`.
///
/// ## Time Complexity
///
/// O(1): two array accesses and one addition.
pub fn multiply(a: u8, b: u8) -> u8 {
    // The product of anything with zero is zero.
    if a == 0 || b == 0 {
        return 0;
    }
    let t = tables();
    // Convert to exponents, add, reduce modulo the group order, convert back.
    let exp = (t.log[a as usize] as u32 + t.log[b as usize] as u32) % 255;
    t.alog[exp as usize]
}

/// Divide `a` by `b` in GF(256).
///
/// `a / b = g^(log(a) - log(b)) = ALOG[(LOG[a] - LOG[b] + 255) % 255]`
///
/// The `+ 255` before `% 255` ensures the result is non-negative when
/// `LOG[a] < LOG[b]`. Without it, integer subtraction could underflow.
///
/// ## Special Cases
///
/// - `a = 0`: returns 0 (zero divided by anything is zero)
///
/// ## Panics
///
/// Panics if `b == 0` (division by zero is undefined in any field).
pub fn divide(a: u8, b: u8) -> u8 {
    assert!(b != 0, "GF256: division by zero");
    if a == 0 {
        return 0;
    }
    let t = tables();
    // Subtract exponents (with +255 to avoid underflow), reduce mod 255.
    let log_a = t.log[a as usize] as i32;
    let log_b = t.log[b as usize] as i32;
    let exp = ((log_a - log_b + 255) % 255) as usize;
    t.alog[exp]
}

/// Raise a GF(256) element to a non-negative integer power.
///
/// Uses the logarithm table: `base^exp = ALOG[(LOG[base] * exp) % 255]`
///
/// The modulo 255 reflects the multiplicative group order — every non-zero
/// element satisfies `g^255 = 1` (Fermat's little theorem for finite fields).
///
/// ## Special Cases
///
/// - `base = 0, exp = 0`: returns 1 by convention (consistent with most math
///   libraries and the binomial theorem).
/// - `base = 0, exp > 0`: returns 0 (zero to any positive power is zero).
/// - `exp = 0`: returns 1 for any non-zero base.
///
/// ## Overflow Note
///
/// For very large `exp`, `LOG[base] * exp` could theoretically overflow `u64`
/// if `exp` is near `u64::MAX`. For the expected use cases in Reed-Solomon
/// (exp ≤ 255), this is not an issue.
pub fn power(base: u8, exp: u32) -> u8 {
    // Special case: 0^0 = 1 by convention.
    if base == 0 {
        return if exp == 0 { 1 } else { 0 };
    }
    // Any non-zero element raised to the power 0 is 1.
    if exp == 0 {
        return 1;
    }
    let t = tables();
    // Exponent arithmetic: (log * exp) mod 255 stays in [0, 254].
    let log_base = t.log[base as usize] as u64;
    let exp = ((log_base * exp as u64) % 255) as usize;
    t.alog[exp]
}

/// Compute the multiplicative inverse of a GF(256) element.
///
/// The inverse of `a` satisfies: `a × inverse(a) = 1`.
///
/// ## Derivation
///
/// By the cyclic group property:
/// ```text
/// a × a^(-1) = 1 = g^0 = g^255
/// log(a) + log(a^(-1)) ≡ 0 (mod 255)
/// log(a^(-1)) = 255 - log(a)
/// a^(-1) = ALOG[255 - LOG[a]]
/// ```
///
/// ## Panics
///
/// Panics if `a == 0` (zero has no multiplicative inverse — it is the additive
/// identity, and no element times zero can equal 1).
pub fn inverse(a: u8) -> u8 {
    assert!(a != 0, "GF256: zero has no multiplicative inverse");
    let t = tables();
    // 255 - log(a) gives the exponent of the inverse.
    let exp = 255 - t.log[a as usize] as usize;
    t.alog[exp]
}

// =============================================================================
// Field — parameterizable GF(2^8) field
// =============================================================================
//
// The functions above are fixed to the Reed-Solomon polynomial 0x11D.
// AES uses the polynomial 0x11B. `Field` encapsulates any primitive polynomial
// with its own log/alog tables so that both polynomials can coexist.
//
// Usage:
//
// ```rust
// let aes = Field::new(0x11B);
// assert_eq!(aes.multiply(0x53, 0xCA), 0x01);  // AES field inverses
// ```

/// A GF(2^8) field parameterized by an arbitrary primitive polynomial.
///
/// The module-level functions are fixed to the Reed-Solomon polynomial `0x11D`.
/// `Field` lets you work with any polynomial — most notably `0x11B` for AES.
///
/// Operations use Russian peasant (shift-and-XOR) multiplication. No log/antilog
/// tables are stored. This approach works correctly for any irreducible polynomial
/// regardless of which element is a primitive generator. (Log/antilog tables with
/// g=2 fail for `0x11B` because x is not a primitive element of the AES field;
/// AES uses g=0x03 per FIPS 197 §4.1.)
pub struct Field {
    /// The primitive (irreducible) polynomial used to build this field.
    pub primitive_polynomial: u16,
    /// Low byte of the polynomial, used as the reduction constant in gf_mul.
    reduce: u8,
}

impl Field {
    /// Construct a GF(2^8) field for the given primitive polynomial.
    ///
    /// `primitive_poly` is the degree-8 irreducible polynomial as an integer:
    /// - `0x11D` — Reed-Solomon (same as the module-level default)
    /// - `0x11B` — AES (x^8 + x^4 + x^3 + x + 1)
    pub fn new(primitive_poly: u16) -> Self {
        Self {
            primitive_polynomial: primitive_poly,
            reduce: (primitive_poly & 0xFF) as u8,
        }
    }

    /// Russian peasant multiplication: a * b mod p(x) in GF(2^8).
    fn gf_mul(&self, a: u8, b: u8) -> u8 {
        let mut result: u8 = 0;
        let mut aa = a;
        let mut bb = b;
        for _ in 0..8 {
            if bb & 1 != 0 {
                result ^= aa;
            }
            let hi = aa & 0x80;
            aa <<= 1;
            if hi != 0 {
                aa ^= self.reduce;
            }
            bb >>= 1;
        }
        result
    }

    /// Raise base to exp via repeated squaring.
    fn gf_pow(&self, base: u8, exp: u32) -> u8 {
        if base == 0 {
            return if exp == 0 { 1 } else { 0 };
        }
        if exp == 0 {
            return 1;
        }
        let mut result: u8 = 1;
        let mut b = base;
        let mut e = exp;
        while e > 0 {
            if e & 1 != 0 {
                result = self.gf_mul(result, b);
            }
            b = self.gf_mul(b, b);
            e >>= 1;
        }
        result
    }

    /// Add two field elements: `a XOR b`.
    ///
    /// Addition is polynomial-independent in GF(2^8); included for API symmetry.
    #[inline]
    pub fn add(&self, a: u8, b: u8) -> u8 { a ^ b }

    /// Subtract two field elements: `a XOR b` (same as add).
    #[inline]
    pub fn subtract(&self, a: u8, b: u8) -> u8 { a ^ b }

    /// Multiply two field elements using Russian peasant multiplication.
    pub fn multiply(&self, a: u8, b: u8) -> u8 {
        self.gf_mul(a, b)
    }

    /// Divide `a` by `b` in this field.
    ///
    /// # Panics
    ///
    /// Panics if `b == 0`.
    pub fn divide(&self, a: u8, b: u8) -> u8 {
        assert!(b != 0, "GF256::Field: division by zero");
        self.gf_mul(a, self.gf_pow(b, 254))
    }

    /// Raise `base` to a non-negative integer power.
    pub fn power(&self, base: u8, exp: u32) -> u8 {
        self.gf_pow(base, exp)
    }

    /// Compute the multiplicative inverse of `a`.
    ///
    /// inverse(a) = a^254 since a^255 = 1 in GF(2^8) (Fermat's little theorem).
    ///
    /// # Panics
    ///
    /// Panics if `a == 0`.
    pub fn inverse(&self, a: u8) -> u8 {
        assert!(a != 0, "GF256::Field: zero has no multiplicative inverse");
        self.gf_pow(a, 254)
    }
}
