/**
 * Deterministic pseudo-randomness and shuffling (TF01 §5.1, §5.2).
 */

const GOLDEN_GAMMA = 0x9e37_79b9_7f4a_7c15n;
const SCRAMBLE_1 = 0xbf58_476d_1ce4_e5b9n;
const SCRAMBLE_2 = 0x94d0_49bb_1331_11ebn;

const FNV_OFFSET_BASIS = 0xcbf2_9ce4_8422_2325n;
const FNV_PRIME = 0x0000_0100_0000_01b3n;

export class SplitMix64 {
  private state: bigint;

  constructor(seed: bigint | number) {
    this.state = BigInt.asUintN(64, BigInt(seed));
  }

  /**
   * Generates the next 64-bit pseudo-random unsigned integer.
   */
  nextU64(): bigint {
    this.state = BigInt.asUintN(64, this.state + GOLDEN_GAMMA);
    let z = this.state;
    z = BigInt.asUintN(64, (z ^ (z >> 30n)) * SCRAMBLE_1);
    z = BigInt.asUintN(64, (z ^ (z >> 27n)) * SCRAMBLE_2);
    return BigInt.asUintN(64, z ^ (z >> 31n));
  }

  /**
   * Generates an unbiased integer in `[0, n)` for `1 <= n <= 2^32`.
   */
  bounded(n: number | bigint): number {
    const bn = BigInt(n);
    if (bn < 1n || bn > 0x1_0000_0000n) {
      throw new Error(`bounded(n) needs 1 <= n <= 2^32, got ${n}`);
    }
    const threshold = BigInt.asUintN(64, -bn) % bn;
    while (true) {
      const x = this.nextU64();
      if (x >= threshold) {
        return Number(x % bn);
      }
    }
  }
}

/**
 * Computes 64-bit FNV-1a hash over raw byte array.
 */
export function fnv1a64(data: Uint8Array): bigint {
  let hash = FNV_OFFSET_BASIS;
  for (let i = 0; i < data.length; i++) {
    hash ^= BigInt(data[i]);
    hash = BigInt.asUintN(64, hash * FNV_PRIME);
  }
  return hash;
}

/**
 * Derives a deterministic per-test sub-seed from the run seed and test ID (TF01 §5.2 rule 4).
 */
export function subSeed(runSeed: bigint, testId: string): bigint {
  const encoder = new TextEncoder();
  const idHash = fnv1a64(encoder.encode(testId));
  const rng = new SplitMix64(runSeed ^ idHash);
  return rng.nextU64();
}

/**
 * Computes the deterministic execution order for a set of test IDs (TF01 §5.2).
 * 1. Sort IDs by exact byte order (UTF-8).
 * 2. Shuffle with Fisher-Yates driven by SplitMix64(seed).
 */
export function shuffleOrder(seed: bigint, ids: string[]): string[] {
  const encoder = new TextEncoder();
  const order = [...ids].sort((a, b) => {
    const bytesA = encoder.encode(a);
    const bytesB = encoder.encode(b);
    const minLen = Math.min(bytesA.length, bytesB.length);
    for (let i = 0; i < minLen; i++) {
      if (bytesA[i] !== bytesB[i]) {
        return bytesA[i] - bytesB[i];
      }
    }
    return bytesA.length - bytesB.length;
  });

  if (order.length > 1) {
    const rng = new SplitMix64(seed);
    for (let i = order.length - 1; i >= 1; i--) {
      const j = rng.bounded(i + 1);
      const tmp = order[i];
      order[i] = order[j];
      order[j] = tmp;
    }
  }

  return order;
}
