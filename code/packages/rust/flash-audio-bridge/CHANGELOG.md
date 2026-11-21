# Changelog

All notable changes to `flash-audio-bridge` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
