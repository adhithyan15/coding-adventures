# Changelog

All notable changes to `flash-audio-bridge` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Security
- Added a zero-allocation dry-run pass in `decode_adpcm_into` validating `code_size`, all packet headers (`step_index < 89`), and bit length before writing into `out` so mid-stream `Err` paths preserve `out` untouched.
- Enforced upfront capacity checks in `push_stream_block` (`mono_frames <= STREAM_FIFO_CAPACITY.saturating_sub(self.stream_fifo.len())` and ADPCM `est_samples`) and `bridge_render_to_ring_buffer` (`frame_samples <= N.saturating_sub(ring.len())`) so `RingBufferOverrun` never leaves partial FIFO, voice, or telemetry mutations.
- Made `advance_frame_clock` commit `self.clock` only after `checked_mul` and `checked_add` succeed, and validated `class_name` (non-empty, `<= MAX_CLASS_NAME_BYTES`, no NUL bytes) in `register_pcm_sound` and `apply_start_sound2`.
- Added `tests/hostile_inputs.rs` (`#![forbid(unsafe_code)]`) covering multi-packet ADPCM corruption, odd-byte PCM16/stereo slices, MP3 header edge cases, atomic buffer/state preservation on `Err`, `u64::MAX` clock delta, and a seeded 1,500-case hostile fuzz campaign.

## [0.1.0] - 2026-11-21

### Added
- Initial implementation of `SWF06: flash-audio-bridge`.
- Bounded SWF audio tag parsers for `DefineSound` (Tag 14), `StartSound` (Tag 15), `StartSound2` (Tag 89), `SoundStreamHead` (Tag 18), `SoundStreamHead2` (Tag 45), and `SoundStreamBlock` (Tag 19), with `swf_container::SwfTag` dispatch (`parse_swf_audio_tag`).
- Zero-allocation PCM decoders (`decode_pcm8_into`, `decode_pcm16_le_into`, `decode_pcm16_ne_into`, `mix_stereo_to_mono_into`) using `.as_chunks::<2>()`.
- Zero-allocation Flash ADPCM decoder (`decode_adpcm_into`) supporting 2-bit, 3-bit, 4-bit, and 5-bit mono and stereo multi-packet streams.
- MP3 frame header validator (`parse_mp3_frame_header`) and zero-allocation bounded iterator (`Mp3FrameIter`, `extract_define_sound_mp3_info`).
- Deterministic fixed-point event sound mixer (`FlashAudioState`, `render_flash_audio_into`), ActionScript `Sound.start()` / `stopAllSounds()` helpers, streaming `PcmRingBuffer` FIFO, `game-clock` frame synchronization (`advance_frame_clock`), and `pcm-audio` / `audio-device-sink` export bridges.
- Conformance test suite (`tests/conformance.rs`), zero-allocation `CountingAllocator` microbenchmarks (`tests/microbenchmarks.rs`), and runtime capability conformance gate (`tests/runtime_capability.rs`).
- Formal verification and security assurance files: `code/specs/flash-audio-bridge.warrant`, `required_capabilities.json`, `assurance-profile.json`, `effect_certificate.json`, and 26-CWE `assurance_report.json`.
