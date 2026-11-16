// ============================================================================
// Trig.swift — Trigonometric Functions from First Principles (TRIG-REF-1)
// ============================================================================
//
// This module implements sin, cos, tan, atan, atan2, sqrt, and angle
// conversion functions using ONLY basic arithmetic — no Foundation, no Darwin,
// no Swift.Numerics, no standard-library math functions.
//
// `sin`, `cos`, `radians` and `degrees` follow the reference algorithm
// TRIG-REF-1 of code/specs/PHY00a-trig-convergence.md, and return exactly the
// same BITS as the other sixteen language ports (checked against the shared
// vectors in code/specs/fixtures/trig-reference-v1 by ConformanceTests).
//
// Everything is derived from:
//   - Cody–Waite range reduction + Maclaurin series (Horner form) for sin/cos
//   - Newton's (Babylonian) method for sqrt
//   - Taylor series + range reduction + half-angle identity for atan
//   - Quadrant logic for atan2
//
// Why build trig from scratch?
// ----------------------------
// When you call Swift's `.sin()` or Darwin's `sin()`, the C library evaluates
// a polynomial approximation — the same Taylor series we implement here. By
// writing it ourselves we learn:
//
//   1. How polynomials approximate transcendental functions.
//   2. Why "range reduction" is critical for numerical stability.
//   3. How Newton's method achieves quadratic convergence for square roots.
//   4. The geometry behind atan2's four-quadrant logic.
//
// The arithmetic model (PHY00a 3.1)
// ---------------------------------
// Every value is an IEEE 754 binary64 `Double`.  Every operation rounds ONCE,
// to nearest-even, and nothing is fused: a product that feeds an addition is
// always written as its own statement,
//
//     let m = k * PIO2_1      // one rounding
//     r -= m                  // another rounding
//
// never as `r.addingProduct(-k, PIO2_1)` (a fused multiply-add rounds once and
// would change the last bit).  Swift's compiler does not contract `a*b + c` on
// its own, so the separate statements are what keep the answer identical on
// x86-64 and arm64; this file never calls `addingProduct` or `.squareRoot()`,
// and must never be built with `-Ounchecked`.  Constants are built from their
// IEEE bit patterns with `Double(bitPattern:)`, so no decimal parser is involved.
//
// Layer: PHY00 (physics, leaf package — zero dependencies beyond Swift stdlib)
// ============================================================================

// ----------------------------------------------------------------------------
// Public Constants
// ----------------------------------------------------------------------------

/// PI — the ratio of a circle's circumference to its diameter.
///
/// Hard-coded to the full precision of a Double (IEEE 754 binary64), bits
/// `0x400921FB54442D18`.  This is the same value as `Double.pi`, but we define
/// it ourselves to make the module fully self-contained.
public let PI: Double = 3.141592653589793

/// Two times PI, one full revolution (`0x401921FB54442D18`).
let TWO_PI: Double = Double(bitPattern: 0x401921FB_54442D18)

/// PI / 2 (`0x3FF921FB54442D18`). Used by atan and the atan2 quadrant logic.
let HALF_PI: Double = Double(bitPattern: 0x3FF921FB_54442D18)

// ----------------------------------------------------------------------------
// Reduction constants (PHY00a 3.2) — bit patterns are authoritative
// ----------------------------------------------------------------------------
//
// To find "how many quarter-turns fit in x" we multiply by 2/pi, and to find
// what is left over we subtract k * (pi/2).  pi/2 is irrational, so a single
// 53-bit pi/2 times a big k throws away the very digits we need.  The
// Cody–Waite trick (William J. Cody and William Waite, "Software Manual for the
// Elementary Functions", 1980) stores pi/2 as a SUM of five pieces.  The first
// four have at most 24 significant bits, so k * piece needs at most 24 + 29 = 53
// bits and is EXACT for every whole k below 2^29.  The fifth is the remainder
// rounded to 53 bits.  Together they carry pi/2 to about 149 bits.

/// 2/pi, the double nearest to it (`0.6366197723675814`).
let INV_PIO2: Double = Double(bitPattern: 0x3FE45F30_6DC9C883)

