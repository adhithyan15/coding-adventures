// ============================================================================
// Trig.java — Trigonometric Functions from First Principles (TRIG-REF-1)
// ============================================================================
//
// This library implements sin, cos, tan, atan, atan2, sqrt, and angle
// conversion using nothing but basic arithmetic.  No Math.sin or Math.cos
// anywhere — every value is computed from the Maclaurin (Taylor-at-zero)
// series, after an exact-as-possible range reduction.
//
// Since 0.2.0 the arithmetic follows the shared reference algorithm TRIG-REF-1
// of specification PHY00a, *bit for bit*: every Trig port in every language is
// meant to return the same 64 bits for the same input, so that a claim like
// "sin(1.0) returns exactly these bits" holds in all of them.  The vectors in
// code/specs/fixtures/trig-reference-v1 are the proof (see ConformanceTest).
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
// The arithmetic model (PHY00a 3.1) and why Java can meet it
// ============================================================================
//
// Every value is an IEEE 754 binary64 number and every operation rounds once,
// to nearest, ties to even.  Nothing is fused, reassociated, or computed in
// extended precision.  Since Java 17 (JEP 306) all floating-point arithmetic is
// strict, so `a * b` is always one rounded multiply and `+ c` another; the JIT
// never contracts `a * b + c` into a fused multiply-add unless the program
// calls Math.fma, which this file never does.  The pseudo-code of the
// specification writes every product that feeds a sum on its own line (a
// "fusable site"); this file does the same, so a reader can see each rounding.
//
// ============================================================================

package com.codingadventures.trig;

/**
 * Trigonometric functions computed from first principles (algorithm TRIG-REF-1).
 *
 * <p>All functions operate in radians unless otherwise specified.
 * No {@code java.lang.Math} functions are used by {@code sin}, {@code cos},
 * {@code radians} or {@code degrees} — every result comes from IEEE addition,
 * subtraction and multiplication alone (no division, no square root).
 *
 * <p><b>Domain.</b> {@code sin}, {@code cos} and {@code tan} are defined for
 * {@code |x| <= 2^29}.  Outside it (larger finite values, infinities, NaN) they
 * return NaN: a value that cannot be mistaken for a sine, and that propagates.
 *
 * <p><b>Accuracy.</b> Within the domain {@code sin} and {@code cos} are within
 * 3 ulp of the exact value (observed worst: 2.38 ulp); {@code radians} and
 * {@code degrees} are within 1 ulp.  No accuracy claim is made for {@code tan}.
 */
public final class Trig {

    // =========================================================================
    // Constants (PHY00a 3.2) — specified by BIT PATTERN
    // =========================================================================
    //
    // A decimal literal's value depends on the compiler's parser; a bit pattern
    // does not.  So every reference constant is built with
    // Double.longBitsToDouble, and ConformanceTest asserts each one against the
    // manifest of the shared fixture.

    /**
     * The ratio of a circle's circumference to its diameter
     * ({@code 0x400921FB54442D18}).  A decimal literal is kept here so that the
     * constant stays usable in constant expressions; its bits are asserted.
     */
    public static final double PI = 3.141592653589793;

    /** Two times pi — a full revolution in radians ({@code 0x401921FB54442D18}, exact). */
    static final double TWO_PI = 2.0 * PI;

    /** Half of pi ({@code 0x3FF921FB54442D18}, exact), used by atan and atan2. */
    static final double HALF_PI = PI / 2.0;

    /** The double nearest 2/pi: {@code x * INV_PIO2} estimates how many quarter-turns fit in x. */
    static final double INV_PIO2 = Double.longBitsToDouble(0x3FE45F306DC9C883L);

    /**
     * pi/2 split into five pieces (the Cody–Waite device).  The first four have
     * at most 24 significant bits each, so {@code k * PIO2[i]} is EXACT for any
     * integer |k| < 2^29 (24 + 29 = 53 bits).  The fifth holds the remainder
     * rounded to 53 bits.  Together they carry pi/2 to about 149 bits.
     *
     * <pre>
     *   PIO2[0] = 0x3FF921FB40000000   1.570796251296997
     *   PIO2[1] = 0x3E74442D00000000   7.549789415861596e-08
     *   PIO2[2] = 0x3CF8469880000000   5.390302529957765e-15
     *   PIO2[3] = 0x3B78CC5160000000   3.282003415807913e-22
     *   PIO2[4] = 0x39F01B839A25204A   1.2706558760139879e-29
     * </pre>
     */
    static final double[] PIO2 = {
        Double.longBitsToDouble(0x3FF921FB40000000L),
        Double.longBitsToDouble(0x3E74442D00000000L),
        Double.longBitsToDouble(0x3CF8469880000000L),
        Double.longBitsToDouble(0x3B78CC5160000000L),
        Double.longBitsToDouble(0x39F01B839A25204AL),
    };

