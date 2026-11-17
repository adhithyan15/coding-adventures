// ============================================================================
// trig — Trigonometric functions from first principles (TRIG-REF-1)
// ============================================================================
//
// `sin`, `cos`, `radians` and `degrees` are built from addition, subtraction,
// multiplication and comparison alone: no `f64::sin`, no libm, no division.
// Every operation rounds once, nothing is fused (Rust never contracts `a*b+c`;
// we never call `mul_add`), so the answer is *bit for bit* the same in all
// seventeen language ports.  The algorithm is PHY00a
// (code/specs/PHY00a-trig-convergence.md); the shared vectors are
// code/specs/fixtures/trig-reference-v1.
//
// The big idea: a sine repeats, so an angle splits into whole quarter-turns
// and a small leftover,
//
//     x = k * (pi/2) + r,      |r| <= pi/4,      k a whole number
//
// `sin x` is one of  sin r, cos r, -sin r, -cos r  chosen by k mod 4, and
// `cos x` one of  cos r, -sin r, -cos r, sin r.
//
// Step 1, the quotient.  `MAGIC = 1.5 * 2^52` has its units digit in the last
// place of the significand, so `(t + MAGIC) - MAGIC` rounds t to the nearest
// whole number with only `+` and `-` (no floor, no cast, nothing special at NaN).
//
// Step 2, the remainder (Cody-Waite).  pi/2 is irrational, so a 53-bit pi/2
// times a big k loses the digits we need.  We store pi/2 as FIVE pieces, the
// first four with at most 24 significant bits so that k * piece is exact
// (24 + 29 bits = 53):
//
//   x ---------------------------------------------------------- the angle
//   |-- k*PIO2_1 --|                                              24 bits
//                  |-- k*PIO2_2 --|                               next 24
//                                 |-- k*PIO2_3 --|                next 24
//                                                |- k*PIO2_4 -|   next 24
//                                                             |k*PIO2_5| rounds
//
// Step 3, the kernels: Maclaurin series on |r| <= pi/4, Horner's rule,
// coefficients (-1)^n/(2n+1)! and (-1)^n/(2n)! stored as bit patterns:
//
//   sin r = r + r*z*(S1 + S2 z + ... + S9 z^8)           z = r*r
//   cos r = 1 - z/2 + z*z*(C2 + C3 z + ... + C9 z^7)
//
// Domain: |x| <= 2^29.  NaN, the infinities and larger values return NaN (a
// finite answer there would be wrong but look right).  Accuracy: 3 ulp.
// ============================================================================

// ============================================================================
// Constants (PHY00a 3.2): bit patterns are authoritative
// ============================================================================

/// The ratio of a circle's circumference to its diameter (`0x400921FB54442D18`).
// Intentionally hand-written: the crate does not depend on std's constant.
#[allow(clippy::approx_constant)]
pub const PI: f64 = 3.141592653589793;

