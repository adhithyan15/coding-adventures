# Changelog

All notable changes to `music-machine-ffi` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- `tests/test_ffi_golden.rs` carries the new shared Tier-0 golden digest `25bc4eb1b0d7a10d40a55afc0f8b0b4d65132a82f538245ee2698700ff4acee1` (was `15c793ee...364766`), regenerated once for the Python, Rust and TypeScript lanes after TRIG-CONV TC-2 (PHY00a). Both FFI golden parity tests (portable text and JSON) pass.

## [0.1.0] - 2026-11-15

### Added
- Initial C ABI implementation of `music-machine-ffi`.
- `mm_render_score_text_to_pcm16` for V1 beginner sheet music.
- `mm_render_portable_json_to_pcm16` for V2 canonical JSON multi-track scores.
- `mm_render_portable_text_to_pcm16` for V2 canonical text multi-track scores.
- `mm_free_buffer` for safe, single-owner PCM buffer deallocation.
- C header `include/music_machine_ffi.h`.
- Hardened memory safety (CWE-476, CWE-125, CWE-787, CWE-190, CWE-415, CWE-416).
- Zero-capability declaration in `required_capabilities.json`.
