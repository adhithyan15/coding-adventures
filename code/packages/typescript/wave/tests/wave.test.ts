// ============================================================================
// wave.test.ts — Comprehensive Tests for the Wave Class
// ============================================================================
//
// These tests verify that the Wave class correctly models simple harmonic
// motion. We test:
//   1. Basic wave evaluation at key time points
//   2. Periodicity (the wave repeats after one period)
//   3. Phase shifting (starting the wave at a different point)
//   4. Derived quantities (period, angular frequency)
//   5. Input validation (rejecting unphysical parameters)
//
// We use toBeCloseTo(expected, 10) for floating-point comparisons. The
// second argument is the number of decimal digits that must match. With
// 10 digits of precision we're well within double-precision accuracy
// while allowing for minor floating-point drift.
// ============================================================================

import { Wave } from "../src/wave";
import { PI } from "trig/src/trig";

// ============================================================================
// Basic Wave Evaluation
// ============================================================================

describe("Wave evaluation", () => {
  // --------------------------------------------------------------------------
  // At t=0 with no phase offset, sin(0) = 0, so the wave should be 0.
  // This is the most fundamental test: a sinusoidal wave starts at zero
  // when there's no phase shift.
  // --------------------------------------------------------------------------

  test("wave at t=0 with phase 0 gives 0", () => {
    const wave = new Wave(5.0, 1.0, 0.0);
    expect(wave.evaluate(0.0)).toBeCloseTo(0.0, 10);
  });

  // --------------------------------------------------------------------------
  // A 1 Hz wave completes one full cycle per second. At t = 0.25 seconds
  // (quarter of a cycle), the argument to sin is:
  //   2π · 1.0 · 0.25 = π/2
  // And sin(π/2) = 1, so the wave should reach its amplitude.
  // --------------------------------------------------------------------------

  test("1 Hz wave reaches amplitude at t=0.25", () => {
    const wave = new Wave(3.0, 1.0, 0.0);
    expect(wave.evaluate(0.25)).toBeCloseTo(3.0, 10);
  });

  // --------------------------------------------------------------------------
  // At t = 0.5 (half cycle), the argument is π, and sin(π) = 0.
  // The wave crosses zero again on its way to the negative peak.
  // --------------------------------------------------------------------------

  test("1 Hz wave returns to zero at t=0.5", () => {
    const wave = new Wave(2.0, 1.0, 0.0);
    expect(wave.evaluate(0.5)).toBeCloseTo(0.0, 10);
  });

  // --------------------------------------------------------------------------
  // At t = 0.75 (three-quarter cycle), the argument is 3π/2, and
  // sin(3π/2) = -1. The wave reaches its negative peak (trough).
  // --------------------------------------------------------------------------

  test("1 Hz wave reaches negative amplitude at t=0.75", () => {
    const wave = new Wave(4.0, 1.0, 0.0);
    expect(wave.evaluate(0.75)).toBeCloseTo(-4.0, 10);
  });

  // --------------------------------------------------------------------------
  // A wave with amplitude 0 should always evaluate to 0, regardless of
  // time or frequency. This is the trivial "no wave" case.
  // --------------------------------------------------------------------------

  test("zero amplitude wave is always zero", () => {
    const wave = new Wave(0.0, 5.0, 0.0);
    expect(wave.evaluate(0.0)).toBeCloseTo(0.0, 10);
    expect(wave.evaluate(0.25)).toBeCloseTo(0.0, 10);
    expect(wave.evaluate(1.0)).toBeCloseTo(0.0, 10);
  });
});

// ============================================================================
// Periodicity
// ============================================================================

describe("Wave periodicity", () => {
  // --------------------------------------------------------------------------
  // The defining property of a periodic wave: after one full period T = 1/f,
  // the wave returns to the same value. We test this at several time points
  // to ensure it holds generally, not just at t = 0.
  // --------------------------------------------------------------------------

  test("wave repeats after one period", () => {
    const wave = new Wave(2.5, 3.0, 0.5);
    const period = wave.period();

    // Test at several time points
    for (const t of [0.0, 0.1, 0.25, 0.33]) {
      expect(wave.evaluate(t + period)).toBeCloseTo(wave.evaluate(t), 10);
    }
  });

  // --------------------------------------------------------------------------
  // Periodicity should hold over multiple periods too. After N complete
  // cycles, the wave should be back where it started.
  // --------------------------------------------------------------------------

  test("wave repeats after multiple periods", () => {
    const wave = new Wave(1.0, 2.0, 0.0);
    const t = 0.1;
    const period = wave.period();

    expect(wave.evaluate(t + 5 * period)).toBeCloseTo(wave.evaluate(t), 10);
  });
});

// ============================================================================
// Phase Shifting
// ============================================================================

