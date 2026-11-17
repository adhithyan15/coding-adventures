/**
 * Deterministic seeded micro-variation profile (MUS05 §7).
 */

import type { VariationProfileOptions } from "./types.js";
import { validateFinite, validateNonNegative } from "./validation.js";

export class VariationProfile {
  readonly pitchCentsJitter: number;
  readonly amplitudeJitter: number;
  readonly attackJitterMs: number;
  readonly seed: bigint | number | null;
  readonly harmonicJitter: number;

  constructor(options: VariationProfileOptions = {}) {
    let pJitter = options.pitchCentsJitter ?? 0.0;
    if (options.pitchJitterCents !== undefined) {
      pJitter = options.pitchJitterCents;
    }

    let atkJitter = options.attackJitterMs ?? 0.0;
    if (options.timingJitterSeconds !== undefined) {
      atkJitter =
        validateNonNegative("timingJitterSeconds", options.timingJitterSeconds) * 1000.0;
    }

    this.pitchCentsJitter = validateNonNegative("pitchCentsJitter", pJitter);
    this.amplitudeJitter = validateNonNegative(
      "amplitudeJitter",
      options.amplitudeJitter ?? 0.0,
    );
    this.attackJitterMs = validateNonNegative("attackJitterMs", atkJitter);
    this.harmonicJitter = validateNonNegative(
      "harmonicJitter",
      options.harmonicJitter ?? 0.0,
    );

    if (options.seed !== undefined && options.seed !== null) {
      if (typeof options.seed === "number") {
        validateFinite("seed", options.seed);
        this.seed = Math.trunc(options.seed);
      } else if (typeof options.seed === "bigint") {
        this.seed = options.seed;
      } else {
        throw new TypeError("seed must be a number, bigint, or null");
      }
    } else {
      this.seed = null;
    }
  }

  get pitchJitterCents(): number {
    return this.pitchCentsJitter;
  }

  get timingJitterSeconds(): number {
    return this.attackJitterMs / 1000.0;
  }
}
