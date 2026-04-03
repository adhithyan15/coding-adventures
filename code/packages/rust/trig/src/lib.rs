// ============================================================================
// trig — Trigonometric functions from first principles
// ============================================================================
//
// This library implements sin, cos, and angle-conversion functions using
// nothing but basic arithmetic.  No `std::f64::sin` or `libm` anywhere —
// every value is computed from the Maclaurin (Taylor-at-zero) series.
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
// Constants
// ============================================================================

/// The ratio of a circle's circumference to its diameter.
///
/// We hard-code this to the same precision as `std::f64::consts::PI` so that
/// our library is fully self-contained — no dependency on `std` constants.
pub const PI: f64 = 3.141592653589793;

/// Two times pi — a full revolution in radians.
///
/// Pre-computing this avoids a multiply every time we do range reduction.
const TWO_PI: f64 = 2.0 * PI;

// ============================================================================
// Range Reduction
// ============================================================================
//
// The Maclaurin series converges for any x, but convergence is slow for
// large |x|.  Worse, floating-point round-off accumulates when you raise a
// large number to the 39th power.
//
// The fix: since sin and cos are periodic with period 2*pi, we can always
// map x into the interval [-pi, pi] without changing the function's value.
//
//   1. Divide x by 2*pi and take the remainder  →  x is now in (-2pi, 2pi)
//   2. If x > pi, subtract 2*pi                 →  x is now in [-pi, pi]
//   3. If x < -pi, add 2*pi                     →  same guarantee
//
// After this, |x| <= pi ≈ 3.14, so our series terms shrink quickly.
//

/// Reduce `x` into the range [-pi, pi].
///
/// This preserves the value of any 2*pi-periodic function (sin, cos, etc.)
/// while keeping the magnitude small for faster series convergence.
fn range_reduce(x: f64) -> f64 {
    // Step 1: use the remainder operator to land in (-2*pi, 2*pi).
    let mut x = x % TWO_PI;

    // Step 2 & 3: nudge into [-pi, pi].
    if x > PI {
        x -= TWO_PI;
    }
    if x < -PI {
        x += TWO_PI;
    }

    x
}

// ============================================================================
// sin(x) — Maclaurin Series
// ============================================================================
//
// Recall the series:
//
//   sin(x) = x - x³/3! + x⁵/5! - x⁷/7! + ...
//
// Writing out successive terms, notice a pattern in how each term relates
// to the previous one:
//
//   term_0 = x
//   term_1 = term_0 * (-x²) / (2 * 3)
//   term_2 = term_1 * (-x²) / (4 * 5)
//   term_k = term_{k-1} * (-x²) / (2k * (2k+1))
//
// This "iterative term computation" avoids computing factorials (which
// overflow quickly) and avoids recomputing powers from scratch.
//

/// Compute the sine of `x` (in radians) using a 20-term Maclaurin series.
///
/// # How it works
///
/// 1. Range-reduce `x` to [-pi, pi] so the series converges quickly.
/// 2. Start with `term = x` (the first term of the series).
/// 3. Multiply by `-x² / (2k)(2k+1)` to get the next term.
/// 4. Accumulate 20 terms — more than enough for f64 precision.
///
/// # Examples
///
/// ```
/// assert!((trig::sin(0.0)).abs() < 1e-10);
/// assert!((trig::sin(trig::PI / 2.0) - 1.0).abs() < 1e-10);
/// ```
pub fn sin(x: f64) -> f64 {
    // --- Step 1: Range reduction -----------------------------------------
    let x = range_reduce(x);

    // --- Step 2: Pre-compute x² (we reuse it every iteration) ------------
    let x_squared = x * x;

    // --- Step 3: Iterative summation -------------------------------------
    //
    // `term` tracks the current series term.  We start with x (the k=0 term).
    // `sum` accumulates the running total.
    let mut term = x;
    let mut sum = term;

    for k in 1..20 {
        // Each new term is the previous term multiplied by:
        //   -x² / ( (2k) * (2k + 1) )
        //
        // The negation flips the sign each iteration (alternating series).
        // The denominator incorporates the next two factorial factors.
        let denom = (2 * k) as f64 * (2 * k + 1) as f64;
        term *= -x_squared / denom;
        sum += term;
    }

    sum
}

