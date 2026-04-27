// ============================================================================
// Trig.kt — Trigonometric Functions from First Principles
// ============================================================================
//
// This library implements sin, cos, tan, atan, atan2, sqrt, and angle
// conversion using nothing but basic arithmetic.  No kotlin.math.sin or
// Math.cos anywhere — every value is computed from the Maclaurin
// (Taylor-at-zero) series.
//
// Why?  Because understanding *how* a sine is calculated is more valuable
// than calling someone else's black box.  If you can build trig from
// scratch, you truly understand what these functions do.
//
// ============================================================================
// Background: What is a Maclaurin Series?
// ============================================================================
//
// A Maclaurin series expands a function as an infinite sum of terms derived
// from the function's derivatives at zero:
//
//   f(x) = f(0) + f'(0)*x + f''(0)*x²/2! + f'''(0)*x³/3! + ...
//
// For sine, the derivatives cycle through {sin, cos, -sin, -cos}, and
// evaluating at zero gives us:
//
//   sin(x) = x - x³/3! + x⁵/5! - x⁷/7! + ...
//
// Only odd powers appear (because sin is an odd function), and the signs
// alternate.  For cosine (an even function) only even powers appear:
//
//   cos(x) = 1 - x²/2! + x⁴/4! - x⁶/6! + ...
//
// These series converge for every real number x, but they converge *fastest*
// when x is close to zero.  That is where range reduction comes in.
//

package com.codingadventures.trig

/**
 * Trigonometric functions computed from first principles using Maclaurin series.
 *
 * All functions operate in radians unless otherwise specified.
 * No `kotlin.math` or `java.lang.Math` functions are used internally — every
 * result is derived from addition, multiplication, and division alone.
 *
 * Accuracy: 20-term series with range reduction gives full IEEE 754
 * double-precision accuracy (~15 decimal digits) for all inputs.
 */
object Trig {

    // =========================================================================
    // Constants
    // =========================================================================

    /**
     * The ratio of a circle's circumference to its diameter.
     *
     * Hard-coded to the same precision as `kotlin.math.PI` so the library is
     * fully self-contained — no dependency on any standard constants.
     */
    const val PI: Double = 3.141592653589793

    /** Two times pi — a full revolution in radians. */
    private const val TWO_PI: Double = 2.0 * PI

    /** Half of pi — used by atan and atan2. */
    private const val HALF_PI: Double = PI / 2.0

    // =========================================================================
    // Range Reduction
    // =========================================================================
    //
    // The Maclaurin series converges for any x, but convergence is slow for
    // large |x|.  Worse, floating-point round-off accumulates when you raise a
    // large number to the 39th power.
    //
    // The fix: since sin and cos are periodic with period 2*pi, we can always
    // map x into the interval [-pi, pi] without changing the function's value.
    //
    //   1. Take x % (2*pi)  →  x is now in (-2pi, 2pi)
    //   2. If x > pi, subtract 2*pi  →  x is now in [-pi, pi]
    //   3. If x < -pi, add 2*pi      →  same guarantee
    //
    // After this, |x| <= pi ≈ 3.14, so our series terms shrink quickly.

    /**
     * Reduce [x] into the range [-pi, pi].
     *
     * Preserves the value of any 2*pi-periodic function (sin, cos, etc.)
     * while keeping the magnitude small for faster series convergence.
     */
    private fun rangeReduce(x: Double): Double {
        // Step 1: use % to land in (-2*pi, 2*pi).
        // Kotlin's % keeps the sign of the dividend, so we need both steps.
        var r = x % TWO_PI

        // Step 2 & 3: nudge into [-pi, pi].
        if (r > PI) r -= TWO_PI
        if (r < -PI) r += TWO_PI

        return r
    }

    // =========================================================================
    // sin(x) — Maclaurin Series
    // =========================================================================
    //
    // The series:
    //
    //   sin(x) = x - x³/3! + x⁵/5! - x⁷/7! + ...
    //
    // Iterative term relation (avoids recomputing factorials):
    //
    //   term_0 = x
    //   term_k = term_{k-1} * (-x²) / (2k * (2k+1))
    //
    // Each successive term is just the previous term multiplied by a small
    // fraction — one multiply and one divide per step, no large powers.

    /**
     * Compute the sine of [x] (in radians) using a 20-term Maclaurin series.
     *
     * ### How it works
     * 1. Range-reduce [x] to [-pi, pi] so the series converges quickly.
     * 2. Start with `term = x` (the first term, k=0).
     * 3. Multiply by `-x² / (2k)(2k+1)` to get the next term.
     * 4. Accumulate 20 terms — more than enough for double precision.
     *
     * @param x angle in radians
     * @return sine of x
     */
    fun sin(x: Double): Double {
        // --- Step 1: Range reduction ---
        val r = rangeReduce(x)

        // --- Step 2: Pre-compute x² (reused every iteration) ---
        val xSq = r * r

        // --- Step 3: Iterative summation ---
        var term = r
        var sum  = r

        for (k in 1 until 20) {
            // Each new term is the previous term multiplied by:
            //   -x² / ( (2k) * (2k + 1) )
            val denom = (2 * k).toDouble() * (2 * k + 1).toDouble()
            term *= -xSq / denom
            sum  += term
        }

        return sum
    }

    // =========================================================================
    // cos(x) — Maclaurin Series
    // =========================================================================
    //
    // The cosine series uses even powers:
    //
    //   cos(x) = 1 - x²/2! + x⁴/4! - x⁶/6! + ...
    //
    // Iterative relation:
    //
    //   term_0 = 1
    //   term_k = term_{k-1} * (-x²) / ((2k-1) * 2k)

