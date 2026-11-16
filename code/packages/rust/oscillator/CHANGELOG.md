# Changelog

All notable changes to the `oscillator` crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
