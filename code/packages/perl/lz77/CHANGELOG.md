# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [0.1.0] - 2026-04-11

### Added

- Initial implementation of LZ77 encoding/decoding (CMP00 specification)
- Streaming API: `encode` and `decode` for token stream manipulation
- One-shot API: `compress` and `decompress` for byte string I/O
- Fixed-width 4-byte token serialisation using `pack`/`unpack` with formats `N` and `nCC`
- Token hashrefs `{offset, length, next_char}`
- Byte-by-byte copy in `decode` handles overlapping matches correctly
- 23 tests, all passing — Test2::V0
- Windows not supported (BUILD_windows prints skip message)
