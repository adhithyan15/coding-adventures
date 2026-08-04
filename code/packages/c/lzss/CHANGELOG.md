# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C17 port of the Rust `lzss` crate (CMP02): the LZSS sliding-window
  compression algorithm with the same greedy longest-match encoder, overlap-safe
  decoder, and wire format.
- API: `lzss_encode` / `lzss_decode` (`LzssToken` arrays), `lzss_serialise` /
  `lzss_deserialise` (CMP02 wire format), and one-shot `lzss_compress` /
  `lzss_decompress` (defaults: window 4096, max match 255, min match 3).
  Returns `LZSS_OK` / `LZSS_ERR_ALLOC`; malloc-owned outputs; growable buffers
  overflow-guarded.
- Robustness: decoding skips malformed matches (offset 0 or beyond the output),
  caps the block count to the payload, and bounds the output by the declared
  length — no out-of-bounds access, no unbounded allocation.
- Tests use the crate's own token vectors, window/match limits, decode overlap
  cases, text/binary round trips, and a malformed-input safety check, under GCC
  and Clang via `iso-harness`.
