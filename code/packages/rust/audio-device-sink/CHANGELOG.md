# Changelog

All notable changes to `audio-device-sink` will be documented in this file.

## 0.3.0

- Upgraded `code/specs/audio-device-sink.warrant` and local `audio-device-sink.warrant` to Warrant v1 with pure oracle models, examples, and claims for `validate_format`, `frame_count_checked`, and `checked_pcm_byte_len_u64` (VER04h).
- Added `FrameCount`, `frame_count_checked`, and `checked_pcm_byte_len_u64` in `src/lib.rs` plus `evidence.bindings`, `tests/warrant_evidence.rs` (local pure-Rust SHA-256 shim keeping `Cargo.lock` untouched), and generated `tests/generated/warrant_evidence.rs` (`gen_evidence --check` + `warrant_negctl.py` negative controls).
- Added VER23/VER24 `src/capabilities.rs` and `effect_certificate.json`, VER01/VER15 `assurance-profile.json`, `evidence/test-manifest.json`, `evidence/proof-manifest.json`, and `assurance_report.json` (`ca-assurance-report --check`).
- Updated `BUILD` and `BUILD_windows` with clippy, release evidence, capability, `gen_evidence --check`, `warrant_negctl.py`, and `ca-assurance-report --check` gates.

## 0.2.0

- Added `#![forbid(unsafe_code)]` to `src/lib.rs` and local `audio-device-sink.warrant`.
- Enforced checked `u64`/`usize` sample, frame, and byte math (CWE-190 / CWE-400) with pre-flight resource caps (`MIN_SAMPLE_RATE_HZ`, `MAX_SAMPLE_RATE_HZ`, `MAX_CHANNELS`, `MAX_SAMPLES`, `MAX_PCM_BYTES`).
- Added strict CWE-20 parameter and sample validation helpers (`validate_format`, `checked_frame_count`, `checked_pcm_byte_len`, `validate_sample_rate_f64`, `validate_duration_seconds`, `checked_sample_count_for_duration`, `validate_finite_samples`) and O(1) bounded error messages (`MAX_ERROR_MESSAGE_BYTES`).
- Added `tests/test_hostile_inputs.rs` and `tests/test_pbt.rs` (SplitMix64).

## 0.1.0

- Added the backend-neutral Rust audio sink contract.
- Added `PcmFormat` and `PcmPlaybackBuffer` for mono signed 16-bit PCM.
- Added `AudioSink`, `PlaybackReport`, `AudioSinkError`, and `NoopAudioSink`.
- Added safety guardrails for sample rate, format, and blocking playback size.
