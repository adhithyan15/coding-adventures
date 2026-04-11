# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

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
