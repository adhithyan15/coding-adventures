/**
 * @coding-adventures/rng
 *
 * LCG, Xorshift64, and PCG32 pseudorandom number generators.
 *
 * # Why BigInt?
 *
 * JavaScript numbers are IEEE 754 doubles with only 53 bits of integer
 * precision. That is not enough to hold a 64-bit state without losing
 * low bits. We use `BigInt` for ALL internal state arithmetic so that every
 * bit is exact. Outputs are converted back to `number` when they fit in 32
 * bits (which they always do for `nextU32`, `nextFloat`, and
 * `nextIntInRange`). `nextU64` returns `bigint` directly.
 *
 * # Reference values for seed = 1n
 *
 * | Call | LCG        | Xorshift64 | PCG32      |
 * |------|------------|------------|------------|
 * | 1st  | 1817669548 | 1082269761 | 1412771199 |
 * | 2nd  | 2187888307 | 201397313  | 1791099446 |
 * | 3rd  | 2784682393 | 1854285353 | 124312908  |
 *
 * These values match the Go reference implementation exactly.
 *
 * This package is part of the coding-adventures monorepo, a ground-up
 * implementation of the computing stack from transistors to operating systems.
 */

export const VERSION = "0.2.0";

// ── Constants ─────────────────────────────────────────────────────────────────
//
// Knuth/Numerical Recipes constants — satisfy Hull-Dobell for full period 2^64.

const LCG_MULTIPLIER: bigint = 6364136223846793005n;
const LCG_INCREMENT: bigint = 1442695040888963407n;

/** Mask to keep BigInt arithmetic within 64-bit unsigned range. */
const MASK64: bigint = 0xffffffffffffffffn;

/** Mask for 32-bit unsigned output. */
const MASK32: bigint = 0xffffffffn;

/** Denominator for float normalisation: 2^32 = 4_294_967_296. */
const FLOAT_DIV: number = 4_294_967_296;

// ── LCG ───────────────────────────────────────────────────────────────────────

/**
 * Linear Congruential Generator (Knuth 1948).
 *
 * Recurrence: `state = (state × a + c) mod 2^64`
 *
 * - Period: 2^64 — every 64-bit value appears exactly once per cycle.
 * - Output: upper 32 bits (`>>> 32` on BigInt state). Lower bits have
 *   shorter sub-periods and are discarded.
 * - Weakness: consecutive outputs are linearly correlated.
 *
 * ```ts
 * const g = new LCG(1n);
 * g.nextU32(); // 1817669548
 * g.nextU32(); // 2187888307
 * ```
 */
export class LCG {
  private state: bigint;

  /** Seed the generator. Any 64-bit value is valid. Pass seed as `bigint`. */
  constructor(seed: bigint) {
    this.state = seed & MASK64;
  }

  /** Advance state; return upper 32 bits as a `number` in `[0, 2^32)`. */
  nextU32(): number {
    this.state = (this.state * LCG_MULTIPLIER + LCG_INCREMENT) & MASK64;
    return Number(this.state >> 32n);
  }

  /**
   * Return a 64-bit value as a `bigint`: `(hi << 32n) | lo` from two
   * consecutive `nextU32` calls.
   */
  nextU64(): bigint {
    const hi = BigInt(this.nextU32());
    const lo = BigInt(this.nextU32());
    return (hi << 32n) | lo;
  }

  /** Return a `number` uniformly distributed in `[0.0, 1.0)`. */
  nextFloat(): number {
    return this.nextU32() / FLOAT_DIV;
  }

  /**
   * A uniform integer in `[min, max]` inclusive, for bounds that are safe
   * JavaScript integers (|n| <= 2^53 − 1). Throws `RangeError` for unsafe or
   * reversed bounds, before drawing anything. See {@link bigIntInRange}.
   */
  nextIntInRange(min: number, max: number): number {
    return intInRange(() => this.nextU32(), min, max);
  }

  /**
   * A uniform integer in `[min, max]` inclusive over the whole signed 64-bit
   * domain, as `bigint`. Throws `RangeError` for out-of-domain or reversed
   * bounds, before drawing anything. See {@link bigIntInRange}.
   */
  nextBigIntInRange(min: bigint, max: bigint): bigint {
    return bigIntInRange(() => this.nextU32(), min, max);
  }
}