/// pi/2 as five pieces, most significant first (PIO2_1 ... PIO2_5).
let PIO2: [Double] = [
    Double(bitPattern: 0x3FF921FB_40000000),  // PIO2_1  1.570796251296997
    Double(bitPattern: 0x3E74442D_00000000),  // PIO2_2  7.549789415861596e-08
    Double(bitPattern: 0x3CF84698_80000000),  // PIO2_3  5.390302529957765e-15
    Double(bitPattern: 0x3B78CC51_60000000),  // PIO2_4  3.282003415807913e-22
    Double(bitPattern: 0x39F01B83_9A25204A),  // PIO2_5  1.2706558760139879e-29
]

/// 1.5 * 2^52.  Adding it to t (|t| < 2^51) pushes the fraction bits off the end
/// of the significand, so `(t + MAGIC) - MAGIC` is t rounded to the nearest
/// whole number (ties to even) using only an add and a subtract.
let MAGIC: Double = Double(bitPattern: 0x43380000_00000000)

/// The edge of the domain, 2^29.  Beyond it k no longer fits the exact
/// products above, so sin, cos and tan return NaN (PHY00a 3.4.1).
let LIMIT: Double = Double(bitPattern: 0x41C00000_00000000)

/// 2^-27.  Below it sin(x) == x exactly and cos(x) == 1 (PHY00a 3.3).  It is
/// also the threshold below which `atan(x) == x`.
let TINY: Double = Double(bitPattern: 0x3E400000_00000000)

/// pi/180 and 180/pi, pre-divided so a conversion is ONE multiply (one rounding).
let RAD_PER_DEG: Double = Double(bitPattern: 0x3F91DF46_A2529D39)
let DEG_PER_RAD: Double = Double(bitPattern: 0x404CA5DC_1A63C1F8)

// ----------------------------------------------------------------------------
// Series coefficients (PHY00a 3.2) — the factorial table
// ----------------------------------------------------------------------------
//
//   S[n] = (-1)^n / (2n+1)!    n = 1...9      (sine,   x^3 ... x^19)
//   C[n] = (-1)^n / (2n)!      n = 2...9      (cosine, x^4 ... x^18)
//
// each rounded ONCE to the nearest double from the exact fraction.  (The n = 1
// cosine coefficient, -1/2, is applied as `0.5 * z`, which is exact.)  The
// arrays are indexed from 0, so SIN_COEFFS[0] is S[1] and COS_COEFFS[0] is C[2].
//
//        n :   1      2      3       4       5        6         7          8           9
//   S[n]   : -1/3!  +1/5!  -1/7!   +1/9!   -1/11!   +1/13!    -1/15!     +1/17!      -1/19!
//   C[n]   :   -    +1/4!  -1/6!   +1/8!   -1/10!   +1/12!    -1/14!     +1/16!      -1/18!
//
// Transcription check: where 2n+2 is a power of two the factorials differ by it,
// so S[3]/C[4] (1/7!, 1/8!) share the digits ...A01A01A01A01A and S[7]/C[8]
// (1/15!, 1/16!) share ...AE7F3E733B81F.

let SIN_COEFFS: [Double] = [
    Double(bitPattern: 0xBFC55555_55555555),  // S1 = -1/3!
    Double(bitPattern: 0x3F811111_11111111),  // S2 = +1/5!
    Double(bitPattern: 0xBF2A01A0_1A01A01A),  // S3 = -1/7!
    Double(bitPattern: 0x3EC71DE3_A556C734),  // S4 = +1/9!
    Double(bitPattern: 0xBE5AE645_67F544E4),  // S5 = -1/11!
    Double(bitPattern: 0x3DE61246_13A86D09),  // S6 = +1/13!
    Double(bitPattern: 0xBD6AE7F3_E733B81F),  // S7 = -1/15!
    Double(bitPattern: 0x3CE952C7_7030AD4A),  // S8 = +1/17!
    Double(bitPattern: 0xBC62F49B_46814157),  // S9 = -1/19!
]

