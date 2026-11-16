# Changelog

All notable changes to this package will be documented in this file.

## [0.1.1] - 2026-11-15

### Added

- Security & CWE hardening (MUS-W1-D):
  - CWE-400 (Resource Exhaustion Defense): enforced `MAX_SAMPLE_RATE_HZ = 192_000.0`, `MAX_DURATION_SECONDS = 60.0`, and `MAX_SAMPLES = 16_777_216` limits on PCM buffers.
  - CWE-190 / CWE-20 (Strict Type and Boundary Validation): reject `bool` inputs, validate 16-bit PCM integer boundaries, and reject non-finite query timestamps (`NaN`, `±Inf`).
- Added VER specification `virtual_dac.warrant` and `code/specs/virtual-dac.warrant` matching grammar v0 specification.
- Added SplitMix64 deterministic PRNG property-based testing (PBT) suite achieving 100% test coverage.

## [0.1.0] - 2026-04-20

### Added

- Added a reusable zero-order-hold virtual DAC signal.
- Added signed 16-bit PCM integer to voltage conversion.