// ── Xorshift64 ────────────────────────────────────────────────────────────────

/**
 * Xorshift64 generator (Marsaglia 2003).
 *
 * Three XOR-shift operations scramble 64-bit state with no multiplication:
 *
 * ```
 * x ^= x << 13n
 * x ^= x >> 7n
 * x ^= x << 17n
 * ```
 *
 * - Period: 2^64 − 1. Seed 0 is a fixed point and is replaced with 1.
 * - Output: lower 32 bits.
 *
 * ```ts
 * const g = new Xorshift64(1n);
 * g.nextU32(); // 1082269761
 * ```
 */
export class Xorshift64 {
  private state: bigint;

  /** Seed the generator. Seed `0n` is replaced with `1n`. */
  constructor(seed: bigint) {
    const s = seed & MASK64;
    this.state = s === 0n ? 1n : s;
  }

  /** Apply three XOR-shifts; return lower 32 bits as a `number`. */
  nextU32(): number {
    let x = this.state;
    x ^= (x << 13n) & MASK64;
    x ^= x >> 7n;
    x ^= (x << 17n) & MASK64;
    this.state = x & MASK64;
    return Number(x & MASK32);
  }

  /** Return a 64-bit `bigint`: `(hi << 32n) | lo`. */
  nextU64(): bigint {
    const hi = BigInt(this.nextU32());
    const lo = BigInt(this.nextU32());
    return (hi << 32n) | lo;
  }

  /** Return a `number` uniformly distributed in `[0.0, 1.0)`. */
  nextFloat(): number {
    return this.nextU32() / FLOAT_DIV;
  }

  /**
   * A uniform integer in `[min, max]` inclusive, for bounds that are safe
   * JavaScript integers (|n| <= 2^53 − 1). Throws `RangeError` for unsafe or
   * reversed bounds, before drawing anything. See {@link bigIntInRange}.
   */
  nextIntInRange(min: number, max: number): number {
    return intInRange(() => this.nextU32(), min, max);
  }

  /**
   * A uniform integer in `[min, max]` inclusive over the whole signed 64-bit
   * domain, as `bigint`. Throws `RangeError` for out-of-domain or reversed
   * bounds, before drawing anything. See {@link bigIntInRange}.
   */
  nextBigIntInRange(min: bigint, max: bigint): bigint {
    return bigIntInRange(() => this.nextU32(), min, max);
  }
}

// ── PCG32 ─────────────────────────────────────────────────────────────────────

/**
 * Permuted Congruential Generator (O'Neill 2014).
 *
 * Uses the same LCG recurrence as {@link LCG} but applies the XSH RR
 * (XOR-Shift High / Random Rotate) output permutation:
 *
 * ```
 * xorshifted = ((old >> 18n) ^ old) >> 27n    // mix high bits down
 * rot        = old >> 59n                      // 5-bit rotation amount
 * output     = rotr32(xorshifted, rot)         // scatter all bits
 * ```
 *
 * Passes all known statistical test suites (TestU01 BigCrush, PractRand).
 *
 * initseq warm-up: state=0 → advance → add seed → advance.
 *
 * ```ts
 * const g = new PCG32(1n);
 * g.nextU32(); // 1412771199
 * ```
 */
export class PCG32 {
  private state: bigint;
  private readonly increment: bigint;

  /** Seed the generator (pass seed as `bigint`). */
  constructor(seed: bigint) {
    const inc = LCG_INCREMENT | 1n; // must be odd (already is)
    this.increment = inc;
    // Step 1: advance once from state=0
    let state = (0n * LCG_MULTIPLIER + inc) & MASK64;
    // Step 2: mix seed in
    state = (state + (seed & MASK64)) & MASK64;
    // Step 3: advance once more to scatter seed bits
    state = (state * LCG_MULTIPLIER + inc) & MASK64;
    this.state = state;
  }

  /** Advance LCG; return XSH RR permuted output of old state as a `number`. */
  nextU32(): number {
    const old = this.state;
    this.state = (old * LCG_MULTIPLIER + this.increment) & MASK64;

    // XSH RR permutation ─────────────────────────────────────────────────────
    // Step 1: XOR-shift — mix high bits down to lower 32.
    const xorshifted = (((old >> 18n) ^ old) >> 27n) & MASK32;

    // Step 2: 5-bit rotation amount from the top 5 bits.
    const rot = Number(old >> 59n);

    // Step 3: rotate-right 32 bits.
    //   rotr32(x, n) = (x >> n) | (x << (32-n))  mod 2^32
    const rotB = BigInt(rot);
    const leftRot = BigInt((-rot) & 31);
    const rotated = ((xorshifted >> rotB) | ((xorshifted << leftRot) & MASK32)) & MASK32;

    return Number(rotated);
  }

