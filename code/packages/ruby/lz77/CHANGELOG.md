# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [0.1.0] - 2026-04-11

### Added

- Initial implementation of LZ77 encoding/decoding (CMP00 specification)
- Streaming API: `encode` and `decode` for token stream manipulation
- One-shot API: `compress` and `decompress` for byte I/O
- Fixed-width 4-byte token serialisation format using Ruby's `Array#pack`
- `Token` struct with `offset`, `length`, `next_char` fields
- Byte-by-byte copy in `decode` handles overlapping matches correctly
- Comprehensive test suite with 27 tests, 134 assertions, 0 failures
- Module: `CodingAdventures::LZ77`, requires `coding_adventures_lz77`