// ============================================================================
// cos(x) — Maclaurin Series
// ============================================================================
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

/// Compute the cosine of `x` (in radians) using a 20-term Maclaurin series.
///
/// # How it works
///
/// 1. Range-reduce `x` to [-pi, pi].
/// 2. Start with `term = 1` (the zeroth term).
/// 3. Multiply by `-x² / ((2k-1) * 2k)` to get the next term.
/// 4. Accumulate 20 terms.
///
/// # Examples
///
/// ```
/// assert!((trig::cos(0.0) - 1.0).abs() < 1e-10);
/// assert!((trig::cos(trig::PI) + 1.0).abs() < 1e-10);
/// ```
pub fn cos(x: f64) -> f64 {
    // --- Step 1: Range reduction -----------------------------------------
    let x = range_reduce(x);

    // --- Step 2: Pre-compute x² -----------------------------------------
    let x_squared = x * x;

    // --- Step 3: Iterative summation -------------------------------------
    //
    // `term` starts at 1 (the k=0 term of the cosine series).
    let mut term: f64 = 1.0;
    let mut sum = term;

    for k in 1..20 {
        // The denominator for cosine's k-th term uses (2k-1) and (2k),
        // which are the next two integers in the factorial sequence.
        let denom = (2 * k - 1) as f64 * (2 * k) as f64;
        term *= -x_squared / denom;
        sum += term;
    }

    sum
}

// ============================================================================
// Angle Conversion
// ============================================================================
//
// Degrees and radians are two ways to measure the same thing — how far
// around a circle you've gone.
//
//   360 degrees = 2*pi radians  →  1 degree = pi/180 radians
//                                   1 radian = 180/pi degrees
//

/// Convert degrees to radians.
///
/// # Formula
///
/// `radians = degrees * (pi / 180)`
///
/// # Examples
///
/// ```
/// assert!((trig::radians(180.0) - trig::PI).abs() < 1e-10);
/// ```
pub fn radians(deg: f64) -> f64 {
    deg * (PI / 180.0)
}

/// Convert radians to degrees.
///
/// # Formula
///
/// `degrees = radians * (180 / pi)`
///
/// # Examples
///
/// ```
/// assert!((trig::degrees(trig::PI) - 180.0).abs() < 1e-10);
/// ```
pub fn degrees(rad: f64) -> f64 {
    rad * (180.0 / PI)
}

// ============================================================================
// sqrt(x) — Newton's (Babylonian) Method
// ============================================================================
//
// Newton's method for square roots is one of the oldest algorithms in human
// history — it appears in Babylonian clay tablets from ~1700 BCE. The idea:
//
//   If `guess` is an approximation to sqrt(x), the average of `guess` and
//   `x / guess` is a better approximation.
//
// Why does it work? If guess < sqrt(x), then x/guess > sqrt(x), so the
// average "squeezes" both sides inward. The method has *quadratic convergence*
// — each iteration roughly doubles the number of correct digits.
//
// Convergence table for sqrt(2), starting guess = 2.0:
//
//   iter | guess                | error
//   -----|----------------------|----------
//   0    | 2.0                  | 0.58579
//   1    | 1.5                  | 0.08579
//   2    | 1.41667              | 0.00245
//   3    | 1.41422              | 0.0000006
//   4    | 1.41421356237...     | < 1e-13
//
// Converges in ~5 iterations for this example, ~15 for a generic double.

