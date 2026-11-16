/**
 * Fast percussion amplitude shape models: attack, hold, then decay (MUS07 §4.3).
 */

import { validateDuration, validateNonNegative } from "./validation.js";

export class PercussionEnvelope {
  readonly attackSeconds: number;
  readonly holdSeconds: number;
  readonly decaySeconds: number;

  constructor(
    attackSeconds: number,
    holdSeconds: number,
    decaySeconds: number,
  ) {
    this.attackSeconds = validateDuration("attackSeconds", attackSeconds);
    this.holdSeconds = validateDuration("holdSeconds", holdSeconds);
    const dec = validateDuration("decaySeconds", decaySeconds);
    if (dec <= 0.0) {
      throw new RangeError(`decaySeconds must be > 0.0, got ${String(dec)}`);
    }
    this.decaySeconds = dec;
  }

  renderedDurationSeconds(hitDurationSeconds: number): number {
    const hitDur = validateDuration("hitDurationSeconds", hitDurationSeconds);
    return hitDur + this.decaySeconds;
  }

  valueAt(timeSeconds: number, hitDurationSeconds: number): number {
    const time = validateNonNegative("timeSeconds", timeSeconds);
    const hitDur = validateNonNegative("hitDurationSeconds", hitDurationSeconds);

    if (time < this.attackSeconds) {
      if (this.attackSeconds <= 0.0) {
        return 1.0;
      }
      return time / this.attackSeconds;
    }

    const sustainEnd = this.attackSeconds + Math.max(this.holdSeconds, hitDur);
    if (time < sustainEnd) {
      return 1.0;
    }

    const decayTime = time - sustainEnd;
    if (decayTime >= this.decaySeconds) {
      return 0.0;
    }

    const frac = decayTime / this.decaySeconds;
    return Math.max(0.0, Math.min(1.0, 1.0 - frac));
  }
}
