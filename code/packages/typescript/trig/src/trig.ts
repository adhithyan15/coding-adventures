// ============================================================================
// trig.ts — Trigonometric Functions from First Principles
// ============================================================================
//
// This module computes sin(x) and cos(x) using their Maclaurin series
// (Taylor series centered at zero). No built-in Math.sin or Math.cos
// is used — everything is derived from basic arithmetic.
//
// Why Taylor series?
// ------------------
// Brook Taylor (1715) and Colin Maclaurin (1742) showed that smooth
// functions can be expressed as infinite sums of polynomial terms.
// For trigonometry, these series converge for ALL real numbers,
// making them a universal computation method.
//
// The Maclaurin series for sine:
//
//   sin(x) = x - x³/3! + x⁵/5! - x⁷/7! + ...
//          = Σ (-1)^n · x^(2n+1) / (2n+1)!    for n = 0, 1, 2, ...
//
// The Maclaurin series for cosine:
//
//   cos(x) = 1 - x²/2! + x⁴/4! - x⁶/6! + ...
//          = Σ (-1)^n · x^(2n) / (2n)!        for n = 0, 1, 2, ...
//
// Each successive term is smaller than the last (for |x| < some bound),
// so the series converges quickly — especially after range reduction.
// ============================================================================

// ----------------------------------------------------------------------------
// Constants
// ----------------------------------------------------------------------------
// PI to double-precision accuracy (same as Math.PI). This is the ratio of
// a circle's circumference to its diameter — the most fundamental constant
// in trigonometry.

export const PI = 3.141592653589793;

// TWO_PI represents one full revolution (360°). We use it for range
// reduction: since sin and cos are periodic with period 2π, we can
// always reduce the input to a small range before computing the series.

const TWO_PI = 2.0 * PI;

// ----------------------------------------------------------------------------
// Range Reduction
// ----------------------------------------------------------------------------
// The Taylor series converges fastest when |x| is small. Since sin and cos
// repeat every 2π, we can subtract multiples of 2π to bring x into [-π, π].
//
// Example: sin(1000π) = sin(0) = 0, because 1000π is an exact multiple of π.
//
// Without range reduction, computing sin(1000) would require many more terms
// to converge, and floating-point errors would accumulate badly.

function rangeReduce(x: number): number {
  // Step 1: Reduce modulo 2π to get into [0, 2π) or (-2π, 0]
  x = x % TWO_PI;

  // Step 2: Shift into [-π, π] for optimal convergence
  // If x > π, subtract 2π to bring it to the negative side
  if (x > PI) {
    x -= TWO_PI;
  }
  // If x < -π, add 2π to bring it to the positive side
  if (x < -PI) {
    x += TWO_PI;
  }

  return x;
}

// ----------------------------------------------------------------------------
// sin(x) — Sine via Maclaurin Series
// ----------------------------------------------------------------------------
// Computes sin(x) using the iterative form of the Maclaurin series.
//
// Instead of computing x^n and n! separately (which overflow quickly),
// we compute each term from the previous one:
//
//   term_0 = x
//   term_n = term_{n-1} * (-x²) / ((2n)(2n+1))
//
// This is numerically stable and avoids large intermediate values.
//
// We use 20 terms, which gives approximately 15 digits of precision
// for inputs in [-π, π] — more than enough for double-precision floats.

export function sin(x: number): number {
  x = rangeReduce(x);

  let sum = 0.0;
  let term = x; // First term: x¹/1! = x

  for (let n = 1; n <= 20; n++) {
    // Accumulate this term into the running sum
    sum += term;

    // Derive the next term from the current one:
    //   next = current * (-x²) / ((2n)(2n+1))
    //
    // This works because:
    //   term_n   = (-1)^(n-1) · x^(2(n-1)+1) / (2(n-1)+1)!
    //   term_n+1 = (-1)^n     · x^(2n+1)     / (2n+1)!
    //   ratio    = -x² / ((2n)(2n+1))
    term *= (-x * x) / (2 * n * (2 * n + 1));
  }

  return sum;
}