/// Compute the square root of `x` using Newton's method.
///
/// # Panics
///
/// Panics if `x < 0` — square roots of negative numbers are not real.
///
/// # Examples
///
/// ```
/// assert!((trig::sqrt(4.0) - 2.0).abs() < 1e-10);
/// assert!((trig::sqrt(2.0) - 1.41421356237).abs() < 1e-10);
/// ```
pub fn sqrt(x: f64) -> f64 {
    // Negative inputs are not in the domain of the real square root.
    if x < 0.0 {
        panic!("sqrt: domain error — input {} is negative", x);
    }

    // sqrt(0) is exactly 0.
    if x == 0.0 {
        return 0.0;
    }

    // Initial guess: x itself for x >= 1 (avoids a slow start for large values),
    // 1.0 for x < 1 (avoids the first step dividing by a tiny number).
    let mut guess = if x >= 1.0 { x } else { 1.0 };

    // Iterate up to 60 times. Quadratic convergence means 60 is an extreme
    // safety margin; typical convergence happens in 15 or fewer iterations.
    for _ in 0..60 {
        let next = (guess + x / guess) / 2.0;

        // Convergence criterion: stop when improvement is negligible.
        // 1e-15 * guess handles relative precision near large values.
        // 1e-300 is a floor to handle subnormal (near-zero) inputs safely.
        if (next - guess).abs() < 1e-15 * guess + 1e-300 {
            return next;
        }

        guess = next;
    }

    guess
}

// ============================================================================
// tan(x) — Tangent as Sine / Cosine
// ============================================================================
//
// The tangent of an angle is the ratio of its sine to its cosine:
//
//   tan(x) = sin(x) / cos(x)
//
// This follows from the unit circle definition: a ray at angle x intersects
// the unit circle at (cos x, sin x), and the slope of that ray from the
// origin is sin(x)/cos(x). The name "tangent" comes from a tangent line
// drawn to the unit circle at (1, 0) — the ray's intersection with that line
// has y-coordinate equal to tan(x).
//
// Undefined points (poles):
//   tan is undefined at x = π/2 + k·π (any integer k), where cos(x) = 0.
//   At these points the function shoots to ±∞. We guard with a threshold.

/// Compute the tangent of `x` (in radians).
///
/// Uses our own `sin` and `cos` — no `std::f64` math functions.
///
/// # Examples
///
/// ```
/// assert!((trig::tan(0.0)).abs() < 1e-10);
/// assert!((trig::tan(trig::PI / 4.0) - 1.0).abs() < 1e-10);
/// ```
pub fn tan(x: f64) -> f64 {
    let s = sin(x); // our own sin
    let c = cos(x); // our own cos

    // Guard against poles: when |cos(x)| < 1e-15 we're within a tiny sliver
    // of a discontinuity. Return the largest finite f64 (magnitude ~1.8e308)
    // with appropriate sign.
    if c.abs() < 1e-15 {
        return if s > 0.0 { 1.0e308_f64 } else { -1.0e308_f64 };
    }

    s / c
}

// ============================================================================
// atan(x) — Arctangent via Taylor Series with Range Reduction
// ============================================================================
//
// The Taylor series for atan:
//
//   atan(x) = x - x^3/3 + x^5/5 - x^7/7 + ...   (for |x| <= 1)
//
// This converges for |x| <= 1 but slowly near x = 1.
//
// Two layers of range reduction are applied:
//
// Layer 1 — for |x| > 1:
//   atan(x)  = π/2 - atan(1/x)    for x > 1
//   atan(x)  = -π/2 - atan(1/x)   for x < -1
//
// Layer 2 — half-angle reduction (inside atan_core):
//   atan(x) = 2·atan( x / (1 + sqrt(1 + x²)) )
//   This halves the argument, making the series converge in ~15 terms.
//
// The half-angle identity comes from the double-angle formula for tangent:
//   tan(2θ) = 2·tan(θ) / (1 - tan²(θ))
// If you set y = tan(θ) and x = tan(2θ), then θ = atan(y) = atan(x)/2,
// and solving for y gives y = x / (1 + sqrt(1 + x²)).