let COS_COEFFS: [Double] = [
    Double(bitPattern: 0x3FA55555_55555555),  // C2 = +1/4!
    Double(bitPattern: 0xBF56C16C_16C16C17),  // C3 = -1/6!
    Double(bitPattern: 0x3EFA01A0_1A01A01A),  // C4 = +1/8!
    Double(bitPattern: 0xBE927E4F_B7789F5C),  // C5 = -1/10!
    Double(bitPattern: 0x3E21EED8_EFF8D898),  // C6 = +1/12!
    Double(bitPattern: 0xBDA93974_A8C07C9D),  // C7 = -1/14!
    Double(bitPattern: 0x3D2AE7F3_E733B81F),  // C8 = +1/16!
    Double(bitPattern: 0xBCA68278_63B97D97),  // C9 = -1/18!
]

// ----------------------------------------------------------------------------
// Errors
// ----------------------------------------------------------------------------

/// Error thrown by the non-trapping `Trig.sqrtChecked(_:)`.
public enum TrigError: Error, Equatable {
    /// The radicand was negative (including -infinity).  `Trig.sqrt` traps on
    /// exactly these inputs.
    case negativeSquareRoot
}

// ----------------------------------------------------------------------------
// Public API — wrapped in a struct with static methods
// ----------------------------------------------------------------------------
//
// Using a struct with `static func` methods is idiomatic Swift for a
// namespace of pure mathematical functions. Callers write:
//
//   let s = Trig.sin(1.0)
//   let r = Trig.sqrt(2.0)

/// A namespace for trigonometric functions computed from first principles.
public struct Trig {

    // Private initialiser — prevents instantiation. This is a pure namespace.
    private init() {}

    // -------------------------------------------------------------------------
    // Range Reduction (PHY00a 3.4)
    // -------------------------------------------------------------------------
    //
    // A sine is a repeating wave: knowing sin on [-pi/4, pi/4] and which
    // quarter-turn you are in is enough to know it everywhere.  Reducing answers
    // two questions: how many quarter-turns fit in x (a whole number k), and what
    // is left over (r = x - k*pi/2, with |r| <= pi/4)?
    //
    // Step 1 — the quotient, with no floor, round or integer cast:
    //
    //     t = x * INV_PIO2                  one rounding; x / (pi/2), roughly
    //     k = (t + MAGIC) - MAGIC           t rounded to the nearest whole number
    //
    // Step 2 — the remainder, by Cody–Waite subtraction of five pieces of pi/2:
    //
    //     x ---------------------------------------------------------- the angle
    //     |-- k*PIO2_1 --|                                             24 bits
    //                    |-- k*PIO2_2 --|                              next 24
    //                                   |-- k*PIO2_3 --|               next 24
    //                                                  |- k*PIO2_4 -|  next 24
    //                                                               |k*PIO2_5|  rounds
    //     r = what is left, |r| <= pi/4 (plus a few ulps: t was rounded first)
    //
    // The first two subtractions are exact (Sterbenz); the last three round,
    // and that rounding is the biggest term in the error budget (about 1 ulp of
    // r).  When x is very near a multiple of pi/2 the partial results cancel
    // rather than round, which is why the relative error of r stays small.
    //
    // Precondition: |x| <= 2^29 (the callers have already checked), so k is a
    // finite whole number below 2^29 in magnitude.
    //
    // FUSION: every product below is its own statement (`m = k * piece`, then
    // `r -= m`).  Do not rewrite them as addingProduct.

    private static func reduce(_ x: Double) -> (k: Double, r: Double) {
        let t = x * INV_PIO2
        let k = (t + MAGIC) - MAGIC

        var r = x
        for piece in PIO2 {
            let m = k * piece      // exact for the first four pieces
            r -= m
        }
        return (k, r)
    }

    // -------------------------------------------------------------------------
    // The series kernels (PHY00a 3.5)
    // -------------------------------------------------------------------------
    //
    // On |r| <= pi/4 sin and cos are polynomials in z = r*r:
    //
    //   sin(r) = r + r*z*(S1 + S2 z + S3 z^2 + ... + S9 z^8)      ten terms
    //   cos(r) = 1 - z/2 + z^2*(C2 + C3 z + ... + C9 z^7)         ten terms
    //
    // evaluated by Horner's rule, innermost coefficient first, one multiply and
    // one add per step.  Horner adds the smallest terms first (the accurate
    // order), needs no division at all, and leaves no choice of grouping, so
    // every language gets the same bits.  The leading term (r, or 1 - z/2) is
    // added LAST and unrounded inside the polynomial, so the dominant part of
    // the result carries a single rounding.

