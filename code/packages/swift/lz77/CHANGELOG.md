# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [0.1.0] - 2026-04-11

### Added

- Initial implementation of LZ77 encoding/decoding (CMP00 specification)
- Streaming API: `encode` and `decode` for token stream manipulation
- One-shot API: `compress` and `decompress` for `[UInt8]` I/O
- Fixed-width 4-byte token serialisation using bit shifts
- `Token` struct with `offset: UInt16`, `length: UInt8`, `nextChar: UInt8`
- `serialiseTokens` and `deserialiseTokens` exported for testing
- Byte-by-byte copy in `decode` handles overlapping matches correctly
- 26 tests, 0 failures — XCTest
- BUILD_windows prints skip message (Swift not available on Windows runners)