// ----------------------------------------------------------------------------
// cos(x) — Cosine via Maclaurin Series
// ----------------------------------------------------------------------------
// Same approach as sin(x), but starting from term_0 = 1 (the constant term)
// and using even powers of x:
//
//   term_0 = 1
//   term_n = term_{n-1} * (-x²) / ((2n-1)(2n))
//
// The recurrence ratio differs slightly from sine because cosine uses
// even-indexed terms: x⁰/0!, x²/2!, x⁴/4!, etc.

export function cos(x: number): number {
  x = rangeReduce(x);

  let sum = 0.0;
  let term = 1.0; // First term: x⁰/0! = 1

  for (let n = 1; n <= 20; n++) {
    // Accumulate this term
    sum += term;

    // Derive next term:
    //   next = current * (-x²) / ((2n-1)(2n))
    //
    // The denominator uses (2n-1)(2n) because cosine terms are:
    //   1, -x²/2!, x⁴/4!, -x⁶/6!, ...
    // and 2!/0! = 2·1, 4!/2! = 4·3, 6!/4! = 6·5, etc.
    term *= (-x * x) / ((2 * n - 1) * (2 * n));
  }

  return sum;
}

// ----------------------------------------------------------------------------
// Unit Conversions
// ----------------------------------------------------------------------------
// Degrees and radians are two ways to measure angles:
//   - Degrees: a full circle = 360°  (convenient for humans)
//   - Radians: a full circle = 2π    (natural for mathematics)
//
// Conversion formulas:
//   radians = degrees × (π / 180)
//   degrees = radians × (180 / π)

export function radians(deg: number): number {
  return deg * (PI / 180.0);
}

export function degrees(rad: number): number {
  return rad * (180.0 / PI);
}

// ----------------------------------------------------------------------------
// sqrt(x) — Square Root via Newton's (Babylonian) Method
// ----------------------------------------------------------------------------
// Newton's method for square roots is one of the oldest numerical algorithms,
// known to Babylonian mathematicians over 3,000 years ago. The idea is simple:
// if `guess` is an approximation to sqrt(x), then the average of `guess` and
// `x / guess` is a better approximation.
//
// Why does this work? If guess < sqrt(x), then x/guess > sqrt(x), so their
// average lands somewhere closer to the true value. If guess > sqrt(x), the
// argument is symmetric. In either case, we move closer.
//
// The convergence is *quadratic* — the number of correct decimal digits
// doubles with each iteration. For x = 2:
//
//   Iteration 0: guess = 2.0
//   Iteration 1: guess = (2.0 + 2.0/2.0) / 2 = 1.5
//   Iteration 2: guess = (1.5 + 2.0/1.5) / 2 ≈ 1.41667
//   Iteration 3: guess = ≈ 1.41422
//   Iteration 4: guess = ≈ 1.41421356237...  (converged!)
//
// Typically converges in 10-15 iterations for most double-precision inputs.

export function sqrt(x: number): number {
  // Negative inputs are mathematically undefined (in real numbers).
  // We throw an error to signal clearly rather than returning NaN silently.
  if (x < 0) {
    throw new Error(`sqrt: domain error — input ${x} is negative`);
  }

  // sqrt(0) is exactly 0 by definition.
  if (x === 0.0) return 0.0;

  // Initial guess: use x itself for x >= 1 (decent for large numbers),
  // and 1.0 for x < 1 (avoids dividing by a very small number early on).
  // Both converge correctly; this just saves a few iterations.
  let guess = x >= 1.0 ? x : 1.0;

  // Iterate up to 60 times. In practice the loop exits in ~15 iterations
  // due to quadratic convergence. The cap prevents infinite loops on
  // edge cases near floating-point subnormals.
  for (let i = 0; i < 60; i++) {
    const next = (guess + x / guess) / 2.0;

    // Convergence check: stop when the improvement is smaller than the
    // best precision we can hope for at this magnitude.
    // The 1e-15 * guess term accounts for relative precision near large values.
    // The 1e-300 absolute floor handles subnormals safely.
    if (Math.abs(next - guess) < 1e-15 * guess + 1e-300) {
      return next;
    }

    guess = next;
  }

  return guess;
}

