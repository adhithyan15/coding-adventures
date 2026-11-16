// ============================================================================
// Trig.kt — Trigonometric Functions from First Principles (TRIG-REF-1)
// ============================================================================
//
// This library implements sin, cos, tan, atan, atan2, sqrt, and angle
// conversion using nothing but basic arithmetic.  No kotlin.math.sin or
// Math.cos anywhere — every value is computed from the Maclaurin
// (Taylor-at-zero) series, after an exact-as-possible range reduction.
//
// Since 0.2.0 the arithmetic follows the shared reference algorithm TRIG-REF-1
// of specification PHY00a, *bit for bit*: every Trig port in every language is
// meant to return the same 64 bits for the same input, so that a claim like
// "sin(1.0) returns exactly these bits" holds in all of them.  The vectors in
// code/specs/fixtures/trig-reference-v1 are the proof (see ConformanceTest).
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
// ============================================================================
// The arithmetic model (PHY00a 3.1) and why Kotlin/JVM can meet it
// ============================================================================
//
// Every value is an IEEE 754 binary64 number and every operation rounds once,
// to nearest, ties to even.  Nothing is fused, reassociated, or computed in
// extended precision.  Since Java 17 (JEP 306) all floating-point arithmetic
// on the JVM is strict, and Kotlin/JVM compiles `a * b` to one `dmul` and
// `+ c` to one `dadd`; the JIT never contracts `a * b + c` into a fused
// multiply-add unless the program calls Math.fma, which this file never does.
// The pseudo-code of the specification writes every product that feeds a sum
// on its own line (a "fusable site"); this file does the same, so a reader
// can see each rounding.
//
// ============================================================================

package com.codingadventures.trig

/**
 * Trigonometric functions computed from first principles (algorithm TRIG-REF-1).
 *
 * All functions operate in radians unless otherwise specified.
 * No `kotlin.math` or `java.lang.Math` functions are used by `sin`, `cos`,
 * `radians` or `degrees` — every result comes from IEEE addition, subtraction
 * and multiplication alone (no division, no square root).
 *
 * **Domain.** `sin`, `cos` and `tan` are defined for `|x| <= 2^29`.  Outside it
 * (larger finite values, infinities, NaN) they return NaN: a value that cannot
 * be mistaken for a sine, and that propagates.
 *
 * **Accuracy.** Within the domain `sin` and `cos` are within 3 ulp of the exact
 * value (observed worst: 2.38 ulp); `radians` and `degrees` are within 1 ulp.
 * No accuracy claim is made for `tan`.
 */
object Trig {

    // =========================================================================
    // Constants (PHY00a 3.2) — specified by BIT PATTERN
    // =========================================================================
    //
    // A decimal literal's value depends on the compiler's parser; a bit pattern
    // does not.  Kotlin has no hexadecimal floating-point literal, so every
    // reference constant that is not a plain decimal is built with
    // Double.fromBits (not a `const`, so these are `val`s set when the object
    // loads), and ConstantsTest asserts each one against the manifest of the
    // shared fixture.

    /**
     * The ratio of a circle's circumference to its diameter
     * (`0x400921FB54442D18`).  A decimal literal is kept here so that the
     * constant stays a compile-time `const`; its bits are asserted.
     */
    const val PI: Double = 3.141592653589793

    /** Two times pi — a full revolution in radians (`0x401921FB54442D18`, exact). */
    internal const val TWO_PI: Double = 2.0 * PI

    /** Half of pi (`0x3FF921FB54442D18`, exact), used by atan and atan2. */
    internal const val HALF_PI: Double = PI / 2.0

    /** The double nearest 2/pi: `x * INV_PIO2` estimates how many quarter-turns fit in x. */
    internal val INV_PIO2: Double = Double.fromBits(0x3FE45F306DC9C883L)

