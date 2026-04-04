// ============================================================================
// PolynomialNative.swift — Swift wrapper over the Rust polynomial-c C ABI.
// ============================================================================
//
// This file bridges the gap between the raw C ABI (pointer + length arrays,
// caller-managed buffers, C int return codes) and idiomatic Swift ([Double]
// arrays, throwing errors, result types).
//
// ## How the C Interop Works
//
// When Swift imports `CPolynomial`, the compiler has access to all the
// function signatures in `polynomial_c.h`. It automatically maps:
//
//   C signature                      → Swift signature
//   ─────────────────────────────────────────────────────
//   size_t poly_c_add(...)           → poly_c_add(...) -> Int
//   const double *a, size_t a_len    → UnsafePointer<Double>!, Int
//   double *out, size_t out_cap      → UnsafeMutablePointer<Double>!, Int
//
// The `withUnsafeBufferPointer` and `withUnsafeMutableBufferPointer` methods
// on Swift arrays give us the pointer + length pairs that C expects, without
// copying data and without heap allocation.
//
// ## Literate Explanation: Why Buffer Pointers?
//
// A Swift `[Double]` is a heap-allocated array. When you pass it to C, you
// have two choices:
//
//  1. **Copy the array** into a C-compatible heap allocation, pass that
//     pointer, then copy the result back. Expensive: two extra allocations
//     and two memcpy calls for every function call.
//
//  2. **Borrow a temporary pointer** to the Swift array's internal storage.
//     Swift guarantees the pointer is valid for the duration of the closure
//     passed to `withUnsafeBufferPointer`. No copying, no extra allocation.
//     This is what we do here.
//
// The output pattern is similar: we pre-allocate a Swift array of worst-case
// size, pass a mutable pointer to its storage, let the Rust code write into
// it, then truncate to the actual length returned.
//
// ## Design: Public Enum Namespace
//
// We use `public enum Polynomial` (an enum with no cases) as a namespace.
// This is a common Swift idiom for grouping related free functions without
// creating an object that can be instantiated. It is cleaner than using
// a struct or module-level free functions because:
//
//   - It prevents `Polynomial()` from being written (no init).
//   - All members are unambiguously scoped: `Polynomial.add(...)`.
//   - Extensions can add functionality in other files.
//
// ============================================================================

import CPolynomial

/// Swift wrapper for polynomial arithmetic, backed by the Rust `polynomial-c`
/// static library via a C ABI bridge.
///
/// Polynomials are represented as `[Double]` where **index = degree**:
///
/// ```swift
/// [3.0, 0.0, 2.0]   // 3 + 0·x + 2·x²  =  3 + 2x²
/// [1.0, 2.0, 3.0]   // 1 + 2x + 3x²
/// []                 // the zero polynomial
/// ```
///
/// All functions call through to the Rust implementation, which:
/// - Strips trailing near-zero coefficients (normalization).
/// - Uses floating-point epsilon `~2.2e-10` as the zero threshold.
/// - Uses Horner's method for evaluation (O(n), no exponentiation).
/// - Uses the classical Euclidean algorithm for GCD.
///
/// ## Prerequisites
///
/// This library requires `libpolynomial_c.a` to be compiled and present at
/// `Sources/CPolynomial/libpolynomial_c.a` before building. See the BUILD
/// file for the two-step compilation process.
public enum Polynomial {

    // =========================================================================
    // MARK: — Fundamentals
    // =========================================================================

    /// Normalize a polynomial by stripping trailing near-zero coefficients.
    ///
    /// Two polynomials that represent the same mathematical object will be
    /// equal after normalization:
    ///
    /// ```swift
    /// Polynomial.normalize([1.0, 0.0, 0.0])  // → [1.0]
    /// Polynomial.normalize([0.0])             // → []
    /// Polynomial.normalize([])                // → []
    /// Polynomial.normalize([1.0, 2.0, 3.0])  // → [1.0, 2.0, 3.0]
    /// ```
    ///
    /// The "near-zero" threshold is `f64::EPSILON * 1e6 ≈ 2.22e-10`, which
    /// absorbs floating-point rounding errors that accumulate during division.
    public static func normalize(_ poly: [Double]) -> [Double] {
        // Allocate a worst-case output buffer (normalization cannot grow).
        var out = [Double](repeating: 0, count: max(poly.count, 1))
        let n = poly.withUnsafeBufferPointer { polyBuf in
            out.withUnsafeMutableBufferPointer { outBuf in
                poly_c_normalize(polyBuf.baseAddress, polyBuf.count,
                                 outBuf.baseAddress, outBuf.count)
            }
        }
        return Array(out.prefix(n))
    }