// ----------------------------------------------------------------------------
// tan(x) — Tangent as Sine / Cosine
// ----------------------------------------------------------------------------
// Tangent is defined geometrically as the ratio of the opposite side to the
// adjacent side in a right triangle, which on the unit circle becomes:
//
//   tan(x) = sin(x) / cos(x)
//
// This is the standard definition. We use our own sin and cos here (not
// any built-in Math functions) to keep the module self-contained.
//
// Where is tangent undefined?
// At x = π/2 + k·π for any integer k, cos(x) = 0, so the ratio is undefined.
// The tangent function "blows up" — approaching +∞ from the left and −∞
// from the right. We guard against cos(x) being too close to zero and return
// a large finite number to signal the near-singularity.
//
// Visual: The tangent function on [-π, π]:
//
//    y
//    |        /
//    |       /
//  __|______/_____ x
//    |   /
//    |  /
//    | /
//    |/               ← tan(0) = 0
//   /|
//  / |
// ∞ at ±π/2

export function tan(x: number): number {
  const s = sin(x); // our own sin — no Math.sin
  const c = cos(x); // our own cos — no Math.cos

  // Guard against poles: when |cos(x)| < 1e-15, we are extremely close
  // to x = π/2 + k·π. Return the largest representable float with the
  // appropriate sign to indicate the direction of divergence.
  if (Math.abs(c) < 1e-15) {
    return s > 0 ? 1.0e308 : -1.0e308;
  }

  return s / c;
}

// ----------------------------------------------------------------------------
// HALF_PI — Used internally by atan and atan2
// ----------------------------------------------------------------------------
// This is π/2 ≈ 1.5707963267948966. It appears in atan's range reduction
// (atan(x) = π/2 - atan(1/x) for x > 1) and in atan2's quadrant rules.

const HALF_PI = PI / 2.0;

// ----------------------------------------------------------------------------
// atan_core(x) — Taylor Series for |x| ≤ 1, After Half-Angle Reduction
// ----------------------------------------------------------------------------
// This is the inner workhorse of atan. It computes atan(x) for |x| ≤ 1
// by first applying the half-angle identity to shrink the argument further,
// then applying the Taylor series.
//
// Why not just apply the Taylor series directly?
// atan(x) = x - x³/3 + x⁵/5 - x⁷/7 + ...  (for |x| ≤ 1)
// Near x = 1, convergence is slow: atan(1) = π/4, but we need ~50 terms.
//
// Fix — half-angle identity:
//   atan(x) = 2·atan( x / (1 + sqrt(1 + x²)) )
// This reduces the argument by roughly half each application. One application
// brings |x| ≤ 1 down to |x| ≤ tan(π/8) ≈ 0.414, where the series
// converges in ~15 terms with 17-digit accuracy.
//
// Then we multiply the series result by 2 to undo the identity.

