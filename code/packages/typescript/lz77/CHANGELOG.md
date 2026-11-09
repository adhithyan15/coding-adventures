# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Security

- Hardening (CWE-789): deserialiseTokens clamps the untrusted header count to the complete tokens present instead of looping over the claimed count (no preallocation existed, so not a crash, but iteration no longer depends on the claim). decode stops at an invalid (zero or too-large) offset instead of pushing undefined bytes.
- Added tests/hostile_headers.test.ts (wired into the npm test scripts): hostile headers checked for bounded time, plus round trips over every byte string of length <= 2 and a seeded random sample.

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
