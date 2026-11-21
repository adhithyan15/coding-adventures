# Changelog

All notable changes to `audio-device-sink` will be documented in this file.

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