    /**
     * pi/2 split into five pieces (the Cody–Waite device).  The first four have
     * at most 24 significant bits each, so `k * PIO2[i]` is EXACT for any
     * integer |k| < 2^29 (24 + 29 = 53 bits).  The fifth holds the remainder
     * rounded to 53 bits.  Together they carry pi/2 to about 149 bits.
     *
     * ```
     *   PIO2[0] = 0x3FF921FB40000000   1.570796251296997
     *   PIO2[1] = 0x3E74442D00000000   7.549789415861596e-08
     *   PIO2[2] = 0x3CF8469880000000   5.390302529957765e-15
     *   PIO2[3] = 0x3B78CC5160000000   3.282003415807913e-22
     *   PIO2[4] = 0x39F01B839A25204A   1.2706558760139879e-29
     * ```
     */
    internal val PIO2: DoubleArray = doubleArrayOf(
        Double.fromBits(0x3FF921FB40000000L),
        Double.fromBits(0x3E74442D00000000L),
        Double.fromBits(0x3CF8469880000000L),
        Double.fromBits(0x3B78CC5160000000L),
        Double.fromBits(0x39F01B839A25204AL),
    )

    /**
     * 1.5 * 2^52.  Adding it to a value t with |t| < 2^51 pushes the fraction
     * bits off the end of the significand, so `(t + MAGIC) - MAGIC` is t
     * rounded to the nearest integer (ties to even) using only an add and a
     * subtract: no floor, no round, no float-to-int cast.
     */
    internal val MAGIC: Double = Double.fromBits(0x4338000000000000L)

    /** 2^29: the edge of the domain.  Beyond it k no longer fits the exact products. */
    internal val LIMIT: Double = Double.fromBits(0x41C0000000000000L)

    /** 2^-27: below this sin(x) is x and cos(x) is 1 exactly (the next term is under half an ulp). */
    internal val TINY: Double = Double.fromBits(0x3E40000000000000L)

    /**
     * Sine coefficients, `S[n] = (-1)^n / (2n+1)!` for n = 1..9, each rounded
     * once from the exact rational.  Index 0 is unused so that the index is
     * the series index.  The factorials are visible in the table:
     *
     * ```
     *   n   value   bits
     *   1  -1/3!    0xBFC5555555555555
     *   2  +1/5!    0x3F81111111111111
     *   3  -1/7!    0xBF2A01A01A01A01A
     *   4  +1/9!    0x3EC71DE3A556C734
     *   5  -1/11!   0xBE5AE64567F544E4
     *   6  +1/13!   0x3DE6124613A86D09
     *   7  -1/15!   0xBD6AE7F3E733B81F
     *   8  +1/17!   0x3CE952C77030AD4A
     *   9  -1/19!   0xBC62F49B46814157
     * ```
     */
    internal val S: DoubleArray = doubleArrayOf(
        0.0,
        Double.fromBits(0xBFC5555555555555uL.toLong()),
        Double.fromBits(0x3F81111111111111L),
        Double.fromBits(0xBF2A01A01A01A01AuL.toLong()),
        Double.fromBits(0x3EC71DE3A556C734L),
        Double.fromBits(0xBE5AE64567F544E4uL.toLong()),
        Double.fromBits(0x3DE6124613A86D09L),
        Double.fromBits(0xBD6AE7F3E733B81FuL.toLong()),
        Double.fromBits(0x3CE952C77030AD4AL),
        Double.fromBits(0xBC62F49B46814157uL.toLong()),
    )