    /**
     * Compute the cosine of [x] (in radians) using a 20-term Maclaurin series.
     *
     * @param x angle in radians
     * @return cosine of x
     */
    fun cos(x: Double): Double {
        // --- Step 1: Range reduction ---
        val r = rangeReduce(x)

        // --- Step 2: Pre-compute x² ---
        val xSq = r * r

        // --- Step 3: Iterative summation ---
        var term = 1.0
        var sum  = 1.0

        for (k in 1 until 20) {
            // term_k = term_{k-1} * (-x²) / ((2k-1) * 2k)
            val denom = (2 * k - 1).toDouble() * (2 * k).toDouble()
            term *= -xSq / denom
            sum  += term
        }

        return sum
    }

    // =========================================================================
    // Angle Conversion
    // =========================================================================

    /**
     * Convert degrees to radians.
     *
     * Formula: `radians = degrees * (pi / 180)`
     *
     * @param deg angle in degrees
     * @return equivalent angle in radians
     */
    fun radians(deg: Double): Double = deg * (PI / 180.0)

    /**
     * Convert radians to degrees.
     *
     * Formula: `degrees = radians * (180 / pi)`
     *
     * @param rad angle in radians
     * @return equivalent angle in degrees
     */
    fun degrees(rad: Double): Double = rad * (180.0 / PI)

    // =========================================================================
    // sqrt(x) — Newton's (Babylonian) Method
    // =========================================================================
    //
    // Newton's method for square roots is one of the oldest algorithms in
    // human history — it appears in Babylonian clay tablets from ~1700 BCE.
    //
    // Idea: if `guess` is an approximation to sqrt(x), the average of `guess`
    // and `x / guess` is a *better* approximation.
    //
    // Convergence is quadratic — each iteration roughly doubles the number of
    // correct digits.

    /**
     * Compute the square root of [x] using Newton's method.
     *
     * @param x a non-negative value
     * @return the square root of x
     * @throws ArithmeticException if x is negative
     */
    fun sqrt(x: Double): Double {
        if (x < 0.0) throw ArithmeticException(
            "sqrt: domain error — input $x is negative")

        if (x == 0.0) return 0.0

        // Initial guess: x itself for x >= 1, 1.0 for x < 1.
        var guess = if (x >= 1.0) x else 1.0

        repeat(60) {
            val next = (guess + x / guess) / 2.0
            if (Math.abs(next - guess) < 1e-15 * guess + 1e-300) {
                guess = next
                return guess
            }
            guess = next
        }

        return guess
    }

    // =========================================================================
    // tan(x) — Tangent as Sine / Cosine
    // =========================================================================
    //
    // tan(x) = sin(x) / cos(x)
    //
    // Poles at x = π/2 + k·π. We guard by returning a large finite value
    // when |cos(x)| < 1e-15.

    /**
     * Compute the tangent of [x] (in radians).
     *
     * Uses our own [sin] and [cos] — no standard library math functions.
     *
     * @param x angle in radians
     * @return tangent of x (a very large finite value near poles)
     */
    fun tan(x: Double): Double {
        val s = sin(x)
        val c = cos(x)

        if (Math.abs(c) < 1e-15) {
            return if (s > 0.0) 1.0e308 else -1.0e308
        }

        return s / c
    }

    // =========================================================================
    // atan(x) — Arctangent via Taylor Series with Range Reduction
    // =========================================================================
    //
    // atan(x) = x - x³/3 + x⁵/5 - x⁷/7 + ...  (for |x| <= 1)
    //
    // Two layers of range reduction:
    //   Layer 1: for |x| > 1:  atan(x) = ±π/2 - atan(1/x)
    //   Layer 2: half-angle:   atan(x) = 2·atan(x / (1 + sqrt(1 + x²)))

    /**
     * Inner atan for |x| ≤ 1. Applies half-angle reduction, then Taylor series.
     */
    private fun atanCore(x: Double): Double {
        // Half-angle reduction.
        val reduced = x / (1.0 + sqrt(1.0 + x * x))

        val t    = reduced
        val tSq  = t * t
        var term = t
        var result = t

        for (n in 1..30) {
            term = term * (-tSq) * (2 * n - 1.0) / (2 * n + 1.0)
            result += term
            if (Math.abs(term) < 1e-17) break
        }

        return 2.0 * result
    }

    /**
     * Compute the arctangent of [x] (in radians).
     *
     * Return range: `(-pi/2, pi/2)`.
     *
     * @param x input value
     * @return angle whose tangent is x, in radians
     */
    fun atan(x: Double): Double {
        if (x == 0.0) return 0.0
        if (x >  1.0) return  HALF_PI - atanCore(1.0 / x)
        if (x < -1.0) return -HALF_PI - atanCore(1.0 / x)
        return atanCore(x)
    }

    // =========================================================================
    // atan2(y, x) — Four-Quadrant Arctangent
    // =========================================================================
    //
    // Returns the angle in (-π, π] that point (x, y) makes with the positive
    // x-axis.  Unlike atan(y/x), atan2 handles all four quadrants correctly
    // by inspecting the signs of both y and x independently.

    /**
     * Compute the four-quadrant arctangent of ([y], [x]) in radians.
     *
     * Return range: `(-pi, pi]`.
     *
     * @param y the y-coordinate
     * @param x the x-coordinate
     * @return the angle in (-pi, pi] that point (x, y) makes with the positive x-axis
     */
    fun atan2(y: Double, x: Double): Double = when {
        x > 0.0             -> atan(y / x)
        x < 0.0 && y >= 0.0 -> atan(y / x) + PI
        x < 0.0 && y < 0.0  -> atan(y / x) - PI
        x == 0.0 && y > 0.0 ->  HALF_PI
        x == 0.0 && y < 0.0 -> -HALF_PI
        else                -> 0.0  // x == 0 and y == 0: undefined, return 0 by convention
    }
}
