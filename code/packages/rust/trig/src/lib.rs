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