    /**
     * Cosine coefficients, `C[n] = (-1)^n / (2n)!` for n = 2..9.  (The n = 1
     * coefficient, -1/2, is applied as `0.5 * z`, which is exact.)  Where 2n+2
     * is a power of two, neighbouring factorials differ by it, so S[3] and
     * C[4] (1/7!, 1/8!) share a significand, as do S[7] and C[8].
     *
     * ```
     *   n   value   bits
     *   2  +1/4!    0x3FA5555555555555
     *   3  -1/6!    0xBF56C16C16C16C17
     *   4  +1/8!    0x3EFA01A01A01A01A
     *   5  -1/10!   0xBE927E4FB7789F5C
     *   6  +1/12!   0x3E21EED8EFF8D898
     *   7  -1/14!   0xBDA93974A8C07C9D
     *   8  +1/16!   0x3D2AE7F3E733B81F
     *   9  -1/18!   0xBCA6827863B97D97
     * ```
     */
    internal val C: DoubleArray = doubleArrayOf(
        0.0,
        0.0,
        Double.fromBits(0x3FA5555555555555L),
        Double.fromBits(0xBF56C16C16C16C17uL.toLong()),
        Double.fromBits(0x3EFA01A01A01A01AL),
        Double.fromBits(0xBE927E4FB7789F5CuL.toLong()),
        Double.fromBits(0x3E21EED8EFF8D898L),
        Double.fromBits(0xBDA93974A8C07C9DuL.toLong()),
        Double.fromBits(0x3D2AE7F3E733B81FL),
        Double.fromBits(0xBCA6827863B97D97uL.toLong()),
    )

    /** pi/180 as one stored double (`0x3F91DF46A2529D39`); equals `PI / 180.0`. */
    internal val RAD_PER_DEG: Double = Double.fromBits(0x3F91DF46A2529D39L)

    /** 180/pi as one stored double (`0x404CA5DC1A63C1F8`); equals `180.0 / PI`. */
    internal val DEG_PER_RAD: Double = Double.fromBits(0x404CA5DC1A63C1F8L)

    // =========================================================================
    // Range Reduction (PHY00a 3.4) — nearest-integer Cody–Waite
    // =========================================================================
    //
    // A sine is a repeating wave: knowing sin on [-pi/4, pi/4] and which
    // quarter-turn we are in is enough to know it everywhere.  Reducing means
    // answering two questions: how many quarter-turns fit in x (an integer k),
    // and what is left over (r = x - k*pi/2, with |r| about <= pi/4)?
    //
    // Step 1 — the quotient.
    //
    //     t = x * INV_PIO2                 one rounding; x / (pi/2), approximately
    //     k = (t + MAGIC) - MAGIC          nearest integer to t, ties to even
    //
    // MAGIC = 1.5 * 2^52 has its units digit in the last place of the
    // significand, so t + MAGIC rounds t to a whole number *in hardware* by the
    // rule IEEE already defines.  The following "- MAGIC" is exact and hands
    // the integer back as a double.  This needs only + and -, so there is no
    // library round/floor, no float-to-int conversion, and no behaviour to
    // remember at NaN or infinity (the domain guard has already removed them).
    //
    // Step 2 — the remainder, by Cody–Waite subtraction.
    //
    //     r = x - k*PIO2[0]     product exact; subtraction exact (Sterbenz)
    //     r = r - k*PIO2[1]     product exact; subtraction exact
    //     r = r - k*PIO2[2]     product exact; subtraction rounds
    //     r = r - k*PIO2[3]     product exact; subtraction rounds
    //     r = r - k*PIO2[4]     product rounds; subtraction rounds
    //
    //   x ------------------------------------------------------------ the angle
    //   |-- k*PIO2[0] --|                                               24 bits of k*pi/2
    //                   |-- k*PIO2[1] --|                              next 24
    //                                   |-- k*PIO2[2] --|              next 24
    //                                                   |- k*PIO2[3] -|  next 24
    //                                                               |k*PIO2[4]|  last, rounds
    //   r = what is left, |r| <= pi/4 (plus a few ulps: t was rounded before k was chosen)
    //
    // pi/2 is irrational, so k*(pi/2) cannot be formed exactly in 53 bits.  The
    // trick is to store pi/2 as a SUM of short pieces, each short enough that
    // multiplying by an integer k below 2^29 is exact.  (Compare the old port,
    // which subtracted multiples of the single double TWO_PI with `%` and so
    // drifted by k * 2.4e-16 radians.)
    //
    // Step 3 — the quadrant is  k.toInt() and 3.  k is an integer-valued double
    // below 2^29, so the conversion is exact, and "and 3" on two's complement
    // is the floor modulus 4 (k = -1 gives 3, k = -5 gives 3).

