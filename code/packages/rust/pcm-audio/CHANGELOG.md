# Changelog

All notable changes to the `pcm-audio` crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
