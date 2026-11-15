# Changelog

All notable changes to the `virtual-dac` crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-15

### Added
- Initial safe Rust implementation of MUS01 Stage 6 (`virtual-dac`).
- Implemented `ZeroOrderHoldDACSignal` for staircase voltage reconstruction.
- Implemented `LinearInterpolationDACSignal` for continuous piecewise-linear reconstruction.
- Implemented `clamp_sample_index` with robust `min(max(idx, 0), count - 1)` float clamping.
- Implemented `pcm16_to_voltage` with asymmetric full-scale scaling.
- Enforced `#![forbid(unsafe_code)]` and zero external crate dependencies.
- Defensive verification bounds for CWE-20 and float rounding out-of-bounds guards.
- Formal warrant document `virtual-dac.warrant` and required capabilities manifest (`[]`).
- Comprehensive test suite with SplitMix64 PBT and hostile input assertions.
