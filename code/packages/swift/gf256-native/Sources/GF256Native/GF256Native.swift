// ============================================================================
// GF256Native.swift — Swift wrapper over the Rust gf256-c C ABI.
// ============================================================================
//
// This file provides a clean Swift API over the raw C functions exposed by
// the `gf256-c` Rust crate. The C functions are imported via the `CGF256`
// module (defined by the module.modulemap file).
//
// ## GF(256) in One Paragraph
//
// GF(2^8) — "Galois Field of 256 elements" — is a finite field where the
// 256 elements are bytes (0–255). Arithmetic is NOT normal integer arithmetic:
//
//   - **Add / Subtract**: bitwise XOR. In characteristic-2 fields, -1 = 1,
//     so subtraction equals addition. `3 + 3 = 0`, `5 - 5 = 0`.
//   - **Multiply**: using precomputed log/antilog tables. `a × b` never
//     overflows; it is always another byte in [0, 255].
//   - **Divide**: `a / b = ALOG[(LOG[a] - LOG[b] + 255) % 255]`.
//   - **Power**: `base^exp = ALOG[(LOG[base] * exp) % 255]`.
//   - **Inverse**: `a^{-1} = ALOG[255 - LOG[a]]`.
//
// This arithmetic is the mathematical foundation of Reed-Solomon error
// correction, QR codes, and parts of AES encryption.
//
// ## Error Handling Pattern
//
// The Rust layer catches panics from undefined operations (divide by zero,
// inverse of zero) and signals them via a per-thread error flag. The Swift
// wrapper checks this flag and converts errors to Swift's `Optional` type:
//
//   GF256Native.divide(42, 0)    // → nil  (division by zero)
//   GF256Native.inverse(0)        // → nil  (no inverse for zero)
//
// Callers can pattern match on `nil` to handle errors idiomatically.
//
// ## Prerequisites
//
// This library requires `libgf256_c.a` to be compiled and present at
// `Sources/CGF256/libgf256_c.a` before building. See the BUILD file.
//
// ============================================================================

import CGF256

/// Swift wrapper for GF(2^8) arithmetic, backed by the Rust `gf256-c`
/// static library via a C ABI bridge.
///
/// All operations are O(1) using precomputed log/antilog tables. The tables
/// are initialized on the first call and cached for subsequent calls (this
/// initialization happens in the Rust layer via `std::sync::OnceLock`).
///
/// ## Error Values
///
/// Operations that are mathematically undefined (divide by zero, inverse of
/// zero) return `nil`. This is idiomatic Swift: callers use `if let` or
/// `guard let` to handle potential errors.
///
/// ## GF(256) Properties
///
/// The field uses the primitive polynomial `x^8 + x^4 + x^3 + x^2 + 1`
/// (decimal 285, hex 0x11D). This polynomial is:
/// - **Irreducible**: cannot be factored over GF(2).
/// - **Primitive**: the element `g = 2` generates all 255 non-zero elements.
///
/// The generator property means `{ 2^0, 2^1, …, 2^254 }` cycles through
/// all 255 non-zero elements, which is why the log/antilog approach works.
public enum GF256Native {

    // =========================================================================
    // MARK: — Field Operations
    // =========================================================================

    /// Add two GF(256) elements.
    ///
    /// In GF(2^8), addition is bitwise XOR. This follows from the fact that
    /// each byte represents a polynomial over GF(2) (the field {0, 1}), where
    /// coefficient addition is `1 + 1 = 0 (mod 2)` — which is XOR.
    ///
    /// No error cases: addition is defined for all 256 × 256 input pairs.
    ///
    /// ```swift
    /// GF256Native.add(0x53, 0xCA)  // → 0x99 = 0x53 XOR 0xCA
    /// GF256Native.add(7, 7)        // → 0  (every element is its own additive inverse)
    /// GF256Native.add(0, x)        // → x  (0 is the additive identity)
    /// ```
    public static func add(_ a: UInt8, _ b: UInt8) -> UInt8 {
        gf256_c_add(a, b)
    }

    /// Subtract two GF(256) elements.
    ///
    /// In characteristic-2 fields, `-x = x` for all `x` (because `x + x = 0`).
    /// Therefore `a - b = a + (-b) = a + b = a XOR b`.
    ///
    /// This is identical to `add` and is provided for semantic clarity when
    /// expressing algorithms that use subtraction conceptually.
    ///
    /// ```swift
    /// GF256Native.subtract(0x99, 0xCA)  // → 0x53  (same as add)
    /// GF256Native.subtract(x, x)        // → 0     (for all x)
    /// ```
    public static func subtract(_ a: UInt8, _ b: UInt8) -> UInt8 {
        gf256_c_subtract(a, b)
    }

