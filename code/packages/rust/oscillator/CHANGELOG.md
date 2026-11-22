# Changelog

All notable changes to the `oscillator` crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-11-16

### Changed

- `SineOscillator` now takes its sine from the bit-exact `trig` crate (TRIG-REF-1, PHY00a)
  instead of `f64::sin`, so a sine oscillator gives the same bits in Python, Rust and
  TypeScript. It reduces the phase to [0, 1) cycles (`phase - floor(phase)`, exact) first:
  `trig::sin` is NaN beyond 2^29 radians (a 440 Hz tone passes that after about 54 hours).

## [0.1.0] - 2026-11-15

### Added
- Initial safe Rust implementation of `OSC00` (Continuous Oscillators and Uniform Samplers).
- Implemented `ContinuousSignal` trait.
- Implemented `SineOscillator`, `SquareOscillator`, `PulseOscillator`, `SawtoothOscillator`, and `TriangleOscillator`.
- Implemented `SampleBuffer` and `UniformSampler`.
- Enforced `#![forbid(unsafe_code)]` and zero external crate dependencies.
- Defensive verification bounds for CWE-400, CWE-190, and CWE-20 (`MAX_SAMPLE_RATE_HZ`, `MAX_DURATION_SECONDS`, `MAX_SAMPLES`, non-finite float validation).
- Formal warrant document `oscillator.warrant` and required capabilities manifest.
- Comprehensive test suite with SplitMix64 PBT and hostile input assertions.
