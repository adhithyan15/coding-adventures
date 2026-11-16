# Changelog

All notable changes to `percussion-kit` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.1] - 2026-11-16

### Changed

- **Fix (review of TRIG-CONV TC-2):** the pseudo-noise phase `(i + 1) * (seed * 0.1618 + 0.618)` is unbounded, and `trig.sin` is NaN beyond 2^29 radians (PHY00a 3.4.1), so the noise failed at sample 0 for seed 1e9, after about 43,000 samples for seed 12345 and at 29.6 s at 192 kHz for seed 89. The phase is now reduced to its fractional cycle (`phase - floor(phase)`, exact) before the `2*pi` multiply, identically in Python, Rust and TypeScript; the mode sines are reduced the same way. Noise samples change in the last bits. Tests at the failing seeds and counts, with the same expected values in all three lanes.
- Output changes in the last bits (and, through the hash-style noise, in a few 16-bit
  samples) because `trig::sin` is now TRIG-REF-1 (`trig` 0.2.0). No source change.

## [0.1.0] - 2026-11-15

### Added
- Complete MUS07 unpitched percussion synthesis engine in safe Rust (`#![forbid(unsafe_code)]`).
- All 15 portable starter drum kit voice profiles (`kick`, `snare`, `closed_hihat`, `open_hihat`, `pedal_hihat`, `low_tom`, `mid_tom`, `high_tom`, `crash`, `ride`, `rimshot`, `clap`, `cowbell`, `tambourine`, `shaker`, `woodblock`).
- Canonical alias resolution for common hit names and abbreviations.
- Deterministic pseudo-noise and SplitMix64 PRNG noise synthesis with zero ambient entropy.
- Single hit rendering pipelines (`render_percussion_hit`, `render_percussion_hit_pcm`).
- Streaming sequence rendering with 15 ms hi-hat choke group dynamics (`render_percussion_sequence`, `render_percussion_sequence_pcm`).
- CWE-400, CWE-190, CWE-20, and CWE-674 security hardening and pre-flight validation.
- VER warrant specifications (`percussion-kit.warrant` and `code/specs/percussion-kit.warrant`).
- Capability taxonomy declaration (`required_capabilities.json` with `capabilities: []`).
- Deterministic SplitMix64 PBT, hostile input, hi-hat choke, and golden parity test suites.