const HALF_PI: f64 = PI / 2.0;

/// Inner computation of atan for |x| <= 1.
///
/// Applies half-angle reduction then the Taylor series.
/// This is a private helper — users should call `atan` or `atan2`.
fn atan_core(x: f64) -> f64 {
    // Half-angle reduction: shrink |x| to |y| <= tan(π/8) ≈ 0.414.
    // We use our own `sqrt` here — no standard library math.
    let reduced = x / (1.0 + sqrt(1.0 + x * x));

    // Taylor series for atan(reduced).
    // term_0 = reduced
    // term_n = term_{n-1} * (-t^2) * (2n-1) / (2n+1)
    let t = reduced;
    let t_sq = t * t;
    let mut term = t;
    let mut result = t;

    for n in 1..=30 {
        // Each term multiplies by (-t²) and the ratio (2n-1)/(2n+1).
        // The (2n-1)/(2n+1) factor comes from the ratio of consecutive
        // odd denominators in the series: 1/1, 1/3, 1/5, 1/7, ...
        term = term * (-t_sq) * (2 * n - 1) as f64 / (2 * n + 1) as f64;
        result += term;

        // Early termination when the term is negligibly small.
        if term.abs() < 1e-17 {
            break;
        }
    }

    // Undo the half-angle halving.
    2.0 * result
}

/// Compute the arctangent of `x` (in radians), returning a value in (-π/2, π/2).
///
/// # Examples
///
/// ```
/// assert!((trig::atan(1.0) - trig::PI / 4.0).abs() < 1e-10);
/// assert!((trig::atan(0.0)).abs() < 1e-10);
/// ```
pub fn atan(x: f64) -> f64 {
    if x == 0.0 {
        return 0.0;
    }

    if x > 1.0 {
        return HALF_PI - atan_core(1.0 / x);
    }
    if x < -1.0 {
        return -HALF_PI - atan_core(1.0 / x);
    }

    atan_core(x)
}

// ============================================================================
// atan2(y, x) — Four-Quadrant Arctangent
// ============================================================================
//
// atan2(y, x) returns the angle in (-π, π] that the point (x, y) makes with
// the positive x-axis. It differs from atan(y/x) by correctly handling all
// four quadrants and the special cases where x = 0.
//
// Quadrant diagram:
//
//           y > 0
//       Q2  |  Q1        atan2 in Q1: (0,    π/2)
//     ------+------  x   atan2 in Q2: (π/2,  π  ]
//       Q3  |  Q4        atan2 in Q3: (-π,  -π/2)
//           y < 0        atan2 in Q4: (-π/2,  0 )
//
// Why can't we use atan(y/x) alone?
//   atan(-1/1) = -π/4   (Q4, correct)
//   atan(-1/-1) = -1/-1 = 1, atan(1) = π/4   (but point is in Q3, should be -3π/4!)

/// Compute the four-quadrant arctangent of (`y`, `x`) in radians.
///
/// Returns a value in the range (-π, π].
///
/// # Examples
///
/// ```
/// assert!((trig::atan2(0.0, 1.0)).abs() < 1e-10);      // positive x-axis → 0
/// assert!((trig::atan2(1.0, 0.0) - trig::PI/2.0).abs() < 1e-10);  // positive y-axis → π/2
/// ```
pub fn atan2(y: f64, x: f64) -> f64 {
    if x > 0.0 {
        atan(y / x)
    } else if x < 0.0 && y >= 0.0 {
        atan(y / x) + PI
    } else if x < 0.0 && y < 0.0 {
        atan(y / x) - PI
    } else if x == 0.0 && y > 0.0 {
        HALF_PI
    } else if x == 0.0 && y < 0.0 {
        -HALF_PI
    } else {
        // x == 0 and y == 0: undefined, return 0 by convention.
        0.0
    }
}