    /**
     * 1.5 * 2^52.  Adding it to a value t with |t| < 2^51 pushes the fraction
     * bits off the end of the significand, so {@code (t + MAGIC) - MAGIC} is t
     * rounded to the nearest integer (ties to even) using only an add and a
     * subtract: no floor, no round, no float-to-int cast.
     */
    static final double MAGIC = Double.longBitsToDouble(0x4338000000000000L);

    /** 2^29: the edge of the domain.  Beyond it k no longer fits the exact products. */
    static final double LIMIT = Double.longBitsToDouble(0x41C0000000000000L);

    /** 2^-27: below this sin(x) is x and cos(x) is 1 exactly (the next term is under half an ulp). */
    static final double TINY = Double.longBitsToDouble(0x3E40000000000000L);

    /**
     * Sine coefficients, {@code S[n] = (-1)^n / (2n+1)!} for n = 1..9, each
     * rounded once from the exact rational.  Index 0 is unused so that the
     * index is the series index.  The factorials are visible in the table:
     *
     * <pre>
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
     * </pre>
     */
    static final double[] S = {
        0.0,
        Double.longBitsToDouble(0xBFC5555555555555L),
        Double.longBitsToDouble(0x3F81111111111111L),
        Double.longBitsToDouble(0xBF2A01A01A01A01AL),
        Double.longBitsToDouble(0x3EC71DE3A556C734L),
        Double.longBitsToDouble(0xBE5AE64567F544E4L),
        Double.longBitsToDouble(0x3DE6124613A86D09L),
        Double.longBitsToDouble(0xBD6AE7F3E733B81FL),
        Double.longBitsToDouble(0x3CE952C77030AD4AL),
        Double.longBitsToDouble(0xBC62F49B46814157L),
    };

    /**
     * Cosine coefficients, {@code C[n] = (-1)^n / (2n)!} for n = 2..9.  (The
     * n = 1 coefficient, -1/2, is applied as {@code 0.5 * z}, which is exact.)
     * Where 2n+2 is a power of two, neighbouring factorials differ by it, so
     * S[3] and C[4] (1/7!, 1/8!) share a significand, as do S[7] and C[8].
     *
     * <pre>
     *   n   value   bits
     *   2  +1/4!    0x3FA5555555555555
     *   3  -1/6!    0xBF56C16C16C16C17
     *   4  +1/8!    0x3EFA01A01A01A01A
     *   5  -1/10!   0xBE927E4FB7789F5C
     *   6  +1/12!   0x3E21EED8EFF8D898
     *   7  -1/14!   0xBDA93974A8C07C9D
     *   8  +1/16!   0x3D2AE7F3E733B81F
     *   9  -1/18!   0xBCA6827863B97D97
     * </pre>
     */
    static final double[] C = {
        0.0,
        0.0,
        Double.longBitsToDouble(0x3FA5555555555555L),
        Double.longBitsToDouble(0xBF56C16C16C16C17L),
        Double.longBitsToDouble(0x3EFA01A01A01A01AL),
        Double.longBitsToDouble(0xBE927E4FB7789F5CL),
        Double.longBitsToDouble(0x3E21EED8EFF8D898L),
        Double.longBitsToDouble(0xBDA93974A8C07C9DL),
        Double.longBitsToDouble(0x3D2AE7F3E733B81FL),
        Double.longBitsToDouble(0xBCA6827863B97D97L),
    };

    /** pi/180 as one stored double ({@code 0x3F91DF46A2529D39}); equals {@code PI / 180.0}. */
    static final double RAD_PER_DEG = Double.longBitsToDouble(0x3F91DF46A2529D39L);

    /** 180/pi as one stored double ({@code 0x404CA5DC1A63C1F8}); equals {@code 180.0 / PI}. */
    static final double DEG_PER_RAD = Double.longBitsToDouble(0x404CA5DC1A63C1F8L);

