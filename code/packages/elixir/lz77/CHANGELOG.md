# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [0.1.0] - 2026-04-11

### Added

- Initial implementation of LZ77 encoding/decoding (CMP00 specification)
- Streaming API: `encode/4` and `decode/2` for token stream manipulation
- One-shot API: `compress/4` and `decompress/1` for binary I/O
- Fixed-width 4-byte token serialisation using Elixir bitstring syntax
- Token maps: `%{offset: integer, length: integer, next_char: integer}`
- Byte-by-byte copy in `decode` handles overlapping matches correctly
- 27 tests, 0 failures
- Module: `CodingAdventures.LZ77`
