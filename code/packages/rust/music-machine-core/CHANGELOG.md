# Changelog

All notable changes to `music-machine-core` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- The shared Tier-0 golden (`specs/fixtures/music-machine/expected_render.json`) was regenerated once, for all three lanes together, after the Python, Rust and TypeScript `trig` ports converged on TRIG-REF-1 (PHY00a, TC-2): new `pcm16_le_sha256` `8b578c82...c730aa` (was `15c793ee...364766`); 33 of 96,138 samples moved by 1 LSB; the first and last 32 samples are unchanged. `test_golden_parity` carries the new digest.

## [0.1.0] - 2026-11-15

### Added
- Initial implementation of `music-machine-core` (`MUS01`, `MUS03`, `MUS04`, `MUS07`, and `VER04`).
- Iterative `TextScore` (V1) parser with zero regex (CWE-1333) and zero recursion (CWE-674).
- `PortableScore` (V2) parser with multi-track polyphony, instrument declarations, and event scheduling.
- Deterministic canonical JSON serialization and deserialization with `MAX_JSON_DEPTH = 32`.
- Additive harmonic synthesis, ADSR envelope shaping, and naive preset resolution via `musical-instruments`.
- Percussion kit sequencing and hi-hat choke integration via `percussion-kit`.
- Saturating PCM polyphonic mixer with clipped-sample tracking.
- Pre-flight bounds checking for CWE-400 and CWE-190 resource defense.
- RIFF/WAVE file reading and writing via `wav-sink`.
- Formal warrant specification `music-machine-core.warrant`.
