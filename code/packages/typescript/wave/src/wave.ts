// ============================================================================
// wave.ts — Simple Harmonic Wave Model
// ============================================================================
//
// This module models a simple harmonic wave, the fundamental building block
// of electromagnetic waves, sound waves, and virtually all oscillatory
// phenomena in physics.
//
// What is a wave?
// ----------------
// A wave is a disturbance that propagates through space and time, carrying
// energy without transporting matter. The simplest mathematical model of
// a wave is the sinusoidal (harmonic) wave:
//
//   y(t) = A · sin(2πft + φ)
//
// where:
//   A = amplitude    — the peak displacement from equilibrium
//   f = frequency    — how many complete cycles occur per second (in Hz)
//   t = time         — the moment at which we evaluate the wave
//   φ = phase        — shifts the wave left or right along the time axis
//
// Why sinusoidal?
// ---------------
// Joseph Fourier (1822) proved that ANY periodic wave — square waves,
// sawtooth waves, even the complex waveforms of musical instruments —
// can be decomposed into a sum of sinusoidal waves. This means that
// understanding the simple harmonic wave is the key to understanding
// ALL waves. This principle is called Fourier analysis.
//
// Key relationships:
// ------------------
//   period T = 1/f        — time for one complete cycle (seconds)
//   angular frequency ω = 2πf  — frequency in radians per second
//   wavelength λ = v/f    — spatial extent of one cycle (v = wave speed)
//
// In this module we focus on the temporal behavior y(t), leaving spatial
// propagation for a future wave-propagation package.
//
// VER15 Hardening:
//   - CWE-20 (Improper Input Validation): Validates runtime types, finiteness,
//     non-negative amplitude, positive frequency, and angular-frequency finiteness.
//   - CWE-190 / CWE-400: Period/phase reduction prior to trig evaluation and
//     amplitude-bounded clamping keep every finite input O(1) and finite.
//   - CWE-1321: Instances are frozen via Object.freeze to prevent runtime
//     property tampering or prototype pollution side effects.
// ============================================================================

import { PI, sin } from "trig/dist/trig.js";

function requireFiniteNumber(label: string, value: unknown): number {
  if (typeof value !== "number") {
    throw new TypeError(`${label} must be a finite number`);
  }
  if (!Number.isFinite(value)) {
    throw new RangeError(`${label} must be finite`);
  }
  return value;
}

// ============================================================================
// Wave Class
// ============================================================================

export class Wave {
  /** Peak displacement from zero. Always non-negative. */
  readonly amplitude: number;

  /** Number of complete oscillation cycles per second (Hz). Always positive. */
  readonly frequency: number;

  /** Initial phase offset in radians. Shifts the wave along the time axis. */
  readonly phase: number;

  constructor(amplitude: number, frequency: number, phase: number = 0.0) {
    const amp = requireFiniteNumber("Amplitude", amplitude);
    if (amp < 0) {
      throw new RangeError(
        `Amplitude must be non-negative, got ${amp}. ` +
          `A negative amplitude doesn't make physical sense — ` +
          `use a phase shift of π to invert the wave instead.`
      );
    }

    const freq = requireFiniteNumber("Frequency", frequency);
    if (freq <= 0) {
      throw new RangeError(
        `Frequency must be positive, got ${freq}. ` +
          `A wave must oscillate — zero or negative frequency ` +
          `doesn't define a wave.`
      );
    }

    if (!Number.isFinite(2.0 * PI * freq)) {
      throw new RangeError("Angular frequency must be finite");
    }

    const ph = requireFiniteNumber("Phase", phase);

    this.amplitude = amp === 0 ? 0.0 : amp;
    this.frequency = freq;
    this.phase = ph === 0 ? 0.0 : ph;
    Object.freeze(this);
  }

  private validateState(): void {
    if (
      typeof this.amplitude !== "number" ||
      !Number.isFinite(this.amplitude) ||
      this.amplitude < 0 ||
      typeof this.frequency !== "number" ||
      !Number.isFinite(this.frequency) ||
      this.frequency <= 0 ||
      !Number.isFinite(2.0 * PI * this.frequency) ||
      typeof this.phase !== "number" ||
      !Number.isFinite(this.phase)
    ) {
      throw new RangeError("Wave state must be finite and within physical domain");
    }
  }

  /** Time for one complete cycle in seconds (`T = 1 / f`). */
  period(): number {
    this.validateState();
    return 1.0 / this.frequency;
  }

  /** Angular frequency in radians per second (`ω = 2π · f`). */
  angularFrequency(): number {
    this.validateState();
    return 2.0 * PI * this.frequency;
  }

  /** Evaluate the wave at time `t` in seconds. */
  evaluate(t: number): number {
    this.validateState();
    const time = requireFiniteNumber("Time", t);
    if (this.amplitude === 0.0) {
      return 0.0;
    }

    const twoPI = 2.0 * PI;
    const period = 1.0 / this.frequency;
    const reducedTime = Number.isFinite(period) ? time % period : time;
    const reducedPhase = this.phase % twoPI;
    const angle = twoPI * (this.frequency * reducedTime) + reducedPhase;
    const unit = Math.max(-1.0, Math.min(1.0, sin(angle)));
    const result = this.amplitude * unit;
    if (!Number.isFinite(result)) {
      throw new RangeError("Evaluated wave value must be finite");
    }
    return result === 0 ? 0.0 : result;
  }
}
