/**
 * Deterministic PRNGs for reproducible white noise generation (TF01 §5.1, VER05 §2.1).
 * Completely decoupled from ambient randomness (no Math.random, zero external entropy).
 */

import { safeFormatValue } from "./validation.js";

const GOLDEN_GAMMA = 0x9e37_79b9_7f4a_7c15n;
const SCRAMBLE_1 = 0xbf58_476d_1ce4_e5b9n;
const SCRAMBLE_2 = 0x94d0_49bb_1331_11ebn;

export class SplitMix64 {
  private state: bigint;

  constructor(seed: bigint | number = 0n) {
    if (typeof seed === "number") {
      if (!Number.isFinite(seed)) {
        throw new TypeError(`seed must be finite, got ${String(seed)}`);
      }
      this.state = BigInt.asUintN(64, BigInt(Math.trunc(seed)));
    } else if (typeof seed === "bigint") {
      this.state = BigInt.asUintN(64, seed);
    } else {
      throw new TypeError(`seed must be a number or bigint, got ${typeof seed}`);
    }
  }

  /**
   * Produce the next 64-bit unsigned pseudo-random integer.
   */
  nextU64(): bigint {
    this.state = BigInt.asUintN(64, this.state + GOLDEN_GAMMA);
    let z = this.state;
    z = BigInt.asUintN(64, (z ^ (z >> 30n)) * SCRAMBLE_1);
    z = BigInt.asUintN(64, (z ^ (z >> 27n)) * SCRAMBLE_2);
    return BigInt.asUintN(64, z ^ (z >> 31n));
  }

  /**
   * Produce a deterministic floating-point value in [-1.0, 1.0].
   */
  nextFloat(): number {
    const raw53 = Number(this.nextU64() & 0x1f_ffff_ffff_ffffn);
    const unit = raw53 / 9007199254740991.0; // [0.0, 1.0]
    return unit * 2.0 - 1.0;
  }
}

/**
 * Portable 32-bit Linear Congruential Generator (LCG - Numerical Recipes parameters).
 * a = 1664525, c = 1013904223, m = 2^32.
 */
export class LcgPrng {
  private state: number;

  constructor(seed: number = 0) {
    if (typeof seed !== "number" || !Number.isFinite(seed)) {
      throw new TypeError(`seed must be a finite number, got ${safeFormatValue(seed)}`);
    }
    this.state = Math.trunc(seed) >>> 0;
  }

  nextU32(): number {
    this.state = (Math.imul(this.state, 1664525) + 1013904223) >>> 0;
    return this.state;
  }

  /**
   * Produce a deterministic floating-point value in [-1.0, 1.0].
   */
  nextFloat(): number {
    const u = this.nextU32() / 4294967295.0; // [0.0, 1.0]
    return u * 2.0 - 1.0;
  }
}
