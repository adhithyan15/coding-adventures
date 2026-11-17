# Changelog

All notable changes to the `virtual-speaker` crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-15

### Added
- Initial safe Rust implementation of MUS01 Stage 7 (`virtual-speaker`).
- Implemented `LinearSpeakerSignal` with configurable `speaker_gain` (default `1.0`).
- Implemented `ClippingSpeakerSignal` with finite gain and positive excursion limits.
- Implemented `render_speaker_output` with uniform sampling onto discrete buffers.
- Enforced `#![forbid(unsafe_code)]` and zero external crate dependencies.
- Defensive verification bounds for CWE-400 and CWE-20 (`MAX_SAMPLE_RATE_HZ`, `MAX_SAMPLES`, non-finite validation).
- Formal warrant document `virtual-speaker.warrant` and required capabilities manifest (`[]`).
- Comprehensive test suite with SplitMix64 PBT and hostile input assertions.
