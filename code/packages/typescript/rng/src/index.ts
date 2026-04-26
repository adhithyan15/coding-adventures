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

export const VERSION = "0.1.0";

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
   * Return a uniform random integer in `[min, max]` inclusive.
   *
   * Uses rejection sampling to eliminate modulo bias. The threshold is:
   * `(-rangeSize) mod rangeSize` — any draw below it is discarded.
   */
  nextIntInRange(min: number, max: number): number {
    if (min > max) {
      throw new RangeError(`nextIntInRange requires min <= max, got ${min} > ${max}`);
    }
    const rangeSize = BigInt(max - min + 1);
    const threshold = (-rangeSize & MASK32) % rangeSize;
    for (;;) {
      const r = BigInt(this.nextU32());
      if (r >= threshold) {
        return min + Number(r % rangeSize);
      }
    }
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
   * Return a uniform random integer in `[min, max]` inclusive.
   * Identical rejection-sampling algorithm to {@link LCG.nextIntInRange}.
   */
  nextIntInRange(min: number, max: number): number {
    if (min > max) {
      throw new RangeError(`nextIntInRange requires min <= max, got ${min} > ${max}`);
    }
    const rangeSize = BigInt(max - min + 1);
    const threshold = (-rangeSize & MASK32) % rangeSize;
    for (;;) {
      const r = BigInt(this.nextU32());
      if (r >= threshold) {
        return min + Number(r % rangeSize);
      }
    }
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
   * Return a uniform random integer in `[min, max]` inclusive.
   * Identical rejection-sampling algorithm to {@link LCG.nextIntInRange}.
   */
  nextIntInRange(min: number, max: number): number {
    if (min > max) {
      throw new RangeError(`nextIntInRange requires min <= max, got ${min} > ${max}`);
    }
    const rangeSize = BigInt(max - min + 1);
    const threshold = (-rangeSize & MASK32) % rangeSize;
    for (;;) {
      const r = BigInt(this.nextU32());
      if (r >= threshold) {
        return min + Number(r % rangeSize);
      }
    }
  }
}