    /// sin(r) for |r| <= pi/4.
    private static func sinKernel(_ r: Double) -> Double {
        let z = r * r
        var p = SIN_COEFFS[8]                  // S9
        for n in stride(from: 7, through: 0, by: -1) {
            p = p * z
            p = p + SIN_COEFFS[n]              // S8 ... S1
        }
        var w = r * z
        w = w * p
        return r + w
    }

    /// cos(r) for |r| <= pi/4.
    private static func cosKernel(_ r: Double) -> Double {
        let z = r * r
        var q = COS_COEFFS[7]                  // C9
        for n in stride(from: 6, through: 0, by: -1) {
            q = q * z
            q = q + COS_COEFFS[n]              // C8 ... C2
        }
        var u = 0.5 * z                        // exact
        u = 1.0 - u
        var v = z * z
        v = v * q
        return u + v
    }

    // =========================================================================
    // sin — The Sine Function
    // =========================================================================
    //
    // The Maclaurin series for sine:
    //
    //   sin(x) = x - x³/3! + x⁵/5! - x⁷/7! + ...
    //          = Σ (-1)ⁿ · x^(2n+1) / (2n+1)!    for n = 0, 1, 2, ...
    //
    // We reduce x to r with |r| <= pi/4 and a quadrant q = k mod 4, then use
    //
    //   sin(r + k·pi/2) = { sin r,  cos r, -sin r, -cos r }[q]
    //
    // `Int(k) & 3` is exact (k is a whole number below 2^29) and is the
    // floor-modulus 4 even for negative k, because the integer is two's
    // complement: -1 & 3 == 3.
    //
    // Special values (PHY00a 3.3):
    //
    //   NaN, ±inf, |x| > 2^29   -> NaN  (outside the domain)
    //   ±0                      -> ±0   (sin(-0) is -0)
    //   0 < |x| < 2^-27         -> x
    //
    // The guard is a NEGATED `<=`.  A NaN fails every comparison, so
    // `!(|x| <= LIMIT)` is true for NaN, for both infinities and for every
    // finite value beyond the domain: one test covers all three.  Rewriting it
    // as `|x| > LIMIT` would let a NaN through to `Int(k)`, which traps.

    /// Compute the sine of `x` (in radians), bit-identical to TRIG-REF-1.
    ///
    /// - Parameter x: angle in radians
    /// - Returns: sin(x), within 3 ulp for |x| <= 2^29; NaN for NaN, the
    ///   infinities and |x| > 2^29 (outside the domain).
    public static func sin(_ x: Double) -> Double {
        let a = Swift.abs(x)
        if !(a <= LIMIT) { return Double.nan }
        if a < TINY { return x }               // keeps -0 and subnormals

        let (k, r) = reduce(x)
        switch Int(k) & 3 {
        case 0:  return sinKernel(r)
        case 1:  return cosKernel(r)
        case 2:  return -sinKernel(r)
        default: return -cosKernel(r)
        }
    }

    // =========================================================================
    // cos — The Cosine Function
    // =========================================================================
    //
    // The Maclaurin series for cosine uses even powers:
    //
    //   cos(x) = 1 - x²/2! + x⁴/4! - x⁶/6! + ...
    //          = Σ (-1)ⁿ · x^(2n) / (2n)!    for n = 0, 1, 2, ...
    //
    // With the same reduction:
    //
    //   cos(r + k·pi/2) = { cos r, -sin r, -cos r, sin r }[q]
    //
    // cos(x) == 1 for |x| < 2^-27 (the first correction, x²/2, rounds away).