    /// Return the degree of a polynomial.
    ///
    /// The degree is the index of the highest non-zero coefficient. The zero
    /// polynomial returns 0 by convention.
    ///
    /// ```swift
    /// Polynomial.degree([3.0, 0.0, 2.0])  // → 2
    /// Polynomial.degree([7.0])             // → 0
    /// Polynomial.degree([])                // → 0
    /// ```
    public static func degree(_ poly: [Double]) -> Int {
        poly.withUnsafeBufferPointer { buf in
            poly_c_degree(buf.baseAddress, buf.count)
        }
    }

    /// Evaluate a polynomial at `x` using Horner's method.
    ///
    /// **Horner's method** reformulates `a₀ + a₁x + a₂x² + …` as the nested
    /// form `a₀ + x(a₁ + x(a₂ + … + x·aₙ))`, requiring only n multiplications
    /// and n additions — O(n) total with no exponentiation.
    ///
    /// ```swift
    /// // p(x) = 3 + 2x² evaluated at x = 2: 3 + 2·4 = 11
    /// Polynomial.evaluate([3.0, 0.0, 2.0], at: 2.0)  // → 11.0
    ///
    /// // p(x) = 0 (zero polynomial) evaluates to 0 everywhere
    /// Polynomial.evaluate([], at: 42.0)               // → 0.0
    /// ```
    public static func evaluate(_ poly: [Double], at x: Double) -> Double {
        poly.withUnsafeBufferPointer { buf in
            poly_c_evaluate(buf.baseAddress, buf.count, x)
        }
    }

    // =========================================================================
    // MARK: — Addition and Subtraction
    // =========================================================================

    /// Add two polynomials term-by-term.
    ///
    /// If `a` has degree `m` and `b` has degree `n`, the result has
    /// degree ≤ max(m, n).
    ///
    /// ```swift
    /// // (1 + 2x + 3x²) + (4 + 5x)
    /// Polynomial.add([1.0, 2.0, 3.0], [4.0, 5.0])
    /// // → [5.0, 7.0, 3.0]  =  5 + 7x + 3x²
    /// ```
    public static func add(_ a: [Double], _ b: [Double]) -> [Double] {
        let cap = max(a.count, b.count) + 1
        var out = [Double](repeating: 0, count: cap)
        let n = a.withUnsafeBufferPointer { aBuf in
            b.withUnsafeBufferPointer { bBuf in
                out.withUnsafeMutableBufferPointer { outBuf in
                    poly_c_add(aBuf.baseAddress, aBuf.count,
                               bBuf.baseAddress, bBuf.count,
                               outBuf.baseAddress, outBuf.count)
                }
            }
        }
        return Array(out.prefix(n))
    }

    /// Subtract polynomial `b` from polynomial `a` term-by-term.
    ///
    /// ```swift
    /// // (5 + 7x + 3x²) − (1 + 2x + 3x²)
    /// Polynomial.subtract([5.0, 7.0, 3.0], [1.0, 2.0, 3.0])
    /// // → [4.0, 5.0]  =  4 + 5x  (x² terms cancel)
    /// ```
    public static func subtract(_ a: [Double], _ b: [Double]) -> [Double] {
        let cap = max(a.count, b.count) + 1
        var out = [Double](repeating: 0, count: cap)
        let n = a.withUnsafeBufferPointer { aBuf in
            b.withUnsafeBufferPointer { bBuf in
                out.withUnsafeMutableBufferPointer { outBuf in
                    poly_c_subtract(aBuf.baseAddress, aBuf.count,
                                    bBuf.baseAddress, bBuf.count,
                                    outBuf.baseAddress, outBuf.count)
                }
            }
        }
        return Array(out.prefix(n))
    }

    // =========================================================================
    // MARK: — Multiplication
    // =========================================================================

    /// Multiply two polynomials by polynomial convolution.
    ///
    /// Each term `a[i]·xⁱ` of `a` multiplies each term `b[j]·xʲ` of `b`,
    /// contributing `a[i]·b[j]` to the coefficient at index `i + j`.
    ///
    /// If `a` has degree `m` and `b` has degree `n`, the result has degree
    /// `m + n`. The output array has length `a.count + b.count - 1`.
    ///
    /// ```swift
    /// // (1 + 2x)(3 + 4x) = 3 + 4x + 6x + 8x² = 3 + 10x + 8x²
    /// Polynomial.multiply([1.0, 2.0], [3.0, 4.0])
    /// // → [3.0, 10.0, 8.0]
    /// ```
    public static func multiply(_ a: [Double], _ b: [Double]) -> [Double] {
        // If either input is empty, the product is the zero polynomial.
        if a.isEmpty || b.isEmpty { return [] }
        let cap = a.count + b.count  // worst case: a.count + b.count - 1
        var out = [Double](repeating: 0, count: cap)
        let n = a.withUnsafeBufferPointer { aBuf in
            b.withUnsafeBufferPointer { bBuf in
                out.withUnsafeMutableBufferPointer { outBuf in
                    poly_c_multiply(aBuf.baseAddress, aBuf.count,
                                    bBuf.baseAddress, bBuf.count,
                                    outBuf.baseAddress, outBuf.count)
                }
            }
        }
        return Array(out.prefix(n))
    }

