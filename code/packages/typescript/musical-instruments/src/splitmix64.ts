/**
 * Pure TypeScript deterministic SplitMix64 pseudo-random number generator.
 *
 * Generates reproducible 64-bit pseudo-random streams with zero ambient entropy
 * and zero third-party dependencies, discharging CWE-330 for deterministic
 * musical timbre variation (VER05 §2.1 / TF01 §5.1).
 */

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
   * Return a uniform pseudo-random float in `[-1.0, 1.0]`.
   */
  nextFloatSigned(): number {
    const val = Number(this.nextU64());
    return val / 9223372036854775807.5 - 1.0;
  }

  /**
   * Return a uniform pseudo-random float in `[0.0, 1.0]`.
   */
  nextFloatUnit(): number {
    const val = Number(this.nextU64());
    return val / 18446744073709551615.0;
  }

  /**
   * Return a uniform pseudo-random float in `[-1.0, 1.0]` (oscillator compatibility).
   */
  nextFloat(): number {
    return this.nextFloatSigned();
  }
}
