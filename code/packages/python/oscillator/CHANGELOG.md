# Changelog

## 0.2.1 - 2026-11-21

- VER15 CWE/CVE security hardening (MUS-W8-C):
  - CWE-20 / CWE-190: added O(1) `_safe_repr` error formatting; caught `OverflowError` on `float(...)` conversions (e.g., `10**400`, `Fraction(10**400, 3)`) into `ValueError`; guarded `_fractional_part` against non-finite phase in `SquareOscillator.value_at`; rejected subnormal sample rates whose period is non-finite; added non-finite guard in `time_at_sample`.
  - CWE-400: hardened `SampleBuffer` against generator/infinite-iterator DoS and lying `__len__` iterables (aborting at `MAX_SAMPLES` without full materialization) and rejected `str`/`bytes`/`bytearray`/non-iterables; validated `UniformSampler.samples` eagerly and checked callable `signal.value_at`.
  - Added `[tool.uv.sources]` entry for `coding-adventures-trig` (`{ path = "../trig", editable = true }`, non-workspace).
  - Added dedicated `tests/test_hostile_inputs.py` and `tests/test_pbt.py` (SplitMix64 via `coding_adventures_test_bootstrap`).

## 0.2.0 - 2026-11-16

- A non-finite phase (`frequency * time` overflowing to inf) yields NaN instead of raising `OverflowError`, matching the Rust and TypeScript lanes; finite inputs are bit-identical.

- `SineOscillator.value_at` reduces its phase to [0, 1) cycles
  (`phase - floor(phase)`, exact in binary64) before multiplying by `2*pi`.
  `trig.sin` now returns NaN beyond 2**29 radians (PHY00a 3.4.1, TRIG-CONV), and a
  440 Hz tone would have crossed that after about 54 hours; the reduction keeps
  every time finite and is also more accurate at large times.
- Test: a sine oscillator at 440 Hz stays finite and periodic far beyond 2**29
  radians.

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
