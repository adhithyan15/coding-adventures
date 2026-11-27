/** Small deterministic input generator; it never reads a wall-clock seed. */

import { numberFromBits } from "./oracles.js";

export class XorShift32 {
  private state: number;

  constructor(seed: number) {
    this.state = seed >>> 0;
    if (this.state === 0) throw new RangeError("xorshift seed must be nonzero");
  }

  nextUint32(): number {
    let value = this.state;
    value ^= value << 13;
    value ^= value >>> 17;
    value ^= value << 5;
    this.state = value >>> 0;
    return this.state;
  }

  integerBelow(limit: number): number {
    if (!Number.isInteger(limit) || limit <= 0 || limit > 0x1_0000_0000) {
      throw new RangeError("limit must be a positive 32-bit range");
    }
    return Math.floor((this.nextUint32() / 0x1_0000_0000) * limit);
  }
}

export interface GeneratedFloatCampaign {
  readonly arrays: readonly (readonly number[])[];
  readonly rejectedNonFiniteBitPatterns: number;
}

/** Build an exact number of accepted finite arrays from reproducible bits. */
export function finiteBinary64Campaign(
  seed: number,
  acceptedArrayCount: number,
): GeneratedFloatCampaign {
  const rng = new XorShift32(seed);
  const arrays: number[][] = [];
  let rejectedNonFiniteBitPatterns = 0;

  const nextFiniteNumber = (): number => {
    while (true) {
      const bits = (BigInt(rng.nextUint32()) << 32n) | BigInt(rng.nextUint32());
      if (((bits >> 52n) & 0x7ffn) === 0x7ffn) {
        rejectedNonFiniteBitPatterns += 1;
        continue;
      }
      return numberFromBits(bits.toString(16).padStart(16, "0"));
    }
  };

  for (let index = 0; index < acceptedArrayCount; index += 1) {
    const length = 1 + rng.integerBelow(32);
    const values: number[] = [];
    for (let item = 0; item < length; item += 1) values.push(nextFiniteNumber());
    arrays.push(values);
  }
  return { arrays, rejectedNonFiniteBitPatterns };
}

export function seededIntegerArray(rng: XorShift32, maximumLength = 32): number[] {
  const length = 1 + rng.integerBelow(maximumLength);
  const values: number[] = [];
  for (let index = 0; index < length; index += 1) {
    values.push(rng.integerBelow(129) - 64);
  }
  return values;
}

export function permuted<T>(input: readonly T[], rng: XorShift32): T[] {
  const result = [...input];
  for (let index = result.length - 1; index > 0; index -= 1) {
    const other = rng.integerBelow(index + 1);
    [result[index], result[other]] = [result[other], result[index]];
  }
  return result;
}
