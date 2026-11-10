# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

### Security

- Block count is capped against the payload (no decoder code change needed).
- Added test/hostile_headers_test.exs (ExUnit): hostile headers return only the data actually present, plus round trips over every byte string of length <= 2 and a seeded random sample.

### Fixed

- `encode/4` was O(n^2): every step rebuilt `:array.from_list(data)`, recomputed `length(data)` and used `Enum.at/2`. The array and length are now built once per call. Output is unchanged; a 200 KB input dropped from minutes to ~15 s, which unblocks the zstd suite (its 200 KB test timed out at 60 s on Windows CI).

## [0.1.0] - 2026-04-11

### Added

- Initial implementation of LZSS compression (CMP02).
- `encode/4` — sliding-window encoder; emits `Literal` or `Match` tokens (no `next_char`).
- `decode/2` — token decoder with byte-by-byte overlapping match copy.
- `compress/4` — one-shot compress to CMP02 wire format.
- `decompress/1` — one-shot decompress from CMP02 wire format.
- `serialise_tokens/2` — groups tokens into flag-byte blocks (8 tokens each).
- `deserialise_tokens/1` — parses CMP02 binary; caps `block_count` to prevent DoS.
- `literal/1` and `match/2` token constructors.
- 40+ ExUnit tests covering spec vectors, round-trip, wire format, and compression effectiveness.