    /** The reduced angle together with the quadrant it came from. */
    private class Reduced(val r: Double, val q: Int)

    /**
     * Reduce [x] (already known to satisfy `|x| <= LIMIT`) to a remainder `r`
     * near [-pi/4, pi/4] and a quadrant `q` in 0..3.
     *
     * Every product that feeds a subtraction is its own statement: these are
     * the "fusable sites" of PHY00a 3.1, and none may be fused.
     */
    private fun reduce(x: Double): Reduced {
        val t = x * INV_PIO2                // one rounding
        val k = (t + MAGIC) - MAGIC         // nearest integer, as a double
        var r = x
        for (i in 0 until 5) {
            val m = k * PIO2[i]             // exact for i < 4
            r = r - m
        }
        return Reduced(r, k.toInt() and 3)
    }

    // =========================================================================
    // The series kernels (PHY00a 3.5) — Horner's rule on stored coefficients
    // =========================================================================
    //
    // On |r| <= pi/4 (about 0.7854), sin and cos are polynomials in z = r*r:
    //
    //   sin(r) = r + r*z*(S1 + S2*z + S3*z^2 + ... + S9*z^8)      x^1 .. x^19
    //   cos(r) = 1 - z/2 + z^2*(C2 + C3*z + ... + C9*z^7)         x^0 .. x^18
    //
    // Ten terms each: the first omitted term is under 1e-19 relative, far below
    // half an ulp, so the count is fixed and every port adds the same terms.
    //
    // Horner's rule evaluates innermost coefficient first (the smallest terms
    // are added first, which is the accurate order), with one multiply and one
    // add per step and NO division — so there is no grouping ambiguity and no
    // zero-divisor to worry about.  The leading term is added last and
    // unrounded inside the polynomial, so the dominant part of the answer
    // carries only one rounding.

    /** sin on the reduced interval. */
    private fun sinK(r: Double): Double {
        val z = r * r
        var p = S[9]
        for (n in 8 downTo 1) {
            p = p * z
            p = p + S[n]
        }
        var w = r * z
        w = w * p
        return r + w
    }

    /** cos on the reduced interval. */
    private fun cosK(r: Double): Double {
        val z = r * r
        var q = C[9]
        for (n in 8 downTo 2) {
            q = q * z
            q = q + C[n]
        }
        var u = 0.5 * z                     // exact
        u = 1.0 - u
        var v = z * z
        v = v * q
        return u + v
    }

    // =========================================================================
    // sin(x) and cos(x) (PHY00a 3.3, 3.6)
    // =========================================================================
    //
    // The quadrant table is the identity set
    //
    //   sin(r + k*pi/2) = { sin r,  cos r, -sin r, -cos r }[k mod 4]
    //   cos(r + k*pi/2) = { cos r, -sin r, -cos r,  sin r }[k mod 4]
    //
    // The negations are exact.
    //
    // THE DOMAIN GUARD IS A NEGATED <=, AND MUST BE.  A NaN fails every
    // comparison, so  !(|x| <= LIMIT)  is true for NaN, for both infinities and
    // for every finite value beyond 2^29: one comparison covers all three.  The
    // tempting rewrite  |x| > LIMIT  would let NaN through to the conversion.