    /// Compute the cosine of `x` (in radians), bit-identical to TRIG-REF-1.
    ///
    /// - Parameter x: angle in radians
    /// - Returns: cos(x), within 3 ulp for |x| <= 2^29; NaN outside the domain.
    public static func cos(_ x: Double) -> Double {
        let a = Swift.abs(x)
        if !(a <= LIMIT) { return Double.nan }
        if a < TINY { return 1.0 }

        let (k, r) = reduce(x)
        switch Int(k) & 3 {
        case 0:  return cosKernel(r)
        case 1:  return -sinKernel(r)
        case 2:  return -cosKernel(r)
        default: return sinKernel(r)
        }
    }

    // =========================================================================
    // sqrt — Newton's (Babylonian) Method
    // =========================================================================
    //
    // The square root algorithm used here has been known since Babylonian times
    // (~1700 BCE). The key insight: if `guess` approximates sqrt(x), then the
    // average of `guess` and `x / guess` is a better approximation.
    //
    // Why? If guess < sqrt(x), then x/guess > sqrt(x). Their average "squeezes"
    // inward from both sides. If guess > sqrt(x), the argument is symmetric.
    //
    // This has *quadratic convergence*: the number of correct digits doubles
    // each iteration. Convergence for sqrt(2):
    //
    //   iter | guess              | correct digits
    //   -----|--------------------|---------------
    //   0    | 2.000000           | 0
    //   1    | 1.500000           | 1
    //   2    | 1.416667           | 2
    //   3    | 1.414216           | 5
    //   4    | 1.41421356237...   | 11+ (full Double precision)
    //
    // Typically converges in 10–15 iterations for any normal Double input.

    /// Compute the square root of `x` using Newton's iterative method.
    ///
    /// - Parameter x: the radicand (must be ≥ 0)
    /// - Returns: √x to full Double precision
    /// - Precondition: x ≥ 0. A negative input triggers a `fatalError`.
    public static func sqrt(_ x: Double) -> Double {
        if x < 0 {
            preconditionFailure("Trig.sqrt: domain error — input \(x) is negative")
        }
        return sqrtNonNegative(x)
    }

    /// Non-trapping twin of `sqrt(_:)`: throws `TrigError.negativeSquareRoot`
    /// for exactly the inputs where `sqrt(_:)` traps (every negative number,
    /// including -infinity), and otherwise returns the same bits as `sqrt(_:)`.
    ///
    /// A `preconditionFailure` cannot be caught, so a test (or a caller that
    /// wants an error rather than a crash) cannot exercise the negative case
    /// through `sqrt(_:)`.  The shared TRIG-REF-1 vectors list those inputs with
    /// `expect = error`; the conformance test asserts them against this entry
    /// point (PHY00a 3.9.2, survey 8.1.5).
    public static func sqrtChecked(_ x: Double) throws -> Double {
        if x < 0 { throw TrigError.negativeSquareRoot }
        return sqrtNonNegative(x)
    }

    /// The Newton iteration itself, for x >= 0 or NaN.
    private static func sqrtNonNegative(_ x: Double) -> Double {

        // sqrt(0) = 0 exactly.
        if x == 0.0 { return x }
        if x.isInfinite { return x }

        var scaled = x
        var resultScale = 1.0
        while scaled < 0.25 {
            scaled *= 4.0
            resultScale *= 0.5
        }
        while scaled >= 4.0 {
            scaled *= 0.25
            resultScale *= 2.0
        }

        // Initial guess: x itself for x ≥ 1 (saves iterations for large values),
        // 1.0 for x in (0, 1) (avoids dividing by a tiny number on the first step).
        var guess = scaled >= 1.0 ? scaled : 1.0

        // Iterate up to 60 times. Quadratic convergence means ~15 in practice.
        for _ in 0..<60 {
            let next = (guess + scaled / guess) / 2.0

            // Convergence criterion: stop when improvement is negligibly small.
            // 1e-15 * guess handles relative precision for large values.
            // 1e-300 is an absolute floor for subnormal (near-zero) inputs.
            if Swift.abs(next - guess) < 1e-15 * guess + 1e-300 {
                return next * resultScale
            }

            guess = next
        }

        return guess * resultScale
    }

