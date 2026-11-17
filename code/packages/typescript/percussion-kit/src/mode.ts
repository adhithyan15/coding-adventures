/**
 * Decaying resonant mode model for percussion voices (MUS07 §4.2).
 */

import {
  validateDuration,
  validateFinite,
  validateNonNegative,
  validatePositive,
} from "./validation.js";

export class PercussionMode {
  readonly frequencyHz: number;
  readonly amplitude: number;
  readonly decaySeconds: number;
  readonly phaseCycles: number;

  constructor(
    frequencyHz: number,
    amplitude: number,
    decaySeconds: number,
    phaseCycles = 0.0,
  ) {
    this.frequencyHz = validatePositive("frequencyHz", frequencyHz);
    this.amplitude = validateNonNegative("amplitude", amplitude);
    const dec = validateDuration("decaySeconds", decaySeconds);
    if (dec <= 0.0) {
      throw new RangeError(`decaySeconds must be > 0.0, got ${String(dec)}`);
    }
    this.decaySeconds = dec;
    this.phaseCycles = validateFinite("phaseCycles", phaseCycles);
  }
}
