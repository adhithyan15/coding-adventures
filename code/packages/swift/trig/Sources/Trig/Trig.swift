// ============================================================================
// Trig.swift — Trigonometric Functions from First Principles
// ============================================================================
//
// This module implements sin, cos, tan, atan, atan2, sqrt, and angle
// conversion functions using ONLY basic arithmetic — no Foundation, no Darwin,
// no Swift.Numerics, no standard-library math functions.
//
// Everything is derived from:
//   - Maclaurin (Taylor-at-zero) series for sin and cos
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
// Layer: PHY00 (physics, leaf package — zero dependencies beyond Swift stdlib)
// ============================================================================

// ----------------------------------------------------------------------------
// Public Constants
// ----------------------------------------------------------------------------

/// PI — the ratio of a circle's circumference to its diameter.
///
/// Hard-coded to the full precision of a Double (IEEE 754 binary64).
/// This is the same value as `Double.pi`, but we define it ourselves to
/// make the module fully self-contained.
public let PI: Double = 3.141592653589793

/// Private: two times PI, one full revolution. Used for range reduction.
private let TWO_PI: Double = 2.0 * PI

/// Private: PI / 2. Used by atan range reduction and atan2 quadrant logic.
private let HALF_PI: Double = PI / 2.0

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
    // Range Reduction (private)
    // -------------------------------------------------------------------------
    //
    // The Maclaurin series for sin and cos converges fastest when x is near 0.
    // For large |x|, the series terms start enormous before cancelling, causing
    // floating-point precision loss. Since sin and cos are periodic (period 2π),
    // we can always reduce any input to [-π, π] without changing the value.
    //
    //   sin(x) = sin(x - 2π·k)  for any integer k
    //   cos(x) = cos(x - 2π·k)  for any integer k
    //
    // Algorithm:
    //   1. Compute x mod 2π using division and truncation (toward zero).
    //   2. If the result is outside [-π, π], adjust by one more ±2π.

    private static func rangeReduce(_ x: Double) -> Double {
        // Step 1: remove full rotations. Swift's truncatingRemainder gives
        // the same sign as x (truncation toward zero), unlike floor-based modulo.
        var x = x.truncatingRemainder(dividingBy: TWO_PI)

        // Step 2: adjust to [-π, π].
        if x > PI  { x -= TWO_PI }
        if x < -PI { x += TWO_PI }
        return x
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
    // Rather than computing x^n and n! from scratch (which overflow for large n),
    // we compute each term from the previous one:
    //
    //   term₀ = x
    //   termₙ = term_{n-1} · (-x²) / ((2n)(2n+1))
    //
    // This is numerically stable and converges in about 12-15 terms for |x| ≤ π.
    //
    // Example: sin(π/6) = 0.5
    //
    //   x = 0.5236 (π/6)
    //   term 0: +0.5236
    //   term 1: -0.0239  (multiply by -x²/(2·3))
    //   term 2: +0.0003  (multiply by -x²/(4·5))
    //   ... sum → 0.5000

    /// Compute the sine of `x` (in radians) using a 20-term Maclaurin series.
    ///
    /// - Parameter x: angle in radians
    /// - Returns: sin(x), accurate to full Double precision for any input
    public static func sin(_ x: Double) -> Double {
        let x = rangeReduce(x)
        var term = x          // First term: x¹/1! = x
        var sum  = term       // Running total

        for n in 1...20 {
            // term_n = term_{n-1} · (-x²) / ((2n)(2n+1))
            let denom = Double(2 * n) * Double(2 * n + 1)
            term *= -(x * x) / denom
            sum  += term
        }

        return sum
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
    // Iterative term computation (same idea as sine, different denominator):
    //
    //   term₀ = 1
    //   termₙ = term_{n-1} · (-x²) / ((2n-1)(2n))
    //
    // The denominator differs because cosine uses even factorials while
    // sine uses odd factorials.

    /// Compute the cosine of `x` (in radians) using a 20-term Maclaurin series.
    ///
    /// - Parameter x: angle in radians
    /// - Returns: cos(x), accurate to full Double precision for any input
    public static func cos(_ x: Double) -> Double {
        let x = rangeReduce(x)
        var term = 1.0        // First term: x⁰/0! = 1
        var sum  = term

        for n in 1...20 {
            // term_n = term_{n-1} · (-x²) / ((2n-1)(2n))
            let denom = Double(2 * n - 1) * Double(2 * n)
            term *= -(x * x) / denom
            sum  += term
        }

        return sum
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
        precondition(x >= 0, "Trig.sqrt: domain error — input \(x) is negative")

        // sqrt(0) = 0 exactly.
        if x == 0.0 { return 0.0 }

        // Initial guess: x itself for x ≥ 1 (saves iterations for large values),
        // 1.0 for x in (0, 1) (avoids dividing by a tiny number on the first step).
        var guess = x >= 1.0 ? x : 1.0

        // Iterate up to 60 times. Quadratic convergence means ~15 in practice.
        for _ in 0..<60 {
            let next = (guess + x / guess) / 2.0

            // Convergence criterion: stop when improvement is negligibly small.
            // 1e-15 * guess handles relative precision for large values.
            // 1e-300 is an absolute floor for subnormal (near-zero) inputs.
            if Swift.abs(next - guess) < 1e-15 * guess + 1e-300 {
                return next
            }

            guess = next
        }

        return guess
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
    //   We guard against |cos(x)| < 1e-15 and return ±Double.greatestFiniteMagnitude
    //   to indicate near-singularity without a runtime crash.

    /// Compute the tangent of `x` (in radians).
    ///
    /// Uses our own `sin(_:)` and `cos(_:)` — no Foundation math.
    ///
    /// - Parameter x: angle in radians
    /// - Returns: tan(x). Near poles (x ≈ π/2 + k·π), returns ±1e308.
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
        if x == 0.0 { return 0.0 }

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

    /// Convert degrees to radians.
    ///
    /// - Parameter deg: angle in degrees
    /// - Returns: angle in radians
    public static func radians(_ deg: Double) -> Double {
        return deg * (PI / 180.0)
    }

    /// Convert radians to degrees.
    ///
    /// - Parameter rad: angle in radians
    /// - Returns: angle in degrees
    public static func degrees(_ rad: Double) -> Double {
        return rad * (180.0 / PI)
    }
}