    // =========================================================================
    // tan — Tangent as Sine / Cosine
    // =========================================================================
    //
    // Tangent is defined as the ratio of sine to cosine:
    //
    //   tan(x) = sin(x) / cos(x)
    //
    // Geometric interpretation: on the unit circle, the tangent of angle x is
    // the y-coordinate where the ray at angle x intersects the vertical tangent
    // line drawn at (1, 0). This is literally where the name "tangent" comes from.
    //
    // Undefined points (poles):
    //   tan is undefined at x = π/2 + k·π, where cos(x) = 0.
    //   We guard against |cos(x)| < 1e-15 and return ±1e308 (a documented
    //   sentinel, PHY00 3.6) to indicate near-singularity without a runtime
    //   crash.  Outside the sin/cos domain both are NaN, the guard is false
    //   (a NaN comparison is false) and NaN / NaN is NaN.

    /// Compute the tangent of `x` (in radians).
    ///
    /// Uses our own `sin(_:)` and `cos(_:)` — no Foundation math.
    ///
    /// - Parameter x: angle in radians
    /// - Returns: tan(x). Near poles (x ≈ π/2 + k·π), returns ±1e308 (a pinned
    ///   legacy sentinel, not an approximation; PHY00a 3.8). NaN outside the
    ///   sin/cos domain |x| <= 2^29. No accuracy claim is made for tan.
    public static func tan(_ x: Double) -> Double {
        let s = Trig.sin(x)   // our own sin — no Darwin.sin
        let c = Trig.cos(x)   // our own cos — no Darwin.cos

        // Guard against poles.
        if Swift.abs(c) < 1e-15 {
            return s > 0 ? 1.0e308 : -1.0e308
        }

        return s / c
    }

    // =========================================================================
    // atan — Arctangent via Taylor Series with Half-Angle Reduction
    // =========================================================================
    //
    // The Taylor series for atan:
    //
    //   atan(x) = x - x³/3 + x⁵/5 - x⁷/7 + ...   (for |x| ≤ 1)
    //
    // This converges only for |x| ≤ 1. Two layers of range reduction apply:
    //
    // Layer 1 — for |x| > 1 (outer range reduction):
    //   atan(x)  = π/2 - atan(1/x)    for x > 1
    //   atan(x)  = -π/2 - atan(1/x)   for x < -1
    //
    //   Proof: if θ = atan(x), then tan(π/2 - θ) = cot(θ) = 1/x,
    //   so atan(1/x) = π/2 - θ.
    //
    // Layer 2 — half-angle reduction (inside atanCore):
    //   atan(x) = 2·atan( x / (1 + sqrt(1 + x²)) )
    //
    //   This shrinks |x| ≤ 1 to |y| ≤ tan(π/8) ≈ 0.414. At this size the
    //   Taylor series converges in ~15 terms with 17-digit accuracy.

    /// Inner computation of atan for |x| ≤ 1, using half-angle + Taylor series.
    private static func atanCore(_ x: Double) -> Double {
        // Half-angle reduction: atan(x) = 2·atan( x / (1 + sqrt(1 + x²)) ).
        // We use our own sqrt here — no Darwin.sqrt.
        let reduced = x / (1.0 + Trig.sqrt(1.0 + x * x))

        // Taylor series: atan(t) = t - t³/3 + t⁵/5 - ...
        // Iterative form: term_n = term_{n-1} · (-t²) · (2n-1)/(2n+1)
        let t    = reduced
        let tSq  = t * t
        var term = t
        var result = t

        for n in 1...30 {
            // The ratio (2n-1)/(2n+1) steps through consecutive odd denominators:
            //   1/1, 1/3, 1/5, 1/7, ...
            // Combined with -t² to give the alternating series.
            term = term * (-tSq) * Double(2 * n - 1) / Double(2 * n + 1)
            result += term

            // Early exit when the term is negligibly small.
            if Swift.abs(term) < 1e-17 { break }
        }

        // Undo the half-angle halving: atan(x) = 2·atan(reduced).
        return 2.0 * result
    }

