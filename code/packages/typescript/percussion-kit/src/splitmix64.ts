/**
 * Pure TypeScript deterministic SplitMix64 pseudo-random generator and stateless noise (MUS07).
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

  nextU64(): bigint {
    this.state = BigInt.asUintN(64, this.state + GOLDEN_GAMMA);
    let z = this.state;
    z = BigInt.asUintN(64, (z ^ (z >> 30n)) * SCRAMBLE_1);
    z = BigInt.asUintN(64, (z ^ (z >> 27n)) * SCRAMBLE_2);
    return BigInt.asUintN(64, z ^ (z >> 31n));
  }

  nextFloatSigned(): number {
    const val = Number(this.nextU64());
    return val / 9223372036854775807.5 - 1.0;
  }

  nextFloatUnit(): number {
    const val = Number(this.nextU64());
    return val / 18446744073709551615.0;
  }

  nextFloat(): number {
    return this.nextFloatSigned();
  }
}

/**
 * Statelessly evaluate a deterministic SplitMix64 noise sample for a given index and seed.
 */
export function splitmix64Noise(sampleIndex: number, seed: number | bigint): number {
  const seedBig = BigInt.asUintN(64, typeof seed === "bigint" ? seed : BigInt(Math.trunc(seed)));
  const idxBig = BigInt.asUintN(64, BigInt(Math.trunc(sampleIndex)));
  const state = BigInt.asUintN(64, seedBig + idxBig * GOLDEN_GAMMA);
  let z = BigInt.asUintN(64, state + GOLDEN_GAMMA);
  z = BigInt.asUintN(64, (z ^ (z >> 30n)) * SCRAMBLE_1);
  z = BigInt.asUintN(64, (z ^ (z >> 27n)) * SCRAMBLE_2);
  const u = BigInt.asUintN(64, z ^ (z >> 31n));
  return Number(u) / 9223372036854775807.5 - 1.0;
}
