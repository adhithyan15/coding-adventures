/**
 * Deterministic pseudo-random noise burst models (MUS07 §4.1).
 */

import { sin } from "trig/dist/trig.js";
import { splitmix64Noise } from "./splitmix64.js";
import type { NoiseAlgorithm } from "./types.js";
import { validateDuration, validateNonNegative } from "./validation.js";

/**
 * Deterministic hash-based pseudo-random noise sample in `[-1.0, 1.0]`.
 * Uses golden ratio phase offset and sine fraction folding (Python/Rust MUS07 parity).
 */
export function pseudoNoise(sampleIndex: number, seed: number | bigint): number {
  const seedNum = typeof seed === "bigint" ? Number(seed & 0xffff_ffffn) : Number(seed);
  const phase =
    (sampleIndex + 1.0) * (seedNum * 0.1618033988749895 + 0.61803398875);
  const val = sin(2.0 * Math.PI * phase) * 43758.5453123;
  const frac = val - Math.floor(val);
  return 2.0 * frac - 1.0;
}

export class InvalidParamError extends RangeError {
  constructor(message: string) {
    super(message);
    this.name = "InvalidParamError";
  }
}

export class PercussionNoiseProfile {
  readonly amplitude: number;
  readonly decaySeconds: number;
  readonly seed: number | bigint;
  readonly algorithm: NoiseAlgorithm;

  constructor(
    amplitude: number,
    decaySeconds: number,
    seed: number | bigint = 0,
    algorithm: NoiseAlgorithm = "pseudo_noise",
  ) {
    this.amplitude = validateNonNegative("amplitude", amplitude);
    const dec = validateDuration("decaySeconds", decaySeconds);
    if (dec <= 0.0) {
      throw new RangeError(`decaySeconds must be > 0.0, got ${String(dec)}`);
    }
    this.decaySeconds = dec;

    if (typeof seed === "number") {
      if (!Number.isFinite(seed)) {
        throw new InvalidParamError(`seed must be a finite number, got ${String(seed)}`);
      }
      this.seed = seed;
    } else if (typeof seed === "bigint") {
      this.seed = seed;
    } else {
      throw new TypeError(`seed must be a number or bigint, got ${typeof seed}`);
    }

    this.algorithm = algorithm === "splitmix64" ? "splitmix64" : "pseudo_noise";
  }

  sampleNoise(sampleIndex: number): number {
    if (this.algorithm === "splitmix64") {
      return splitmix64Noise(sampleIndex, this.seed);
    }
    return pseudoNoise(sampleIndex, this.seed);
  }
}