describe("Wave phase", () => {
  // --------------------------------------------------------------------------
  // A phase of π/2 shifts the sine wave so that it starts at its peak.
  // At t = 0:
  //   y = A · sin(0 + π/2) = A · sin(π/2) = A · 1 = A
  //
  // This is how cosine relates to sine: cos(x) = sin(x + π/2).
  // --------------------------------------------------------------------------

  test("phase PI/2 starts at peak", () => {
    const wave = new Wave(1.0, 1.0, PI / 2);
    expect(wave.evaluate(0.0)).toBeCloseTo(1.0, 10);
  });

  // --------------------------------------------------------------------------
  // A phase of π shifts the sine wave by half a cycle, inverting it.
  // At t = 0.25 (where an unshifted 1 Hz wave peaks):
  //   y = A · sin(2π · 0.25 + π) = A · sin(π/2 + π) = A · sin(3π/2) = -A
  // --------------------------------------------------------------------------

  test("phase PI inverts the wave", () => {
    const wave = new Wave(1.0, 1.0, PI);
    expect(wave.evaluate(0.25)).toBeCloseTo(-1.0, 10);
  });

  // --------------------------------------------------------------------------
  // Phase is preserved as a readonly property.
  // --------------------------------------------------------------------------

  test("phase is stored correctly", () => {
    const wave = new Wave(1.0, 1.0, 1.234);
    expect(wave.phase).toBe(1.234);
  });

  // --------------------------------------------------------------------------
  // Default phase is 0 when not specified.
  // --------------------------------------------------------------------------

  test("default phase is 0", () => {
    const wave = new Wave(1.0, 1.0);
    expect(wave.phase).toBe(0.0);
  });
});

// ============================================================================
// Derived Quantities
// ============================================================================

describe("Wave derived quantities", () => {
  // --------------------------------------------------------------------------
  // Period T = 1/f
  //
  // A 2 Hz wave completes 2 cycles per second, so each cycle takes 0.5 s.
  // A 440 Hz wave (concert A) has a period of about 2.27 ms.
  // --------------------------------------------------------------------------

  test("period is reciprocal of frequency", () => {
    const wave = new Wave(1.0, 2.0);
    expect(wave.period()).toBeCloseTo(0.5, 10);
  });

  test("period of 1 Hz wave is 1 second", () => {
    const wave = new Wave(1.0, 1.0);
    expect(wave.period()).toBeCloseTo(1.0, 10);
  });

  test("period of 440 Hz wave", () => {
    const wave = new Wave(1.0, 440.0);
    expect(wave.period()).toBeCloseTo(1.0 / 440.0, 10);
  });

  // --------------------------------------------------------------------------
  // Angular frequency ω = 2πf
  //
  // This converts cycles-per-second to radians-per-second. Since one
  // cycle = 2π radians, multiplying frequency by 2π gives the angular rate.
  // --------------------------------------------------------------------------

  test("angular frequency of 1 Hz wave is 2*PI", () => {
    const wave = new Wave(1.0, 1.0);
    expect(wave.angularFrequency()).toBeCloseTo(2.0 * PI, 10);
  });

  test("angular frequency of 50 Hz wave", () => {
    const wave = new Wave(1.0, 50.0);
    expect(wave.angularFrequency()).toBeCloseTo(100.0 * PI, 10);
  });
});

// ============================================================================
// Properties
// ============================================================================

describe("Wave properties", () => {
  test("amplitude is stored correctly", () => {
    const wave = new Wave(3.5, 2.0, 0.1);
    expect(wave.amplitude).toBe(3.5);
  });

  test("frequency is stored correctly", () => {
    const wave = new Wave(1.0, 7.5, 0.0);
    expect(wave.frequency).toBe(7.5);
  });
});

// ============================================================================
// Input Validation
// ============================================================================

describe("Wave validation", () => {
  // --------------------------------------------------------------------------
  // Negative amplitude is physically meaningless for a simple harmonic wave.
  // A negative amplitude would be equivalent to a positive amplitude with
  // a phase shift of π. We enforce non-negative to keep the model clean.
  // --------------------------------------------------------------------------

  test("negative amplitude throws", () => {
    expect(() => new Wave(-1.0, 1.0)).toThrow("Amplitude must be non-negative");
  });

  // --------------------------------------------------------------------------
  // Zero frequency means the "wave" never oscillates — it's just a constant.
  // That's not a wave, so we reject it.
  // --------------------------------------------------------------------------

  test("zero frequency throws", () => {
    expect(() => new Wave(1.0, 0.0)).toThrow("Frequency must be positive");
  });

  // --------------------------------------------------------------------------
  // Negative frequency is mathematically equivalent to a phase-shifted
  // positive frequency wave, so we reject it to avoid ambiguity.
  // --------------------------------------------------------------------------

  test("negative frequency throws", () => {
    expect(() => new Wave(1.0, -5.0)).toThrow("Frequency must be positive");
  });
});

// ============================================================================
// Higher Frequency Waves
// ============================================================================

describe("Wave with higher frequency", () => {
  // --------------------------------------------------------------------------
  // A 10 Hz wave has a period of 0.1 seconds. At t = 0.025 (quarter period),
  // it should reach its amplitude. This tests that the frequency parameter
  // correctly scales the oscillation rate.
  // --------------------------------------------------------------------------

  test("10 Hz wave peaks at t = 0.025", () => {
    const wave = new Wave(2.0, 10.0, 0.0);
    expect(wave.evaluate(0.025)).toBeCloseTo(2.0, 10);
  });

  test("10 Hz wave zero at t = 0.05", () => {
    const wave = new Wave(2.0, 10.0, 0.0);
    expect(wave.evaluate(0.05)).toBeCloseTo(0.0, 10);
  });
});
