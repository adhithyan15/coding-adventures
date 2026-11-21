# Changelog

All notable changes to the `pcm-audio` crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-11-21

### Added
- Upgraded `code/specs/pcm-audio.warrant` and local `pcm-audio.warrant` (kept byte-for-byte in sync) to Warrant v1 with pure oracle models, hostile input contracts, resource bounds, examples, and claims for `clamp_to_pcm16`, `compute_data_size`, and `compute_wav_file_size` (VER04h).
- Added `clamp_to_pcm16`, `DataSize`, `WavFileSize`, `compute_data_size`, `compute_wav_file_size`, `MIN_WAV_HEADER_SIZE`, and `MAX_WAV_DATA_BYTES` with checked integer arithmetic (CWE-190/400/20) and hardened `PcmFormat::integer_sample_rate` / `PcmAudio::time_at` / `from_wav_bytes` boundary checks (VER15).
- Added `evidence.bindings`, `tests/warrant_evidence.rs` (local pure-Rust SHA-256 shim keeping `Cargo.lock` untouched under `--locked`), and generated `tests/generated/warrant_evidence.rs` (`gen_evidence --check` + `warrant_negctl.py` negative controls).
- Added VER23/VER24 `src/capabilities.rs` and `effect_certificate.json`, VER01/VER15 `assurance-profile.json`, `evidence/test-manifest.json`, `evidence/proof-manifest.json`, and `assurance_report.json` (`ca-assurance-report --check`).
- Updated `BUILD` and `BUILD_windows` with clippy, unit/doc/release evidence tests, capability checks, `gen_evidence --check`, `warrant_negctl.py` gates, and `ca-assurance-report --check`.

## [0.1.0] - 2026-11-15

### Added
- Initial safe Rust implementation of `MUS01` Stage 5 (`PcmAudio` and secure WAV codec).
- Implemented `PcmFormat` metadata container with defaults for 44.1 kHz, mono, 16-bit PCM.
- Implemented `safe_round_sample` / `float_to_pcm16` and `pcm16_to_f32` conversion routines.
- Implemented `PcmAudio` container supporting `i16` storage, `f32`/`f64` conversions, and `oscillator::SampleBuffer` ingestion.
- Implemented secure RIFF/WAVE serializer (`to_wav_bytes`) and parser (`from_wav_bytes`).
- Defensive verification guards against CWE-400, CWE-190, and CWE-20 (allocation bombing, integer overflow, and header validation).
- Formal warrant document `pcm-audio.warrant` and required capabilities manifest.
- Comprehensive test suite with SplitMix64 PBT, round-trip checks, and malformed/hostile input tests.
