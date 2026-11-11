# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Add required_capabilities.json (pure computation).

### Security

- CWE-789/CWE-400: `deserialise_tokens` no longer calls `Vec::with_capacity` with the untrusted 4-byte header count (`decompress(&[0xFF;4])` aborted the process with a ~17 GB allocation). The token count is clamped to what the remaining bytes can hold; a header claiming more tokens than are present is treated as truncated.
- `decode` no longer panics on a match token whose offset is 0 or reaches before the start of the output; it stops at the first such token and returns the bytes produced so far. Valid streams are unchanged.
- Added `tests/hostile_headers.rs`: counting-allocator guard (single allocation > 32 MiB aborts the test binary), hostile headers, and round-trip tests over every byte string of length <= 2 plus a seeded random sample.

## [0.1.0] - 2026-04-11

### Added

- Initial implementation of LZ77 encoding/decoding (CMP00 specification)
- Streaming API: `encode` and `decode` for token stream manipulation
- One-shot API: `compress` and `decompress` for byte slice I/O
- Fixed-width 4-byte token serialisation using `to_be_bytes()` / `from_be_bytes()`
- `Token` struct with `offset: u16`, `length: u8`, `next_char: u8` fields
- `serialise_tokens` and `deserialise_tokens` public helpers
- Byte-by-byte copy in `decode` handles overlapping matches correctly
- 25 unit tests + 4 doc tests, all passing
- Registered in the Rust workspace `Cargo.toml`