    /**
     * Compute the sine of [x] (in radians).
     *
     * 1. NaN, infinities and `|x| > 2^29`: NaN (outside the domain).
     * 2. `|x| < 2^-27`: `x` itself (keeps `-0` and subnormals).
     * 3. Otherwise reduce to `(r, q)` and pick the kernel by quadrant.
     *
     * @param x angle in radians
     * @return sine of x, or NaN when `|x| > 2^29` or x is not finite
     */
    fun sin(x: Double): Double {
        val ax = if (x < 0.0) -x else x     // |x|; NaN stays NaN
        if (!(ax <= LIMIT)) return Double.NaN
        if (ax < TINY) return x             // includes +0, -0 and every subnormal
        val red = reduce(x)
        return when (red.q) {
            0 -> sinK(red.r)
            1 -> cosK(red.r)
            2 -> -sinK(red.r)
            else -> -cosK(red.r)
        }
    }

    /**
     * Compute the cosine of [x] (in radians).
     *
     * @param x angle in radians
     * @return cosine of x, or NaN when `|x| > 2^29` or x is not finite
     */
    fun cos(x: Double): Double {
        val ax = if (x < 0.0) -x else x
        if (!(ax <= LIMIT)) return Double.NaN
        if (ax < TINY) return 1.0
        val red = reduce(x)
        return when (red.q) {
            0 -> cosK(red.r)
            1 -> -sinK(red.r)
            2 -> -cosK(red.r)
            else -> sinK(red.r)
        }
    }

    // =========================================================================
    // Angle Conversion (PHY00a 3.7)
    // =========================================================================
    //
    // Degrees and radians are two ways to measure the same thing — how far
    // around a circle you've gone.
    //
    //   360 degrees = 2*pi radians  →  1 degree  = pi/180 radians
    //                                   1 radian = 180/pi degrees
    //
    // One multiply by a stored, bit-pinned constant: one rounding (not two, as
    // in deg * PI / 180), no intermediate that overflows, and it is the more
    // accurate form (worst 0.65 ulp for radians, 0.81 for degrees).

    /**
     * Convert degrees to radians: one multiply by `pi/180`.
     *
     * @param deg angle in degrees
     * @return equivalent angle in radians
     */
    fun radians(deg: Double): Double = deg * RAD_PER_DEG

    /**
     * Convert radians to degrees: one multiply by `180/pi`.
     *
     * @param rad angle in radians
     * @return equivalent angle in degrees
     */
    fun degrees(rad: Double): Double = rad * DEG_PER_RAD

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

        if (x == 0.0) return x
        if (x == Double.POSITIVE_INFINITY) return x

        var scaled = x
        var resultScale = 1.0
        while (scaled < 0.25) {
            scaled *= 4.0
            resultScale *= 0.5
        }
        while (scaled >= 4.0) {
            scaled *= 0.25
            resultScale *= 2.0
        }

        // Initial guess: x itself for x >= 1, 1.0 for x < 1.
        var guess = if (scaled >= 1.0) scaled else 1.0

        repeat(60) {
            val next = (guess + scaled / guess) / 2.0
            if (Math.abs(next - guess) < 1e-15 * guess + 1e-300) {
                guess = next
                return guess * resultScale
            }
            guess = next
        }

        return guess * resultScale
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
     * Outside `|x| <= 2^29` both are NaN, the pole test is false for NaN, and
     * `NaN / NaN` is NaN.  The pole sentinel is a pinned legacy convention
     * (PHY00 3.6), not an approximation, and no accuracy claim is made for `tan`.
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
        // atan(x) rounds exactly to x here; avoid halving subnormals and retain -0.
        if (kotlin.math.abs(x) <= 7.450580596923828e-9) return x
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
        // A NaN in either position is undefined; without this branch a NaN x would
        // fall through every comparison to the "origin" branch and return 0.0.
        x.isNaN() || y.isNaN() -> Double.NaN
        x > 0.0             -> atan(y / x)
        x < 0.0 && y >= 0.0 -> atan(y / x) + PI
        x < 0.0 && y < 0.0  -> atan(y / x) - PI
        x == 0.0 && y > 0.0 ->  HALF_PI
        x == 0.0 && y < 0.0 -> -HALF_PI
        else                -> 0.0  // x == 0 and y == 0: undefined, return 0 by convention
    }
}
