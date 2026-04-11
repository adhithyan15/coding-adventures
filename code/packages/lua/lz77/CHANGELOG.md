# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [0.1.0] - 2026-04-11

### Added

- Initial implementation of LZ77 encoding/decoding (CMP00 specification)
- Byte array API: `encode/4`, `decode/2`
- String convenience API: `encode_string/4`, `decode_to_string/2`
- One-shot API: `compress/4`, `decompress/1`
- Fixed-width 4-byte token serialisation using `string.char`/`string.byte`
- Token tables: `{offset, length, next_char}` via `token/3` factory
- Byte-by-byte copy in `decode` handles overlapping matches correctly
- 30 tests, 0 failures — Busted test framework
