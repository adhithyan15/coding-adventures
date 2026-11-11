# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Security

- CWE-789/CWE-400: deserialiseTokens no longer calls make([]Token, 0, count) with the untrusted 4-byte header count (a crafted header caused 'fatal error: runtime: out of memory'); count is clamped to (len-4)/4. Decode stops at a zero/out-of-range offset instead of panicking.
- Added hostile_headers_test.go: hostile headers checked for bounded time and cumulative allocation, plus round trips over every byte string of length <= 2 and a seeded random sample.

### Fixed

- 32-bit portability: deserialiseTokens converts the header count with `clampCount`, which compares in uint64 before converting to int. On GOARCH=386 a header of 0xFFFFFFFF used to become -1 and make() panicked. Counts are now never negative and never exceed (len-4)/4. Covered by header_clamp_test.go, and `GOARCH=386 go test ./...` passes.

## [0.1.0] - 2026-04-11

### Added

- Initial implementation of LZ77 encoding/decoding (CMP00 specification)
- Streaming API: `Encode` and `Decode` for token stream manipulation
- One-shot API: `Compress` and `Decompress` for byte I/O
- Fixed-width 4-byte token serialisation format (teaching format, not optimised)
- Comprehensive test suite with 98.3% coverage including:
  - Specification test vectors from CMP00 spec
  - Round-trip invariant tests (table-driven)
  - Parameter constraint tests (windowSize, maxMatch, minMatch)
  - Edge cases (boundaries, overlapping matches, binary data)
  - Serialisation/deserialisation tests
  - Initial buffer support in `Decode`
- Package-level godoc with Knuth-style literate programming
- Full sliding-window diagram and overlapping-match explanation in godoc
