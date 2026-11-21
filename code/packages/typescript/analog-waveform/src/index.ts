/**
 * Continuous-time waveform primitives for the electronics track (PHY03, VER15).
 *
 * Hardened against:
 * - CWE-20 (Improper Input Validation): Validates runtime types, finiteness,
 *   non-negative amplitude, positive frequency, and angular-frequency finiteness.
 * - CWE-190 / CWE-400: Period/phase reduction avoids intermediate overflow on
 *   large frequencies or times, keeping evaluation O(1) and finite.
 * - CWE-1321: Instances are frozen via Object.freeze and state is re-checked on
 *   method entry.
 */

export interface AnalogWaveform {
  sampleAt(timeSeconds: number): number;
}

const TWO_PI = 2 * Math.PI;

function requireFiniteNumber(label: string, value: unknown): number {
  if (typeof value !== "number") {
    throw new TypeError(`${label} must be a finite number`);
  }
  if (!Number.isFinite(value)) {
    throw new RangeError(`${label} must be finite`);
  }
  return value;
}

export class ConstantWaveform implements AnalogWaveform {
  readonly level: number;

  constructor(level: number) {
    const val = requireFiniteNumber("Level", level);
    this.level = val === 0 ? 0 : val;
    Object.freeze(this);
  }

  sampleAt(timeSeconds: number): number {
    requireFiniteNumber("Time", timeSeconds);
    const val = requireFiniteNumber("Level", this.level);
    return val === 0 ? 0 : val;
  }
}

export class SineWaveform implements AnalogWaveform {
  readonly amplitude: number;
  readonly frequencyHz: number;
  readonly phaseRadians: number;
  readonly offset: number;

  constructor(
    amplitude: number,
    frequencyHz: number,
    phaseRadians: number = 0,
    offset: number = 0
  ) {
    const amp = requireFiniteNumber("Amplitude", amplitude);
    if (amp < 0) {
      throw new RangeError(`Amplitude must be >= 0, got ${amp}.`);
    }

    const freq = requireFiniteNumber("Frequency", frequencyHz);
    if (freq <= 0) {
      throw new RangeError(`Frequency must be > 0, got ${freq}.`);
    }
    if (!Number.isFinite(TWO_PI * freq)) {
      throw new RangeError("Angular frequency must be finite");
    }

    const phase = requireFiniteNumber("Phase", phaseRadians);
    const off = requireFiniteNumber("Offset", offset);

    this.amplitude = amp === 0 ? 0 : amp;
    this.frequencyHz = freq;
    this.phaseRadians = phase === 0 ? 0 : phase;
    this.offset = off === 0 ? 0 : off;
    Object.freeze(this);
  }

  private validateState(): void {
    if (
      typeof this.amplitude !== "number" ||
      !Number.isFinite(this.amplitude) ||
      this.amplitude < 0 ||
      typeof this.frequencyHz !== "number" ||
      !Number.isFinite(this.frequencyHz) ||
      this.frequencyHz <= 0 ||
      !Number.isFinite(TWO_PI * this.frequencyHz) ||
      typeof this.phaseRadians !== "number" ||
      !Number.isFinite(this.phaseRadians) ||
      typeof this.offset !== "number" ||
      !Number.isFinite(this.offset)
    ) {
      throw new RangeError("SineWaveform state must be finite and valid");
    }
  }

  periodSeconds(): number {
    this.validateState();
    return 1 / this.frequencyHz;
  }

  angularFrequency(): number {
    this.validateState();
    return TWO_PI * this.frequencyHz;
  }

  sampleAt(timeSeconds: number): number {
    this.validateState();
    const time = requireFiniteNumber("Time", timeSeconds);
    if (this.amplitude === 0) {
      return this.offset === 0 ? 0 : this.offset;
    }

    const period = 1 / this.frequencyHz;
    const reducedTime = Number.isFinite(period) ? time % period : time;
    const reducedPhase = this.phaseRadians % TWO_PI;
    const angle = TWO_PI * (this.frequencyHz * reducedTime) + reducedPhase;
    const unit = Math.max(-1, Math.min(1, Math.sin(angle)));
    const result = this.offset + this.amplitude * unit;
    if (!Number.isFinite(result)) {
      throw new RangeError("Sample value overflowed finite range");
    }
    return result === 0 ? 0 : result;
  }
}