    // =========================================================================
    // MARK: — Division
    // =========================================================================

    /// Perform polynomial long division, returning `(quotient, remainder)`.
    ///
    /// Finds `q` and `r` such that `dividend = divisor × q + r` and
    /// `degree(r) < degree(divisor)`.
    ///
    /// Returns `nil` if the divisor is the zero polynomial (division by zero
    /// is undefined).
    ///
    /// ```swift
    /// // Divide x³ + 3x² + x + 5 by x + 2
    /// // dividend = [5, 1, 3, 2]  =  5 + x + 3x² + 2x³
    /// // divisor  = [2, 1]        =  2 + x
    /// if let (q, r) = Polynomial.divmod([5, 1, 3, 2], [2, 1]) {
    ///     // q → [3, -1, 2]  =  3 − x + 2x²
    ///     // r → [-1]        =  −1
    /// }
    /// ```
    public static func divmod(
        _ dividend: [Double],
        _ divisor: [Double]
    ) -> (quotient: [Double], remainder: [Double])? {
        let quotCap = dividend.count + 1
        let remCap  = divisor.count + 1
        var quotOut = [Double](repeating: 0, count: quotCap)
        var remOut  = [Double](repeating: 0, count: remCap)
        var quotLen: Int = 0
        var remLen:  Int = 0

        let status = dividend.withUnsafeBufferPointer { dBuf in
            divisor.withUnsafeBufferPointer { sBuf in
                quotOut.withUnsafeMutableBufferPointer { qBuf in
                    remOut.withUnsafeMutableBufferPointer { rBuf in
                        withUnsafeMutablePointer(to: &quotLen) { qLenPtr in
                            withUnsafeMutablePointer(to: &remLen) { rLenPtr in
                                poly_c_divmod(
                                    dBuf.baseAddress, dBuf.count,
                                    sBuf.baseAddress, sBuf.count,
                                    qBuf.baseAddress, qBuf.count, qLenPtr,
                                    rBuf.baseAddress, rBuf.count, rLenPtr
                                )
                            }
                        }
                    }
                }
            }
        }

        // poly_c_divmod returns -1 on error (zero divisor).
        guard status == 0 else { return nil }

        return (
            quotient: Array(quotOut.prefix(quotLen)),
            remainder: Array(remOut.prefix(remLen))
        )
    }

    /// Return the quotient of `dividend / divisor`.
    ///
    /// Returns `nil` if the divisor is the zero polynomial.
    ///
    /// ```swift
    /// Polynomial.divide([5, 1, 3, 2], [2, 1])  // → [3, -1, 2]
    /// ```
    public static func divide(_ a: [Double], _ b: [Double]) -> [Double]? {
        divmod(a, b)?.quotient
    }

    /// Return the remainder of `dividend / divisor`.
    ///
    /// Returns `nil` if the divisor is the zero polynomial.
    ///
    /// In GF(2^8) construction, this operation reduces a high-degree polynomial
    /// modulo the primitive polynomial.
    ///
    /// ```swift
    /// Polynomial.modulo([5, 1, 3, 2], [2, 1])  // → [-1.0]
    /// ```
    public static func modulo(_ a: [Double], _ b: [Double]) -> [Double]? {
        divmod(a, b)?.remainder
    }

    // =========================================================================
    // MARK: — Greatest Common Divisor
    // =========================================================================

    /// Compute the GCD of two polynomials using the Euclidean algorithm.
    ///
    /// The GCD is the highest-degree polynomial that divides both inputs with
    /// zero remainder. The algorithm is the direct polynomial analog of the
    /// integer Euclidean algorithm:
    ///
    /// ```text
    /// gcd(a, b):
    ///     while b ≠ zero:
    ///         a, b = b, a mod b
    ///     return a
    /// ```
    ///
    /// ```swift
    /// // gcd(x² − 3x + 2, x − 1) = x − 1  (both are divisible by x − 1)
    /// // x² − 3x + 2 = (x−1)(x−2)
    /// Polynomial.gcd([2.0, -3.0, 1.0], [-1.0, 1.0])
    /// // → [-1.0, 1.0]  = x − 1 (monic up to scalar)
    /// ```
    public static func gcd(_ a: [Double], _ b: [Double]) -> [Double] {
        let cap = max(a.count, b.count) + 1
        var out = [Double](repeating: 0, count: cap)
        let n = a.withUnsafeBufferPointer { aBuf in
            b.withUnsafeBufferPointer { bBuf in
                out.withUnsafeMutableBufferPointer { outBuf in
                    poly_c_gcd(aBuf.baseAddress, aBuf.count,
                               bBuf.baseAddress, bBuf.count,
                               outBuf.baseAddress, outBuf.count)
                }
            }
        }
        return Array(out.prefix(n))
    }
}
