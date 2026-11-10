# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [0.1.1] - 2026-11-10

### Security

- SEC-004: `decode/2` was O(n^2) (`list ++ [x]`, `Enum.at`). It now accumulates
  into a binary with O(1) append and `:binary.at/2`; output is identical for
  valid input. A token whose offset exceeds the output so far now raises
  `ArgumentError` instead of reading from the wrong position.
- A negative or non-integer copy length raises `ArgumentError`; before, it
  never reached the copy loop's base case and grew the output forever.

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
