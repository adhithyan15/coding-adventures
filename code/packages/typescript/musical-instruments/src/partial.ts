/**
 * Harmonic partial model for additive timbre profiles (MUS05 §5).
 */

import {
  validateFinite,
  validateNonNegative,
  validatePositive,
} from "./validation.js";

export class HarmonicPartial {
  readonly frequencyMultiplier: number;
  readonly amplitude: number;
  readonly phaseCycles: number;

  constructor(
    frequencyMultiplier: number,
    amplitude: number,
    phaseCycles = 0.0,
  ) {
    this.frequencyMultiplier = validatePositive(
      "frequencyMultiplier",
      frequencyMultiplier,
    );
    this.amplitude = validateNonNegative("amplitude", amplitude);
    this.phaseCycles = validateFinite("phaseCycles", phaseCycles);
  }
}
