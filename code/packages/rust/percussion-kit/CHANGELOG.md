# Changelog

All notable changes to `percussion-kit` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