/// 2/pi, the double nearest to it.
const INV_PIO2: u64 = 0x3FE45F306DC9C883;
/// pi/2 as five pieces: four of at most 24 significant bits, then the remainder.
const PIO2: [u64; 5] = [
    0x3FF921FB40000000,
    0x3E74442D00000000,
    0x3CF8469880000000,
    0x3B78CC5160000000,
    0x39F01B839A25204A,
];
/// 1.5 * 2^52: add then subtract it to round to the nearest integer.
const MAGIC: u64 = 0x4338000000000000;
/// The edge of the domain, 2^29.
const LIMIT: u64 = 0x41C0000000000000;
/// Below 2^-27, sin(x) == x exactly and cos(x) == 1.
const TINY: u64 = 0x3E40000000000000;
/// pi/180 and 180/pi, pre-divided so a conversion is one multiply.
const RAD_PER_DEG: u64 = 0x3F91DF46A2529D39;
const DEG_PER_RAD: u64 = 0x404CA5DC1A63C1F8;
/// Sine coefficients S[n] = (-1)^n/(2n+1)!, n = 1..9 (index 0 here is n = 1):
/// -1/3!, +1/5!, -1/7!, +1/9!, -1/11!, +1/13!, -1/15!, +1/17!, -1/19!
const SC: [u64; 9] = [
    0xBFC5555555555555,
    0x3F81111111111111,
    0xBF2A01A01A01A01A,
    0x3EC71DE3A556C734,
    0xBE5AE64567F544E4,
    0x3DE6124613A86D09,
    0xBD6AE7F3E733B81F,
    0x3CE952C77030AD4A,
    0xBC62F49B46814157,
];
/// Cosine coefficients C[n] = (-1)^n/(2n)!, n = 2..9 (index 0 here is n = 2):
/// +1/4!, -1/6!, +1/8!, -1/10!, +1/12!, -1/14!, +1/16!, -1/18!
const CC: [u64; 8] = [
    0x3FA5555555555555,
    0xBF56C16C16C16C17,
    0x3EFA01A01A01A01A,
    0xBE927E4FB7789F5C,
    0x3E21EED8EFF8D898,
    0xBDA93974A8C07C9D,
    0x3D2AE7F3E733B81F,
    0xBCA6827863B97D97,
];

#[inline]
fn f(bits: u64) -> f64 {
    f64::from_bits(bits)
}

// ============================================================================
// Range reduction and kernels
// ============================================================================

/// `(k, r)` with `x = k*(pi/2) + r`, `|r| <= pi/4` (about).  Precondition:
/// `|x| <= 2^29`, so `k` is a finite whole number below 2^29.  Every product and
/// the subtraction that uses it are separate statements: they must stay two
/// roundings, never a fused multiply-add.
fn reduce(x: f64) -> (f64, f64) {
    let t = x * f(INV_PIO2);
    let k = (t + f(MAGIC)) - f(MAGIC);
    let mut r = x;
    for p in PIO2 {
        let m = k * f(p);
        r -= m;
    }
    (k, r)
}

/// sin(r) for |r| <= pi/4, Horner on the stored coefficients.
fn sin_k(r: f64) -> f64 {
    let z = r * r;
    let mut p = f(SC[8]);
    for n in (0..8).rev() {
        p *= z;
        p += f(SC[n]);
    }
    let mut w = r * z;
    w *= p;
    r + w
}

/// cos(r) for |r| <= pi/4: (1 - z/2) + z*z*(C2 + ... + C9 z^7).
fn cos_k(r: f64) -> f64 {
    let z = r * r;
    let mut q = f(CC[7]);
    for n in (0..7).rev() {
        q *= z;
        q += f(CC[n]);
    }
    let mut u = 0.5 * z; // exact
    u = 1.0 - u;
    let mut v = z * z;
    v *= q;
    u + v
}

// ============================================================================
// Sine and cosine
// ============================================================================

/// Sine of `x` radians for `|x| <= 2^29`; NaN for NaN, the infinities and
/// anything larger.  `sin(-0.0)` is `-0.0`; `|x| < 2^-27` returns `x`.
///
/// The guard is a *negated* `<=`: a NaN fails every comparison, so one test
/// covers NaN, both infinities and every finite value beyond the domain.
#[allow(clippy::neg_cmp_op_on_partial_ord)]
pub fn sin(x: f64) -> f64 {
    if !(x.abs() <= f(LIMIT)) {
        return f64::NAN;
    }
    if x.abs() < f(TINY) {
        return x;
    }
    let (k, r) = reduce(x);
    // k is a whole number below 2^29: the cast is exact, `& 3` is k mod 4
    // (two's complement, so -1 & 3 == 3).
    match (k as i64) & 3 {
        0 => sin_k(r),
        1 => cos_k(r),
        2 => -sin_k(r),
        _ => -cos_k(r),
    }
}