    /// Multiply two GF(256) elements using log/antilog tables.
    ///
    /// The algorithm: `a × b = ALOG[(LOG[a] + LOG[b]) % 255]`
    ///
    /// This converts the multiplication problem into an addition in the
    /// exponent space, which is just one table lookup + one addition +
    /// one table lookup — O(1) and branch-free (except for the zero check).
    ///
    /// ```swift
    /// GF256Native.multiply(2, 4)    // → 8   (no overflow)
    /// GF256Native.multiply(2, 128)  // → 29  (overflow → reduced mod 0x11D)
    /// GF256Native.multiply(0, 255)  // → 0   (zero annihilates)
    /// GF256Native.multiply(1, x)    // → x   (1 is the multiplicative identity)
    /// ```
    public static func multiply(_ a: UInt8, _ b: UInt8) -> UInt8 {
        gf256_c_multiply(a, b)
    }

    /// Divide `a` by `b` in GF(256).
    ///
    /// Returns `nil` if `b == 0` (division by zero is undefined).
    ///
    /// The algorithm: `a / b = ALOG[(LOG[a] - LOG[b] + 255) % 255]`
    ///
    /// The `+ 255` before `% 255` ensures the exponent stays non-negative
    /// when `LOG[a] < LOG[b]` — the same trick as computing modular inverse
    /// with unsigned arithmetic.
    ///
    /// ```swift
    /// GF256Native.divide(10, 2)   // → 5   (10 ÷ 2 in GF256)
    /// GF256Native.divide(0, 7)    // → 0   (zero divided by anything is zero)
    /// GF256Native.divide(42, 0)   // → nil (division by zero)
    /// ```
    public static func divide(_ a: UInt8, _ b: UInt8) -> UInt8? {
        let result = gf256_c_divide(a, b)
        return gf256_c_had_error() != 0 ? nil : result
    }

    /// Raise a GF(256) element to a non-negative integer power.
    ///
    /// The algorithm: `base^exp = ALOG[(LOG[base] * exp) % 255]`
    ///
    /// The multiplicative group of GF(256) has order 255, so exponents repeat
    /// with period 255: `g^255 = g^0 = 1` (analogous to Fermat's little theorem).
    ///
    /// Special cases follow mathematical convention:
    /// - `0^0 = 1` (convention used by most math libraries)
    /// - `0^exp = 0` for exp > 0
    /// - `base^0 = 1` for any non-zero base
    ///
    /// ```swift
    /// GF256Native.power(2, 0)    // → 1    (2^0 = 1)
    /// GF256Native.power(2, 1)    // → 2    (2^1 = 2)
    /// GF256Native.power(2, 8)    // → 29   (2^8 mod 0x11D = 29)
    /// GF256Native.power(2, 255)  // → 1    (generator cycles back to 1)
    /// GF256Native.power(0, 5)    // → 0    (0 to any positive power)
    /// ```
    public static func power(_ base: UInt8, _ exp: UInt32) -> UInt8 {
        gf256_c_power(base, exp)
    }

    /// Compute the multiplicative inverse of a GF(256) element.
    ///
    /// Returns `a^{-1}` such that `a × a^{-1} = 1`.
    /// Returns `nil` if `a == 0` (zero has no multiplicative inverse).
    ///
    /// The algorithm: `a^{-1} = ALOG[255 - LOG[a]]`
    ///
    /// Derivation: since `a × a^{-1} = 1 = g^0 = g^255` (the group wraps),
    /// we need `LOG[a] + LOG[a^{-1}] ≡ 0 (mod 255)`, so
    /// `LOG[a^{-1}] = 255 - LOG[a]`.
    ///
    /// ```swift
    /// GF256Native.inverse(1)    // → 1   (1 is its own inverse)
    /// GF256Native.inverse(2)    // → 142 (2 × 142 = 1 in GF256)
    /// GF256Native.inverse(0)    // → nil (0 has no inverse)
    ///
    /// // Verify: a × inverse(a) = 1 for all non-zero a
    /// if let inv = GF256Native.inverse(53) {
    ///     GF256Native.multiply(53, inv)  // → 1
    /// }
    /// ```
    public static func inverse(_ a: UInt8) -> UInt8? {
        let result = gf256_c_inverse(a)
        return gf256_c_had_error() != 0 ? nil : result
    }

    // =========================================================================
    // MARK: — Constants
    // =========================================================================

    /// The additive identity element.
    ///
    /// `add(zero, x) == x` for all `x`.
    public static let zero: UInt8 = 0

    /// The multiplicative identity element.
    ///
    /// `multiply(one, x) == x` for all `x`.
    public static let one: UInt8 = 1

    /// The primitive polynomial used to define this GF(256) field.
    ///
    /// `x^8 + x^4 + x^3 + x^2 + 1 = 285 = 0x11D`
    ///
    /// This polynomial determines the structure of the entire field. A
    /// different primitive polynomial would give a different (isomorphic
    /// but not identical) field. AES, for example, uses `0x11B` instead
    /// of the `0x11D` used here.
    public static var primitivePolynomial: UInt32 {
        gf256_c_primitive_polynomial()
    }
}
