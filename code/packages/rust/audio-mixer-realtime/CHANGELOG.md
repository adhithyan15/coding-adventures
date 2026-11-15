# Changelog

All notable changes to the `audio-mixer-realtime` crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.1.0 — 2026-11-15

### Added
- Initial release of `audio-mixer-realtime`: deterministic fixed-point real-time audio mixer and voice allocator (`MUS08`).
- Pure Knuth-style literate implementation in `src/lib.rs` under `#![forbid(unsafe_code)]`.
- Core data structures:
  - `VoiceSlot`: `Copy` value record tracking active state, voice ID, priority, cursor, Q16 gain, Q16 pan, Q16 pitch step, Q48.16 phase accumulator, and looping flag.
  - `RealtimeMixerState`: 16-channel voice manager with master gain, sample rate validation, clipping telemetry counter, and frame statistics.
  - `PcmRingBuffer<const N: usize>`: single-producer / single-consumer ring buffer with bounded storage and overrun/underrun accounting without dynamic reallocation.
  - `MixerError`: comprehensive error enum covering invalid sample rates, parameters, channel exhaustion, buffer underruns, and sink integration errors.
- Core public functions:
  - `mix_channels_into`: zero-heap-allocation multi-voice fixed-point mixing routine with 64-bit accumulators and fail-safe saturation clamping to `[-32768, 32767]`.
  - `export_to_sink_buffer`: zero-copy/owned bridge to `audio_device_sink::PcmPlaybackBuffer`.
  - `export_to_pcm_audio`: container bridge to `pcm_audio::PcmAudio`.
  - `clamp_sample`, `scale_voice_sample`, `validate_sample_rate`: pure mathematical kernel operations.
- Verification and Assurance:
  - `VER07` zero-allocation microbenchmarks (`tests/microbenchmarks.rs`) validating hard zero-heap allocation across `mix_channels_into`, `PcmRingBuffer::push/pop`, and `allocate_voice`, including active counting allocator validation and sabotage detection.
  - Full conformance suite (`tests/conformance.rs`) covering 16-channel full-scale saturation without integer wrap-around (`CWE-190`), ring buffer overrun handling without dynamic resize (`CWE-400`), source slice bounds safety (`CWE-125`), pitch shifting, gain attenuation, priority voice stealing, looping, and sink roundtrip.
  - `VER08b` runtime capability confinement test (`tests/runtime_capability.rs`).
  - Closed capability declaration in `required_capabilities.json` (`capabilities: []`) verified by `ca-capability-analyzer`.
  - Effect certificate (`effect_certificate.json`) proved zero side effects.
  - Assurance report (`assurance_report.json`) addressing all 26 CWE classes per `VER15`.
  - Formal Warrant specification in `code/specs/audio-mixer-realtime.warrant`.