function atanCore(x: number): number {
  // --- Half-angle reduction ---
  // Let y = x / (1 + sqrt(1 + x²)).
  // Then atan(x) = 2·atan(y), and |y| ≤ tan(π/8) ≈ 0.414.
  //
  // We use our own sqrt here (defined above) — no Math.sqrt.
  const reduced = x / (1.0 + sqrt(1.0 + x * x));

  // --- Taylor series for atan(reduced) ---
  // atan(t) = t - t³/3 + t⁵/5 - t⁷/7 + ...
  //
  // Iterative form: term_0 = t, term_n = term_{n-1} * (-t²) * (2n-1)/(2n+1)
  //
  // Why the (2n-1)/(2n+1) factor? Because consecutive terms differ by:
  //   t^(2n+1) / (2n+1)   divided by   t^(2n-1) / (2n-1)
  //   = t² × (2n-1) / (2n+1)
  // We include the sign flip from -t² in the ratio.
  const t = reduced;
  const tSq = t * t;
  let term = t;
  let result = t;

  for (let n = 1; n <= 30; n++) {
    // Advance to the next term of the alternating series.
    // Multiply by -t² and scale by (2n-1)/(2n+1) to hit the next odd power.
    term = term * (-tSq) * (2 * n - 1) / (2 * n + 1);
    result += term;

    // Early exit when the term is negligibly small.
    if (Math.abs(term) < 1e-17) break;
  }

  // Undo the half-angle halving: atan(x) = 2·atan(y).
  return 2.0 * result;
}

// ----------------------------------------------------------------------------
// atan(x) — Four-Quadrant Arctangent (Single-Argument)
// ----------------------------------------------------------------------------
// atan(x) is the inverse of tan: given a ratio, return the angle.
// Its range is (-π/2, π/2) — a single semicircle.
//
// We need range reduction because the Taylor series converges only for |x| ≤ 1.
// For |x| > 1, we use the complementary identity:
//
//   atan(x) = π/2 - atan(1/x)    for x > 1
//   atan(x) = -π/2 - atan(1/x)   for x < -1
//
// This follows from: atan(x) + atan(1/x) = π/2  (for x > 0).
//
// Proof sketch: if θ = atan(x), then tan(θ) = x, so tan(π/2 - θ) = 1/x
// (because tan and cot are complementary), meaning atan(1/x) = π/2 - θ.

export function atan(x: number): number {
  // Special case: atan(0) = 0 exactly.
  if (x === 0.0) return 0.0;

  // Reduce |x| > 1 using the complementary identity.
  if (x > 1.0) {
    return HALF_PI - atanCore(1.0 / x);
  }
  if (x < -1.0) {
    return -HALF_PI - atanCore(1.0 / x);
  }

  // |x| ≤ 1: compute directly via the core routine.
  return atanCore(x);
}

// ----------------------------------------------------------------------------
// atan2(y, x) — Four-Quadrant Arctangent
// ----------------------------------------------------------------------------
// Standard atan(y/x) only returns angles in (-π/2, π/2) — the right half-
// plane. atan2 returns the angle in all four quadrants: (-π, π].
//
// Why is this needed? If y = 1 and x = -1, then y/x = -1, and atan(-1) = -π/4.
// But the actual angle (the one pointing to (-1, 1) in the plane) is 3π/4 —
// in the second quadrant. atan2 handles this correctly by inspecting the
// signs of both y and x.
//
// Quadrant diagram:
//
//        y > 0
//    Q2  |  Q1        atan2 > 0 in Q1, Q2
//  ------+------  x   atan2 < 0 in Q3, Q4
//    Q3  |  Q4        atan2 = ±π on negative x-axis
//        y < 0

export function atan2(y: number, x: number): number {
  if (x > 0.0) {
    // First or fourth quadrant: standard atan works fine.
    return atan(y / x);
  }
  if (x < 0.0 && y >= 0.0) {
    // Second quadrant (or negative x-axis with y = 0 → returns π).
    return atan(y / x) + PI;
  }
  if (x < 0.0 && y < 0.0) {
    // Third quadrant.
    return atan(y / x) - PI;
  }
  if (x === 0.0 && y > 0.0) {
    // Positive y-axis.
    return HALF_PI;
  }
  if (x === 0.0 && y < 0.0) {
    // Negative y-axis.
    return -HALF_PI;
  }
  // Both zero: undefined. Return 0 by convention (matches C's atan2).
  return 0.0;
}
