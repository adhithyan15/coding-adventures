// ============================================================================
// trig — Trigonometric functions from first principles (TRIG-REF-1)
// ============================================================================
//
// `sin`, `cos`, `radians` and `degrees` use addition, subtraction,
// multiplication and comparison only: no Math.sin, no division.  JavaScript has
// one number type (binary64), every operation rounds once and nothing is fused,
// so the answer is bit for bit the same in all seventeen language ports.  The
// algorithm is PHY00a (code/specs/PHY00a-trig-convergence.md); the shared
// vectors are code/specs/fixtures/trig-reference-v1.
//
// The big idea: a sine repeats, so an angle splits into whole quarter-turns and
// a small leftover,
//
//     x = k * (pi/2) + r,      |r| <= pi/4,      k a whole number
//
// sin x is one of  sin r, cos r, -sin r, -cos r  chosen by k mod 4, and cos x
// one of  cos r, -sin r, -cos r, sin r.
//
// Step 1, the quotient.  MAGIC = 1.5 * 2^52 has its units digit in the last
// place of the significand, so (t + MAGIC) - MAGIC rounds t to the nearest whole
// number with only + and - (no floor, no cast, nothing special at NaN).
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

// ----------------------------------------------------------------------------
// Constants (PHY00a 3.2): bit patterns are authoritative.
// ----------------------------------------------------------------------------

const scratch = new DataView(new ArrayBuffer(8));

/** The double whose IEEE 754 bit pattern is `bits` (a decimal parser cannot misround it). */
export function fromBits(bits: bigint): number {
  scratch.setBigUint64(0, bits);
  return scratch.getFloat64(0);
}

/** The IEEE 754 bit pattern of `x`. */
export function toBits(x: number): bigint {
  scratch.setFloat64(0, x);
  return scratch.getBigUint64(0);
}

/** The ratio of a circle's circumference to its diameter (0x400921FB54442D18). */
export const PI = 3.141592653589793;

/** 2/pi, the double nearest to it. */
export const INV_PIO2 = fromBits(0x3fe45f306dc9c883n);
/** pi/2 as five pieces: four of at most 24 significant bits, then the remainder. */
export const PIO2: readonly number[] = [
  fromBits(0x3ff921fb40000000n),
  fromBits(0x3e74442d00000000n),
  fromBits(0x3cf8469880000000n),
  fromBits(0x3b78cc5160000000n),
  fromBits(0x39f01b839a25204an),
];
/** 1.5 * 2^52: add then subtract it to round to the nearest integer. */
export const MAGIC = fromBits(0x4338000000000000n);
/** The edge of the domain, 2^29. */
export const LIMIT = fromBits(0x41c0000000000000n);
/** Below 2^-27, sin(x) === x exactly and cos(x) === 1. */
export const TINY = fromBits(0x3e40000000000000n);
/** pi/180 and 180/pi, pre-divided so a conversion is one multiply. */
export const RAD_PER_DEG = fromBits(0x3f91df46a2529d39n);
export const DEG_PER_RAD = fromBits(0x404ca5dc1a63c1f8n);

/**
 * Sine coefficients S[n] = (-1)^n/(2n+1)!, n = 1..9 (index 0 is unused):
 * -1/3!, +1/5!, -1/7!, +1/9!, -1/11!, +1/13!, -1/15!, +1/17!, -1/19!
 */
export const S: readonly number[] = [
  0,
  fromBits(0xbfc5555555555555n),
  fromBits(0x3f81111111111111n),
  fromBits(0xbf2a01a01a01a01an),
  fromBits(0x3ec71de3a556c734n),
  fromBits(0xbe5ae64567f544e4n),
  fromBits(0x3de6124613a86d09n),
  fromBits(0xbd6ae7f3e733b81fn),
  fromBits(0x3ce952c77030ad4an),
  fromBits(0xbc62f49b46814157n),
];

/**
 * Cosine coefficients C[n] = (-1)^n/(2n)!, n = 2..9 (indices 0 and 1 unused; the
 * n = 1 term, -1/2, is the exact product 0.5 * z):
 * +1/4!, -1/6!, +1/8!, -1/10!, +1/12!, -1/14!, +1/16!, -1/18!
 */
export const C: readonly number[] = [
  0,
  0,
  fromBits(0x3fa5555555555555n),
  fromBits(0xbf56c16c16c16c17n),
  fromBits(0x3efa01a01a01a01an),
  fromBits(0xbe927e4fb7789f5cn),
  fromBits(0x3e21eed8eff8d898n),
  fromBits(0xbda93974a8c07c9dn),
  fromBits(0x3d2ae7f3e733b81fn),
  fromBits(0xbca6827863b97d97n),
];

// ----------------------------------------------------------------------------
// Range reduction and kernels
// ----------------------------------------------------------------------------

