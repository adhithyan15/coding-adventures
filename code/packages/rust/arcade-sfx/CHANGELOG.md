# Changelog

All notable changes to `arcade-sfx` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed
- `code/specs/arcade-sfx.warrant` was not valid Warrant: `scale_gain_model` and `valid_frequency` used `/`, which Warrant does not have. Rewritten with `unwrap_or(div(a, b), 0)`. `div` floors, which matches the crate's `(sample * gain) >> 16` (arithmetic shift) exactly, including negative samples. No Rust code and no claim changed.

## [0.1.0] - 2026-11-15

### Added
- Initial implementation of `arcade-sfx` (`GAME12`):
  - Deterministic Direct Digital Synthesis (DDS) with 32-bit phase accumulator.
  - Periodic waveforms: `Square` (configurable `duty_q16`), `Triangle`, `Sawtooth`.
  - Deterministic pseudo-random `LfsrNoise` supporting 15-bit (long mode) and 7-bit (short mode) with non-zero seed guard (`CWE-20`).
  - Sample-counted `AdsrEnvelope` with Q16.16 sustain level and premature note-off release scaling.
  - Complete classic arcade preset suite: `PongPaddleHit`, `PongWallBounce`, `PongScore`, `BreakoutBrickHit { row }`, `BreakoutLoseLife`, `CoinInsert`, `LaserSweep`.
  - Zero-allocation single voice (`SfxVoice`) and multi-voice polyphonic bank (`SfxBank<const N: usize>`) with priority-based voice stealing.
  - Hard zero-allocation synthesis hot paths: `render_sfx_into` and `mix_bank_into`.
  - Integration bridges for `audio-mixer-realtime` (`PcmRingBuffer`) and `pcm-audio` (`PcmAudio`).
  - Strict security hardening against `CWE-369` (divide by zero), `CWE-190` (integer overflow / saturation clamping), and `CWE-20` (input validation).
  - Formal Warrant document `code/specs/arcade-sfx.warrant`.
  - Formal assurance profile, 26-CWE assurance report, and verified effect certificate.
  - Comprehensive unit, conformance, microbenchmark (`VER07` zero-alloc + sabotage detection), and runtime capability test suites.