    /// Compute the arctangent of `x` (in radians).
    ///
    /// Returns a value in the open interval (-π/2, π/2).
    ///
    /// - Parameter x: any real number
    /// - Returns: atan(x), accurate to ~1e-15
    public static func atan(_ x: Double) -> Double {
        // atan(x) rounds exactly to x here; avoid halving subnormals and retain -0.
        if Swift.abs(x) <= 7.450580596923828e-9 { return x }

        if x > 1.0  { return HALF_PI - atanCore(1.0 / x) }
        if x < -1.0 { return -HALF_PI - atanCore(1.0 / x) }

        return atanCore(x)
    }

    // =========================================================================
    // atan2 — Four-Quadrant Arctangent
    // =========================================================================
    //
    // atan2(y, x) returns the angle in (-π, π] that the point (x, y) makes
    // with the positive x-axis. Unlike atan(y/x), it correctly handles all
    // four quadrants by inspecting the signs of y and x separately.
    //
    // Why atan(y/x) is insufficient:
    //   atan(-1/1) = -π/4       — Q4, correct
    //   atan(-1/-1) = atan(1) = π/4  — but (-1,-1) is in Q3 (should be -3π/4!)
    //
    // Quadrant diagram:
    //
    //          y > 0
    //      Q2  |  Q1        atan2 > 0 in Q1 and Q2
    //    ------+------  x   atan2 < 0 in Q3 and Q4
    //      Q3  |  Q4        atan2 = ±π on negative x-axis
    //          y < 0

    /// Compute the four-quadrant arctangent of (`y`, `x`).
    ///
    /// Returns the angle in radians in the range (-π, π].
    ///
    /// - Parameters:
    ///   - y: y-coordinate of the point
    ///   - x: x-coordinate of the point
    /// - Returns: angle in (-π, π]
    public static func atan2(_ y: Double, _ x: Double) -> Double {
        // A NaN fails every comparison below and would fall through to the
        // "both zero" branch and return 0.0, a finite answer to an undefined
        // question.  TRIG-REF-1 adds this first line (PHY00a 3.9).
        if x.isNaN || y.isNaN { return Double.nan }

        if x > 0.0 {
            return atan(y / x)                          // Q1 or Q4
        } else if x < 0.0 && y >= 0.0 {
            return atan(y / x) + PI                     // Q2 (or negative x-axis with y=0 → π)
        } else if x < 0.0 && y < 0.0 {
            return atan(y / x) - PI                     // Q3
        } else if x == 0.0 && y > 0.0 {
            return HALF_PI                              // Positive y-axis
        } else if x == 0.0 && y < 0.0 {
            return -HALF_PI                             // Negative y-axis
        } else {
            return 0.0                                  // Both zero: undefined → 0 by convention
        }
    }

    // =========================================================================
    // radians / degrees — Angle Unit Conversion
    // =========================================================================
    //
    // Degrees and radians measure the same thing — how far around a circle —
    // but in different units:
    //
    //   Degrees: 360 per full circle (from Babylonian base-60 astronomy)
    //   Radians: 2π per full circle  (natural for calculus: arc = radius × angle)
    //
    // Conversion:
    //   radians = degrees × (π / 180)
    //   degrees = radians × (180 / π)
    //
    // The factors are pre-divided and stored as bit patterns (RAD_PER_DEG,
    // DEG_PER_RAD), so each conversion is ONE multiply and ONE rounding
    // (PHY00a 3.7): no intermediate d*PI that could overflow, 0.65 / 0.81 ulp
    // worst case against 1.5 ulp for the two-step `deg * PI / 180`, and
    // radians(180) == PI and degrees(PI) == 180 exactly.  NaN propagates,
    // ±inf maps to ±inf and ±0 keeps its sign.

    /// Convert degrees to radians.
    ///
    /// - Parameter deg: angle in degrees
    /// - Returns: angle in radians
    public static func radians(_ deg: Double) -> Double {
        return deg * RAD_PER_DEG
    }

    /// Convert radians to degrees.
    ///
    /// - Parameter rad: angle in radians
    /// - Returns: angle in degrees
    public static func degrees(_ rad: Double) -> Double {
        return rad * DEG_PER_RAD
    }
}
