# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C++17, header-only port of the Rust `lzss` crate (CMP02), in namespace
  `ca::lzss`: the LZSS sliding-window compression algorithm with the same greedy
  longest-match encoder, overlap-safe decoder, and wire format.
- API: `encode` / `decode` (`decode` takes `std::optional<std::size_t>`),
  `serialise` / `deserialise`, and one-shot `compress` / `decompress` over
  `std::vector<std::uint8_t>`. `Token` has `lit` / `match` factories and
  `operator==`.
- Robustness: decoding skips malformed matches (offset 0 or beyond the output),
  caps the block count to the payload, and bounds the output by the declared
  length; the overlap copy reads into a local before `push_back` to avoid a
  self-referencing reallocation.
- Tests use the crate's own token vectors, window/match limits, decode overlap
  cases, text/binary round trips, and a malformed-input safety check, under GCC
  and Clang via `iso-harness`.