  /** Return a 64-bit `bigint`: `(hi << 32n) | lo`. */
  nextU64(): bigint {
    const hi = BigInt(this.nextU32());
    const lo = BigInt(this.nextU32());
    return (hi << 32n) | lo;
  }

  /** Return a `number` uniformly distributed in `[0.0, 1.0)`. */
  nextFloat(): number {
    return this.nextU32() / FLOAT_DIV;
  }

  /**
   * A uniform integer in `[min, max]` inclusive, for bounds that are safe
   * JavaScript integers (|n| <= 2^53 − 1). Throws `RangeError` for unsafe or
   * reversed bounds, before drawing anything. See {@link bigIntInRange}.
   */
  nextIntInRange(min: number, max: number): number {
    return intInRange(() => this.nextU32(), min, max);
  }

  /**
   * A uniform integer in `[min, max]` inclusive over the whole signed 64-bit
   * domain, as `bigint`. Throws `RangeError` for out-of-domain or reversed
   * bounds, before drawing anything. See {@link bigIntInRange}.
   */
  nextBigIntInRange(min: bigint, max: bigint): bigint {
    return bigIntInRange(() => this.nextU32(), min, max);
  }
}

// ── Bounded draws (spec rng.md, "Rejection Sampling", v2) ────────────────────

const I64_MIN = -(1n << 63n);
const I64_MAX = (1n << 63n) - 1n;
const TWO_32 = 1n << 32n;
const TWO_64 = 1n << 64n;

/**
 * A uniform integer in `[min, max]`, drawn from a stream of 32-bit outputs,
 * identically in every language of the repository.
 *
 * `span = max − min + 1` lies between 1 and 2^64. Taking `r % span` of a raw
 * draw over-samples small offsets whenever the number of possible draws is
 * not a multiple of `span`; rejecting the `threshold` lowest draws removes
 * exactly the excess. Three regimes, chosen by `span` alone:
 *
 * ```text
 *   span <= 2^32    draw 32 bits,  threshold = (2^32 − span) mod span
 *   span <  2^64    draw 64 bits,  threshold = (2^64 − span) mod span
 *   span == 2^64    the whole int64 domain: every u64 is a valid offset
 * ```
 *
 * Example, span = 3: 2^32 = 3 × 1431655765 + 1, so threshold = 1. Rejecting
 * r = 0 leaves 2^32 − 1 draws, an exact multiple of 3.
 *
 * v1 drew 32 bits for every span, so a wide range could never return more
 * than 2^32 − 1 above `min`; its `number` bounds also lost precision above
 * 2^53, silently drawing from a different range than the caller asked for.
 */
export function bigIntInRange(next32: () => number, min: bigint, max: bigint): bigint {
  if (min < I64_MIN || max > I64_MAX || min > max) {
    throw new RangeError(
      `nextIntInRange needs signed 64-bit bounds with min <= max, got [${min}, ${max}]`,
    );
  }
  const span = max - min + 1n;
  const next64 = (): bigint => (BigInt(next32()) << 32n) | BigInt(next32());
  if (span <= TWO_32) {
    const threshold = (TWO_32 - span) % span;
    for (;;) {
      const r = BigInt(next32());
      if (r >= threshold) return min + (r % span);
    }
  }
  if (span < TWO_64) {
    const threshold = (TWO_64 - span) % span;
    for (;;) {
      const r = next64();
      if (r >= threshold) return min + (r % span);
    }
  }
  return min + next64();
}

/** {@link bigIntInRange} for bounds that are safe JavaScript integers. */
function intInRange(next32: () => number, min: number, max: number): number {
  if (!Number.isSafeInteger(min) || !Number.isSafeInteger(max)) {
    throw new RangeError(
      `nextIntInRange needs safe integer bounds (use nextBigIntInRange), got [${min}, ${max}]`,
    );
  }
  return Number(bigIntInRange(next32, BigInt(min), BigInt(max)));
}