/**
 * `[k, r]` with x = k*(pi/2) + r and |r| <= pi/4 (about).  Precondition:
 * |x| <= 2^29, so k is a finite whole number below 2^29.  Every product and the
 * subtraction that uses it are separate statements (never fused).
 */
function reduce(x: number): [number, number] {
  const t = x * INV_PIO2;
  const k = (t + MAGIC) - MAGIC;
  let r = x;
  for (const piece of PIO2) {
    const m = k * piece;
    r = r - m;
  }
  return [k, r];
}

/** sin(r) for |r| <= pi/4, Horner on the stored coefficients. */
function sinK(r: number): number {
  const z = r * r;
  let p = S[9] as number;
  for (let n = 8; n >= 1; n--) {
    p = p * z;
    p = p + (S[n] as number);
  }
  let w = r * z;
  w = w * p;
  return r + w;
}

/** cos(r) for |r| <= pi/4: (1 - z/2) + z*z*(C2 + ... + C9 z^7). */
function cosK(r: number): number {
  const z = r * r;
  let q = C[9] as number;
  for (let n = 8; n >= 2; n--) {
    q = q * z;
    q = q + (C[n] as number);
  }
  let u = 0.5 * z; // exact
  u = 1.0 - u;
  let v = z * z;
  v = v * q;
  return u + v;
}

// ----------------------------------------------------------------------------
// Sine and cosine
// ----------------------------------------------------------------------------

/**
 * Sine of x radians for |x| <= 2^29; NaN for NaN, the infinities and anything
 * larger.  sin(-0) is -0; |x| < 2^-27 returns x.
 *
 * The guard is a *negated* <=: a NaN fails every comparison, so one test covers
 * NaN, both infinities and every finite value beyond the domain.
 */
export function sin(x: number): number {
  if (!(Math.abs(x) <= LIMIT)) return NaN;
  if (Math.abs(x) < TINY) return x;
  const [k, r] = reduce(x);
  // k is a whole number below 2^29: `& 3` (ToInt32) is exact and is k mod 4
  // (two's complement, so -1 & 3 === 3).
  switch (k & 3) {
    case 0:
      return sinK(r);
    case 1:
      return cosK(r);
    case 2:
      return -sinK(r);
    default:
      return -cosK(r);
  }
}

/** Cosine of x radians for |x| <= 2^29; NaN outside the domain. cos(+-0) = 1. */
export function cos(x: number): number {
  if (!(Math.abs(x) <= LIMIT)) return NaN;
  if (Math.abs(x) < TINY) return 1.0;
  const [k, r] = reduce(x);
  switch (k & 3) {
    case 0:
      return cosK(r);
    case 1:
      return -sinK(r);
    case 2:
      return -cosK(r);
    default:
      return sinK(r);
  }
}

/** Degrees to radians: one multiply by a pre-divided, bit-pinned constant. */
export function radians(deg: number): number {
  return deg * RAD_PER_DEG;
}

/** Radians to degrees: one multiply by a pre-divided, bit-pinned constant. */
export function degrees(rad: number): number {
  return rad * DEG_PER_RAD;
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
  if (x === 0.0) return x;
  if (x === Infinity) return x;

  let scaled = x;
  let resultScale = 1.0;
  while (scaled < 0.25) {
    scaled *= 4.0;
    resultScale *= 0.5;
  }
  while (scaled >= 4.0) {
    scaled *= 0.25;
    resultScale *= 2.0;
  }

  // Initial guess: use x itself for x >= 1 (decent for large numbers),
  // and 1.0 for x < 1 (avoids dividing by a very small number early on).
  // Both converge correctly; this just saves a few iterations.
  let guess = scaled >= 1.0 ? scaled : 1.0;

  // Iterate up to 60 times. In practice the loop exits in ~15 iterations
  // due to quadratic convergence. The cap prevents infinite loops on
  // edge cases near floating-point subnormals.
  for (let i = 0; i < 60; i++) {
    const next = (guess + scaled / guess) / 2.0;

    // Convergence check: stop when the improvement is smaller than the
    // best precision we can hope for at this magnitude.
    // The 1e-15 * guess term accounts for relative precision near large values.
    // The 1e-300 absolute floor handles subnormals safely.
    if (Math.abs(next - guess) < 1e-15 * guess + 1e-300) {
      return next * resultScale;
    }

    guess = next;
  }

  return guess * resultScale;
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
  // atan(x) rounds exactly to x here; avoid halving subnormals and retain -0.
  if (Math.abs(x) <= 7.450580596923828e-9) return x;

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
  // A NaN fails every comparison below and would fall through to 0.0; a finite
  // answer to an undefined question is wrong (PHY00a 3.9).
  if (Number.isNaN(x) || Number.isNaN(y)) return NaN;
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
