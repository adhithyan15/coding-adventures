# Changelog

## [Unreleased]

### Security

- CWE-789: Decode no longer calls make([]byte, 0, originalLength) with the untrusted header length (up to 4 GiB; also panicked for -1); the capacity hint is capped by the token count. Invalid match offsets are skipped instead of panicking and decoding stops at originalLength.
- Added hostile_headers_test.go: hostile headers checked for bounded time and cumulative allocation, plus round trips over every byte string of length <= 2 and a seeded random sample.

## [0.1.0] - 2026-04-11

### Added

- Initial implementation of LZSS encoding and decoding (CMP02 spec).
- `Token` struct with `KindLiteral` and `KindMatch` kinds.
- `Encode` / `Decode` token-level API.
- `Compress` / `Decompress` one-shot CMP02 wire-format API.
- Flag-byte block serialisation: 1 byte per literal, 3 bytes per match.
- Security: `block_count` capped against payload size to prevent DoS.
