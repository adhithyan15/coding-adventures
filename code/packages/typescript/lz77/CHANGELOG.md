# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [0.1.0] - 2026-04-11

### Added

- Initial implementation of LZ77 encoding/decoding (CMP00 specification)
- Streaming API: `encode` and `decode` for token stream manipulation
- One-shot API: `compress` and `decompress` for Uint8Array I/O
- Fixed-width 4-byte token serialisation using DataView (big-endian)
- `Token` interface with `offset`, `length`, `nextChar` fields
- `token()` factory function for constructing tokens
- `serialiseTokens` and `deserialiseTokens` exported for testing
- Byte-by-byte copy in `decode` handles overlapping matches correctly
- 33 tests, 0 failures
