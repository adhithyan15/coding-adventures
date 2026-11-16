# Changelog

## 0.1.1 - 2026-11-15

- Security & CWE hardening (MUS-W1-D):
  - CWE-400 (Resource Exhaustion Defense): enforced `MAX_SAMPLE_RATE_HZ = 192_000.0`, `MAX_DURATION_SECONDS = 60.0`, and `MAX_SAMPLES = 16_777_216` before buffer allocations.
  - CWE-190 / CWE-20 (Overflow & Input Validation): strict type checks rejecting `bool` where numeric input is expected; strict rejection of `NaN`, `±Inf`, and negative or out-of-range rates/durations.
  - CWE-674 / CWE-1333: eliminated direct recursion in `UniformSampler.nyquist_frequency`; verified zero regex (`import re`) and zero unbounded recursion via AST scanner.
- Added VER specification `oscillator.warrant` matching grammar v0 specification.
- Added SplitMix64 deterministic PRNG property-based tests (PBT) and adversarial fuzzing suite achieving 100% code coverage.

## 0.1.0 - 2026-04-20

- Added the initial virtual oscillator package for Python.
- Added `ContinuousSignal`, `SineOscillator`, `SquareOscillator`,
  `UniformSampler`, and `SampleBuffer`.
- Added helper functions for sample counts, sample times, and Nyquist
  frequency.
- Added parity tests from `OSC00: Oscillator and Sampler`.
