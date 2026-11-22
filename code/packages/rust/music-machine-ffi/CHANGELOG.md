# Changelog

All notable changes to `music-machine-ffi` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Formal Warrant specification `music-machine-ffi.warrant` (synced with `code/specs/music-machine-ffi.warrant`).
- `tests/test_pbt_ffi.rs` using `SplitMix64` and bit/byte mutation fuzzers for randomized stress testing across valid, malformed, interior-NUL, NULL-pointer, oversized, and interleaved alloc/free patterns.
- Expanded `tests/test_ffi_safety.rs` regression tests for interior NUL rejection, pre-flight `sample_rate_hz` / `max_samples` bounds, output pointer aliasing/alignment guards, empty portable scores, and clipped sample count parity.

### Changed

- Hardened all `extern "C"` FFI entry points in `src/lib.rs`:
  - Wrapped entire function bodies in `std::panic::catch_unwind` at every `extern "C"` boundary.
  - Added out-pointer alignment and pairwise distinctness checks (`validate_and_init_out_ptrs`) before dereferencing or writing.
  - Added pre-flight bounds on `sample_rate_hz` (`[1000, 192000]` when non-zero) and `max_samples` (`<= MAX_SAMPLE_COUNT = 16,777,216`) plus address-space wraparound and interior NUL byte rejection in `read_bounded_utf8`.
  - Fixed `mm_render_score_text_to_pcm16` to report `rendered.clipped_sample_count` instead of `rendered.pcm_buffer.clipped_sample_count()`.
  - Hardened `mm_free_buffer` with alignment, `MAX_SAMPLE_COUNT`, and `isize::MAX` byte-size guards plus volatile zero-on-free memory wiping before deallocation.
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
