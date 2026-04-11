# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

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