/// Cosine of `x` radians for `|x| <= 2^29`; NaN outside the domain.
/// `cos(+-0.0) == 1`, and `cos(x) == 1` for `|x| < 2^-27`.
#[allow(clippy::neg_cmp_op_on_partial_ord)]
pub fn cos(x: f64) -> f64 {
    if !(x.abs() <= f(LIMIT)) {
        return f64::NAN;
    }
    if x.abs() < f(TINY) {
        return 1.0;
    }
    let (k, r) = reduce(x);
    match (k as i64) & 3 {
        0 => cos_k(r),
        1 => -sin_k(r),
        2 => -cos_k(r),
        _ => sin_k(r),
    }
}

/// Degrees to radians: one multiply by a pre-divided, bit-pinned constant.
pub fn radians(deg: f64) -> f64 {
    deg * f(RAD_PER_DEG)
}

/// Radians to degrees: one multiply by a pre-divided, bit-pinned constant.
pub fn degrees(rad: f64) -> f64 {
    rad * f(DEG_PER_RAD)
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
        return x;
    }
    if x.is_infinite() {
        return x;
    }

    let mut scaled = x;
    let mut result_scale = 1.0;
    while scaled < 0.25 {
        scaled *= 4.0;
        result_scale *= 0.5;
    }
    while scaled >= 4.0 {
        scaled *= 0.25;
        result_scale *= 2.0;
    }

    // Initial guess: x itself for x >= 1 (avoids a slow start for large values),
    // 1.0 for x < 1 (avoids the first step dividing by a tiny number).
    let mut guess = if scaled >= 1.0 { scaled } else { 1.0 };

    // Iterate up to 60 times. Quadratic convergence means 60 is an extreme
    // safety margin; typical convergence happens in 15 or fewer iterations.
    for _ in 0..60 {
        let next = (guess + scaled / guess) / 2.0;

        // Convergence criterion: stop when improvement is negligible.
        // 1e-15 * guess handles relative precision near large values.
        // 1e-300 is a floor to handle subnormal (near-zero) inputs safely.
        if (next - guess).abs() < 1e-15 * guess + 1e-300 {
            return next * result_scale;
        }

        guess = next;
    }

    guess * result_scale
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
    // atan(x) rounds exactly to x here; avoid halving subnormals and retain -0.
    if x.abs() <= 7.450580596923828e-9 {
        return x;
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
    // A NaN fails every comparison below and would fall through to 0.0; a
    // finite answer to an undefined question is wrong (PHY00a 3.9).
    if x.is_nan() || y.is_nan() {
        return f64::NAN;
    }
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

// ============================================================================
// Unit tests: the literals written in this file are the pinned bit patterns
// ============================================================================
//
// "The literal parses to the nearest double" is a property of the compiler, so
// the decimal literals the algorithm uses are asserted bit for bit.

#[cfg(test)]
mod constant_bits {
    use super::*;

    #[test]
    fn decimal_literals_have_the_pinned_bits() {
        assert_eq!(PI.to_bits(), 0x400921FB54442D18);
        assert_eq!(HALF_PI.to_bits(), 0x3FF921FB54442D18);
        assert_eq!(1e-15_f64.to_bits(), 0x3CD203AF9EE75616);
        assert_eq!(1.0e308_f64.to_bits(), 0x7FE1CCF385EBC8A0);
        assert_eq!(1e-17_f64.to_bits(), 0x3C670EF54646D497);
        assert_eq!(1e-300_f64.to_bits(), 0x01A56E1FC2F8F359);
        assert_eq!(7.450580596923828e-9_f64.to_bits(), f(TINY).to_bits());
    }

    #[test]
    fn degree_constants_equal_the_divisions() {
        assert_eq!(f(RAD_PER_DEG), PI / 180.0);
        assert_eq!(f(DEG_PER_RAD), 180.0 / PI);
    }

    #[test]
    fn the_reduction_pieces_have_at_most_24_significant_bits() {
        for &p in &PIO2[..4] {
            let mantissa = (p & 0x000F_FFFF_FFFF_FFFF) | (1 << 52);
            assert!(53 - mantissa.trailing_zeros() <= 24);
        }
    }
}