    // Private constructor: this is a pure utility class — no instances needed.
    private Trig() {}

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
    // library round/floor, no float-to-int cast, and no behaviour to remember
    // at NaN or infinity (the domain guard has already removed them).
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
    // which subtracted multiples of the single double TWO_PI and so drifted by
    // k * 2.4e-16 radians.)
    //
    // Step 3 — the quadrant is  ((int) k) & 3.  k is an integer-valued double
    // below 2^29, so the cast is exact, and "& 3" on two's complement is the
    // floor modulus 4 (k = -1 gives 3, k = -5 gives 3).

    /** The reduced angle together with the quadrant it came from. */
    private static final class Reduced {
        final double r;
        final int q;

        Reduced(double r, int q) {
            this.r = r;
            this.q = q;
        }
    }

    /**
     * Reduce {@code x} (already known to satisfy {@code |x| <= LIMIT}) to a
     * remainder {@code r} near [-pi/4, pi/4] and a quadrant {@code q} in 0..3.
     *
     * <p>Every product that feeds a subtraction is its own statement: these are
     * the "fusable sites" of PHY00a 3.1, and none may be fused.
     */
    private static Reduced reduce(double x) {
        double t = x * INV_PIO2;            // one rounding
        double k = (t + MAGIC) - MAGIC;     // nearest integer, as a double
        double r = x;
        for (int i = 0; i < 5; i++) {
            double m = k * PIO2[i];         // exact for i < 4
            r = r - m;
        }
        return new Reduced(r, ((int) k) & 3);
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
    private static double sinK(double r) {
        double z = r * r;
        double p = S[9];
        for (int n = 8; n >= 1; n--) {
            p = p * z;
            p = p + S[n];
        }
        double w = r * z;
        w = w * p;
        return r + w;
    }

    /** cos on the reduced interval. */
    private static double cosK(double r) {
        double z = r * r;
        double q = C[9];
        for (int n = 8; n >= 2; n--) {
            q = q * z;
            q = q + C[n];
        }
        double u = 0.5 * z;                 // exact
        u = 1.0 - u;
        double v = z * z;
        v = v * q;
        return u + v;
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
    // tempting rewrite  |x| > LIMIT  would let NaN through to the cast.

    /**
     * Compute the sine of {@code x} (in radians).
     *
     * <ol>
     *   <li>NaN, infinities and {@code |x| > 2^29}: NaN (outside the domain).</li>
     *   <li>{@code |x| < 2^-27}: {@code x} itself (keeps {@code -0} and subnormals).</li>
     *   <li>Otherwise reduce to {@code (r, q)} and pick the kernel by quadrant.</li>
     * </ol>
     *
     * @param x angle in radians
     * @return sine of x, or NaN when {@code |x| > 2^29} or x is not finite
     */
    public static double sin(double x) {
        double ax = x < 0.0 ? -x : x;       // |x|; NaN stays NaN
        if (!(ax <= LIMIT)) {
            return Double.NaN;
        }
        if (ax < TINY) {
            return x;                       // includes +0, -0 and every subnormal
        }
        Reduced red = reduce(x);
        switch (red.q) {
            case 0:  return sinK(red.r);
            case 1:  return cosK(red.r);
            case 2:  return -sinK(red.r);
            default: return -cosK(red.r);
        }
    }

    /**
     * Compute the cosine of {@code x} (in radians).
     *
     * @param x angle in radians
     * @return cosine of x, or NaN when {@code |x| > 2^29} or x is not finite
     */
    public static double cos(double x) {
        double ax = x < 0.0 ? -x : x;
        if (!(ax <= LIMIT)) {
            return Double.NaN;
        }
        if (ax < TINY) {
            return 1.0;
        }
        Reduced red = reduce(x);
        switch (red.q) {
            case 0:  return cosK(red.r);
            case 1:  return -sinK(red.r);
            case 2:  return -cosK(red.r);
            default: return sinK(red.r);
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
     * Convert degrees to radians: one multiply by {@code pi/180}.
     *
     * @param deg angle in degrees
     * @return equivalent angle in radians
     */
    public static double radians(double deg) {
        return deg * RAD_PER_DEG;
    }

    /**
     * Convert radians to degrees: one multiply by {@code 180/pi}.
     *
     * @param rad angle in radians
     * @return equivalent angle in degrees
     */
    public static double degrees(double rad) {
        return rad * DEG_PER_RAD;
    }

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
    // Why does it work?  If guess < sqrt(x), then x/guess > sqrt(x), so their
    // average "squeezes" both sides inward. Convergence is quadratic — each
    // iteration roughly doubles the number of correct digits.
    //
    //   Convergence for sqrt(2), starting guess = 2.0:
    //
    //   iter | guess                | error
    //   -----|----------------------|----------
    //   0    | 2.0                  | 0.58579
    //   1    | 1.5                  | 0.08579
    //   2    | 1.41667              | 0.00245
    //   3    | 1.41422              | 0.0000006
    //   4    | 1.41421356237...     | < 1e-13

    /**
     * Compute the square root of {@code x} using Newton's method.
     *
     * @param x a non-negative value
     * @return the square root of x
     * @throws ArithmeticException if x is negative
     */
    public static double sqrt(double x) {
        if (x < 0.0) {
            throw new ArithmeticException(
                "sqrt: domain error — input " + x + " is negative");
        }

        // sqrt(0) is exactly 0.
        if (x == 0.0) {
            return x;
        }

        if (x == Double.POSITIVE_INFINITY) {
            return x;
        }

        double scaled = x;
        double resultScale = 1.0;
        while (scaled < 0.25) {
            scaled *= 4.0;
            resultScale *= 0.5;
        }
        while (scaled >= 4.0) {
            scaled *= 0.25;
            resultScale *= 2.0;
        }

        // Initial guess: x itself for x >= 1 (good for large numbers),
        // 1.0 for x < 1 (avoids dividing by a tiny number in the first step).
        double guess = scaled >= 1.0 ? scaled : 1.0;

        // Iterate up to 60 times. Quadratic convergence means 60 is an extreme
        // safety margin; typical convergence happens in 15 or fewer iterations.
        for (int i = 0; i < 60; i++) {
            double next = (guess + scaled / guess) / 2.0;

            // Convergence criterion: stop when improvement is negligible.
            // 1e-15 * guess handles relative precision near large values.
            // 1e-300 is a floor to handle subnormal (near-zero) inputs safely.
            if (Math.abs(next - guess) < 1e-15 * guess + 1e-300) {
                return next * resultScale;
            }

            guess = next;
        }

        return guess * resultScale;
    }

    // =========================================================================
    // tan(x) — Tangent as Sine / Cosine
    // =========================================================================
    //
    // The tangent of an angle is the ratio of its sine to its cosine:
    //
    //   tan(x) = sin(x) / cos(x)
    //
    // This follows from the unit circle definition: a ray at angle x intersects
    // the unit circle at (cos x, sin x), and the slope of that ray from the
    // origin is sin(x)/cos(x). The name "tangent" comes from a tangent line
    // drawn to the unit circle at (1, 0) — the ray's intersection with that
    // line has y-coordinate equal to tan(x).
    //
    // Poles: tan is undefined at x = π/2 + k·π (where cos(x) = 0). We guard
    // against these by returning the largest finite double when |cos(x)| < 1e-15.

    /**
     * Compute the tangent of {@code x} (in radians).
     *
     * <p>Uses our own {@link #sin} and {@link #cos} — no {@code Math.tan}.
     * Outside {@code |x| <= 2^29} both are NaN, the pole test is false for NaN,
     * and {@code NaN / NaN} is NaN.  The pole sentinel is a pinned legacy
     * convention (PHY00 3.6), not an approximation, and no accuracy claim is
     * made for {@code tan}.
     *
     * @param x angle in radians
     * @return tangent of x (a very large finite value near poles)
     */
    public static double tan(double x) {
        double s = sin(x); // our own sin
        double c = cos(x); // our own cos

        // Guard against poles: when |cos(x)| < 1e-15 we're within a tiny
        // sliver of a discontinuity.  Return the largest finite double, signed
        // to match the direction of divergence.
        if (Math.abs(c) < 1e-15) {
            return s > 0.0 ? 1.0e308 : -1.0e308;
        }

        return s / c;
    }

    // =========================================================================
    // atan(x) — Arctangent via Taylor Series with Range Reduction
    // =========================================================================
    //
    // The Taylor series for atan:
    //
    //   atan(x) = x - x³/3 + x⁵/5 - x⁷/7 + ...   (for |x| <= 1)
    //
    // This converges for |x| <= 1 but slowly near x = 1.
    //
    // Two layers of range reduction:
    //
    // Layer 1 — for |x| > 1:
    //   atan(x)  =  π/2 - atan(1/x)    for x > 1
    //   atan(x)  = -π/2 - atan(1/x)    for x < -1
    //
    // Layer 2 — half-angle reduction (inside atanCore):
    //   atan(x) = 2 · atan( x / (1 + sqrt(1 + x²)) )
    //   This halves the argument so the series converges in ~15 terms.
    //
    // The half-angle identity comes from the double-angle formula for tangent:
    //   tan(2θ) = 2·tan(θ) / (1 - tan²(θ))
    // Solving for θ given tan(2θ) = x yields θ = atan( x / (1 + sqrt(1+x²)) ).

    /**
     * Inner atan computation for |x| &lt;= 1.
     *
     * <p>Applies half-angle reduction, then the Taylor series.
     * Private helper — callers should use {@link #atan}.
     */
    private static double atanCore(double x) {
        // Half-angle reduction: shrink |x| to |y| <= tan(pi/8) ~= 0.414.
        // Uses our own sqrt — no Math.sqrt.
        double reduced = x / (1.0 + sqrt(1.0 + x * x));

        // Taylor series for atan(reduced).
        //   term_0 = reduced
        //   term_n = term_{n-1} * (-t²) * (2n-1) / (2n+1)
        double t    = reduced;
        double tSq  = t * t;
        double term = t;
        double result = t;

        for (int n = 1; n <= 30; n++) {
            // The ratio (2n-1)/(2n+1) comes from the consecutive odd denominators
            // in the atan series: x/1, x³/3, x⁵/5, ... so the ratio is (2n-1)/(2n+1).
            term = term * (-tSq) * (2 * n - 1.0) / (2 * n + 1.0);
            result += term;

            // Early exit when terms are negligibly small.
            if (Math.abs(term) < 1e-17) {
                break;
            }
        }

        // Undo the half-angle: atan(x) = 2 * atan(reduced).
        return 2.0 * result;
    }

    /**
     * Compute the arctangent of {@code x} (in radians).
     *
     * <p>Return range: {@code (-pi/2, pi/2)}.
     *
     * @param x input value
     * @return angle whose tangent is x, in radians
     */
    public static double atan(double x) {
        // atan(x) rounds exactly to x here; avoid halving subnormals and retain -0.
        if (Math.abs(x) <= 7.450580596923828e-9) return x;

        if (x > 1.0)  return  HALF_PI - atanCore(1.0 / x);
        if (x < -1.0) return -HALF_PI - atanCore(1.0 / x);

        return atanCore(x);
    }

    // =========================================================================
    // atan2(y, x) — Four-Quadrant Arctangent
    // =========================================================================
    //
    // atan2(y, x) returns the angle in (-π, π] that the point (x, y) makes
    // with the positive x-axis.  It differs from atan(y/x) by correctly
    // handling all four quadrants and the special case where x = 0.
    //
    // Quadrant diagram:
    //
    //           y > 0
    //       Q2  |  Q1        atan2 in Q1: ( 0,   π/2)
    //     ------+------  x   atan2 in Q2: (π/2,  π  ]
    //       Q3  |  Q4        atan2 in Q3: (-π,  -π/2)
    //           y < 0        atan2 in Q4: (-π/2,  0 )
    //
    // Why can't we use atan(y/x) alone?
    //   Q4: atan(-1/1)   =   atan(-1) = -π/4          (correct)
    //   Q3: atan(-1/-1)  =   atan( 1) =  π/4  ← WRONG; should be -3π/4
    //
    // atan2 inspects the signs of BOTH y and x to determine the right quadrant.

    /**
     * Compute the four-quadrant arctangent of ({@code y}, {@code x}) in radians.
     *
     * <p>Return range: {@code (-pi, pi]}.
     *
     * @param y the y-coordinate
     * @param x the x-coordinate
     * @return the angle in (-pi, pi] that point (x, y) makes with the positive x-axis
     */
    public static double atan2(double y, double x) {
        // A NaN in either position is undefined; without this line a NaN x would
        // fall through every comparison to the "origin" branch and return 0.0.
        if (Double.isNaN(x) || Double.isNaN(y)) {
            return Double.NaN;
        }
        if (x > 0.0) {
            return atan(y / x);
        }
        if (x < 0.0 && y >= 0.0) {
            return atan(y / x) + PI;
        }
        if (x < 0.0 && y < 0.0) {
            return atan(y / x) - PI;
        }
        if (x == 0.0 && y > 0.0) {
            return  HALF_PI;
        }
        if (x == 0.0 && y < 0.0) {
            return -HALF_PI;
        }
        // x == 0.0 and y == 0.0: undefined, return 0 by convention.
        return 0.0;
    }
}
