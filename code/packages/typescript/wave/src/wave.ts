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
//   A = amplitude    — the maximum displacement from equilibrium
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
// ============================================================================

import { PI, sin } from "trig/src/trig";

// ============================================================================
// Wave Class
// ============================================================================
//
// The Wave class encapsulates the three parameters that fully define a
// simple harmonic wave's temporal behavior:
//
//   1. amplitude  — how "tall" the wave is (must be non-negative)
//   2. frequency  — how "fast" the wave oscillates (must be positive)
//   3. phase      — where the wave starts in its cycle (defaults to 0)
//
// Example usage:
//
//   const wave = new Wave(1.0, 440.0);  // Concert A pitch
//   const y = wave.evaluate(0.001);      // Value at t = 1ms
//
// The evaluate() method computes:
//
//   y(t) = amplitude · sin(2π · frequency · t + phase)
//
// This is the standard form of the harmonic wave equation.
// ============================================================================

export class Wave {
  // --------------------------------------------------------------------------
  // Properties
  // --------------------------------------------------------------------------
  // These are readonly because a wave's identity IS its parameters.
  // To get a different wave, create a new Wave instance.

  /** Peak displacement from zero. Always non-negative. */
  readonly amplitude: number;

  /** Number of complete oscillation cycles per second (Hz). Always positive. */
  readonly frequency: number;

  /** Initial phase offset in radians. Shifts the wave along the time axis. */
  readonly phase: number;

  // --------------------------------------------------------------------------
  // Constructor
  // --------------------------------------------------------------------------
  //
  // Validates the physical constraints:
  //   - Amplitude >= 0: A negative amplitude doesn't make physical sense.
  //     An amplitude of 0 is valid — it represents a "flat" wave (no
  //     oscillation), which is useful as a base case.
  //   - Frequency > 0: A wave with zero frequency never oscillates, so
  //     it's not really a wave. Negative frequencies are mathematically
  //     equivalent to a phase shift of π, so we disallow them to keep
  //     the model unambiguous.

  constructor(amplitude: number, frequency: number, phase: number = 0.0) {
    if (amplitude < 0) {
      throw new Error(
        `Amplitude must be non-negative, got ${amplitude}. ` +
          `A negative amplitude doesn't make physical sense — ` +
          `use a phase shift of π to invert the wave instead.`
      );
    }

    if (frequency <= 0) {
      throw new Error(
        `Frequency must be positive, got ${frequency}. ` +
          `A wave must oscillate — zero or negative frequency ` +
          `doesn't define a wave.`
      );
    }

    this.amplitude = amplitude;
    this.frequency = frequency;
    this.phase = phase;
  }

  // --------------------------------------------------------------------------
  // period() — Time for One Complete Cycle
  // --------------------------------------------------------------------------
  //
  // The period T is the reciprocal of frequency:
  //
  //   T = 1 / f
  //
  // A 1 Hz wave has a period of 1 second.
  // A 440 Hz wave (concert A) has a period of about 2.27 milliseconds.
  //
  // The relationship is intuitive: if a wave completes 2 cycles per second
  // (f = 2 Hz), then each cycle takes 0.5 seconds (T = 0.5).

  period(): number {
    return 1.0 / this.frequency;
  }

  // --------------------------------------------------------------------------
  // angularFrequency() — Frequency in Radians per Second
  // --------------------------------------------------------------------------
  //
  // Angular frequency ω converts from cycles-per-second to radians-per-second:
  //
  //   ω = 2π · f
  //
  // Why radians? Because the sine function naturally works in radians.
  // One complete cycle = 2π radians = 360°.
  //
  // A 1 Hz wave has ω ≈ 6.283 rad/s (one full rotation per second).
  // A 60 Hz wave (US power grid) has ω ≈ 376.99 rad/s.

  angularFrequency(): number {
    return 2.0 * PI * this.frequency;
  }

  // --------------------------------------------------------------------------
  // evaluate(t) — Compute the Wave's Value at Time t
  // --------------------------------------------------------------------------
  //
  // This is the heart of the wave model. Given a time t (in seconds),
  // it computes:
  //
  //   y(t) = A · sin(2πft + φ)
  //
  // Let's trace through an example:
  //
  //   Wave: A=1.0, f=1.0, φ=0.0
  //   At t=0:    y = 1.0 · sin(0)      = 0.0     (wave starts at zero)
  //   At t=0.25: y = 1.0 · sin(π/2)    = 1.0     (quarter cycle = peak)
  //   At t=0.5:  y = 1.0 · sin(π)      = 0.0     (half cycle = zero again)
  //   At t=0.75: y = 1.0 · sin(3π/2)   = -1.0    (three-quarter = trough)
  //   At t=1.0:  y = 1.0 · sin(2π)     = 0.0     (full cycle = back to start)
  //
  // The phase φ shifts this pattern. With φ = π/2:
  //   At t=0:    y = 1.0 · sin(π/2) = 1.0        (starts at peak!)

  evaluate(t: number): number {
    return this.amplitude * sin(2.0 * PI * this.frequency * t + this.phase);
  }
}
