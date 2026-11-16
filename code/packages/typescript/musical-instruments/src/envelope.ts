/**
 * Attack, decay, sustain, and release (ADSR) loudness envelopes (MUS05 §6).
 */

import {
  validateDuration,
  validateNonNegative,
  validateUnit,
} from "./validation.js";

export class AdsrEnvelope {
  readonly attackSeconds: number;
  readonly decaySeconds: number;
  readonly sustainLevel: number;
  readonly releaseSeconds: number;

  constructor(
    attackSeconds: number,
    decaySeconds: number,
    sustainLevel: number,
    releaseSeconds: number,
  ) {
    this.attackSeconds = validateDuration("attackSeconds", attackSeconds);
    this.decaySeconds = validateDuration("decaySeconds", decaySeconds);
    this.sustainLevel = validateUnit("sustainLevel", sustainLevel);
    this.releaseSeconds = validateDuration("releaseSeconds", releaseSeconds);
  }

  renderedDurationSeconds(noteDurationSeconds: number): number {
    const dur = validateDuration("noteDurationSeconds", noteDurationSeconds);
    return dur + this.releaseSeconds;
  }

  valueAt(timeSeconds: number, noteDurationSeconds: number): number {
    const time = validateNonNegative("timeSeconds", timeSeconds);
    const noteDuration = validateDuration("noteDurationSeconds", noteDurationSeconds);

    if (this.attackSeconds > 0.0 && time < this.attackSeconds) {
      return time / this.attackSeconds;
    }

    const decayEnd = this.attackSeconds + this.decaySeconds;
    if (time < decayEnd) {
      const progress =
        this.decaySeconds > 0.0
          ? (time - this.attackSeconds) / this.decaySeconds
          : 1.0;
      return 1.0 + (this.sustainLevel - 1.0) * progress;
    }

    if (time < noteDuration) {
      return this.sustainLevel;
    }

    const releaseTime = time - noteDuration;
    if (this.releaseSeconds === 0.0 || releaseTime >= this.releaseSeconds) {
      return 0.0;
    }

    return this.sustainLevel * (1.0 - releaseTime / this.releaseSeconds);
  }
}

export { AdsrEnvelope as ADSREnvelope };
