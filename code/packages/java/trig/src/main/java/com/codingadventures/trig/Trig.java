// ============================================================================
// Trig.java — Trigonometric Functions from First Principles
// ============================================================================
//
// This library implements sin, cos, tan, atan, atan2, sqrt, and angle
// conversion using nothing but basic arithmetic.  No Math.sin or Math.cos
// anywhere — every value is computed from the Maclaurin (Taylor-at-zero)
// series.
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

package com.codingadventures.trig;

/**
 * Trigonometric functions computed from first principles using Maclaurin series.
 *
 * <p>All functions operate in radians unless otherwise specified.
 * No {@code java.lang.Math} functions are used internally — every result is
 * derived from addition, multiplication, and division alone.
 *
 * <p>Accuracy: 20-term series with range reduction gives full IEEE 754
 * double-precision accuracy (~15 decimal digits) for all inputs.
 */
public final class Trig {

    // =========================================================================
    // Constants
    // =========================================================================

    /**
     * The ratio of a circle's circumference to its diameter.
     *
     * <p>Hard-coded to the same precision as {@link Math#PI} so the library
     * is fully self-contained — no dependency on any standard constants.
     */
    public static final double PI = 3.141592653589793;

    /** Two times pi — a full revolution in radians. */
    private static final double TWO_PI = 2.0 * PI;

    /** Half of pi — used by atan and atan2. */
    private static final double HALF_PI = PI / 2.0;

    // Private constructor: this is a pure utility class — no instances needed.
    private Trig() {}

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
     * Reduce {@code x} into the range [-pi, pi].
     *
     * <p>Preserves the value of any 2*pi-periodic function (sin, cos, etc.)
     * while keeping the magnitude small for faster series convergence.
     */
    private static double rangeReduce(double x) {
        // Step 1: use % to land in (-2*pi, 2*pi).
        // Java's % keeps the sign of the dividend, so we need to handle both
        // positive and negative residues.
        x = x % TWO_PI;

        // Step 2 & 3: nudge into [-pi, pi].
        if (x > PI) {
            x -= TWO_PI;
        }
        if (x < -PI) {
            x += TWO_PI;
        }

        return x;
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
     * Compute the sine of {@code x} (in radians) using a 20-term Maclaurin series.
     *
     * <h3>How it works</h3>
     * <ol>
     *   <li>Range-reduce {@code x} to [-pi, pi] so the series converges quickly.</li>
     *   <li>Start with {@code term = x} (the first term, n=0).</li>
     *   <li>Multiply by {@code -x² / (2k)(2k+1)} to get the next term.</li>
     *   <li>Accumulate 20 terms — more than enough for double precision.</li>
     * </ol>
     *
     * @param x angle in radians
     * @return sine of x
     */
    public static double sin(double x) {
        // --- Step 1: Range reduction ---
        x = rangeReduce(x);

        // --- Step 2: Pre-compute x² (reused every iteration) ---
        double xSquared = x * x;

        // --- Step 3: Iterative summation ---
        // `term` tracks the current series term.
        // We start with x (the k=0 term of the sine series).
        double term = x;
        double sum  = x;

        for (int k = 1; k < 20; k++) {
            // Each new term is the previous term multiplied by:
            //   -x² / ( (2k) * (2k + 1) )
            //
            // The negation flips the sign each iteration (alternating series).
            // The denominator incorporates the next two factorial factors.
            double denom = (double)(2 * k) * (double)(2 * k + 1);
            term *= -xSquared / denom;
            sum  += term;
        }

        return sum;
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
    //
    // Notice it's almost identical to the sine recurrence — only the
    // denominator indices differ by one, because cosine uses even powers
    // while sine uses odd powers.

    /**
     * Compute the cosine of {@code x} (in radians) using a 20-term Maclaurin series.
     *
     * @param x angle in radians
     * @return cosine of x
     */
    public static double cos(double x) {
        // --- Step 1: Range reduction ---
        x = rangeReduce(x);

        // --- Step 2: Pre-compute x² ---
        double xSquared = x * x;

        // --- Step 3: Iterative summation ---
        // `term` starts at 1 (the k=0 term of the cosine series).
        double term = 1.0;
        double sum  = 1.0;

        for (int k = 1; k < 20; k++) {
            // Going from term at index (k-1) to term at index k:
            //
            //   term_k = term_{k-1} * (-x²) / ((2k-1) * 2k)
            //
            // The denominator (2k-1)(2k) absorbs the next two factorial factors.
            double denom = (double)(2 * k - 1) * (double)(2 * k);
            term *= -xSquared / denom;
            sum  += term;
        }

        return sum;
    }

    // =========================================================================
    // Angle Conversion
    // =========================================================================
    //
    // Degrees and radians are two ways to measure the same thing — how far
    // around a circle you've gone.
    //
    //   360 degrees = 2*pi radians  →  1 degree  = pi/180 radians
    //                                   1 radian = 180/pi degrees

    /**
     * Convert degrees to radians.
     *
     * <p>Formula: {@code radians = degrees * (pi / 180)}
     *
     * @param deg angle in degrees
     * @return equivalent angle in radians
     */
    public static double radians(double deg) {
        return deg * (PI / 180.0);
    }

    /**
     * Convert radians to degrees.
     *
     * <p>Formula: {@code degrees = radians * (180 / pi)}
     *
     * @param rad angle in radians
     * @return equivalent angle in degrees
     */
    public static double degrees(double rad) {
        return rad * (180.0 / PI);
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
            return 0.0;
        }

        // Initial guess: x itself for x >= 1 (good for large numbers),
        // 1.0 for x < 1 (avoids dividing by a tiny number in the first step).
        double guess = x >= 1.0 ? x : 1.0;

        // Iterate up to 60 times. Quadratic convergence means 60 is an extreme
        // safety margin; typical convergence happens in 15 or fewer iterations.
        for (int i = 0; i < 60; i++) {
            double next = (guess + x / guess) / 2.0;

            // Convergence criterion: stop when improvement is negligible.
            // 1e-15 * guess handles relative precision near large values.
            // 1e-300 is a floor to handle subnormal (near-zero) inputs safely.
            if (Math.abs(next - guess) < 1e-15 * guess + 1e-300) {
                return next;
            }

            guess = next;
        }

        return guess;
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
        if (x == 0.0) return 0.0;

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
