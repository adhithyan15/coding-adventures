# Changelog

All notable changes to this package will be documented in this file.

## [0.1.1] - 2026-11-15

### Added

- Security & CWE hardening (MUS-W1-D):
  - CWE-400: bounded speaker gain with `MAX_SPEAKER_GAIN = 1_000_000.0`.
  - CWE-190 / CWE-20: strict validation rejecting `bool` and non-finite numbers (`NaN`, `±Inf`) on queries, gain, and signal evaluation results.
- Added VER specification `virtual_speaker.warrant` and `code/specs/virtual-speaker.warrant` matching grammar v0 specification.
- Added SplitMix64 deterministic PRNG property-based testing (PBT) suite achieving 100% test coverage.

## [0.1.0] - 2026-04-20

### Added

- Added an `AnalogSignal` protocol for time-queryable virtual signals.
- Added a linear speaker model that scales an input signal into a pressure proxy.
