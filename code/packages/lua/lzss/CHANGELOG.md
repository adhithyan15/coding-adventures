# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

### Security

- No code change: block_count was already capped, invalid offsets are skipped and decode stops at original_length. NOT RUN locally (toolchain unavailable in the review environment).
- Added tests/test_hostile_headers.lua (busted): hostile headers with a wall-clock bound, plus round trips over every byte string of length <= 2 and a seeded random sample. NOT RUN locally (toolchain unavailable in the review environment).

## [0.1.0] - 2026-04-11

### Added

- Initial implementation of LZSS compression (CMP02).
- `encode/4` and `encode_string/4` — sliding-window encoder; emits `Literal` or `Match` tokens (no `next_char`).
- `decode/2` and `decode_to_string/2` — token decoder with byte-by-byte overlapping match copy.
- `compress/4` — one-shot compress to CMP02 wire format.
- `decompress/1` — one-shot decompress from CMP02 wire format.
- `serialise_tokens/2` — groups tokens into flag-byte blocks (8 tokens each).
- `deserialise_tokens/1` — parses CMP02 binary; caps `block_count` to prevent DoS.
- `literal/1` and `match/2` token constructors.
- 30+ Busted tests covering spec vectors, round-trip, wire format, and compression effectiveness.
