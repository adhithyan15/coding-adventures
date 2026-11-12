# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

### Changed

- `BUILD_windows` now runs the real test suite on Windows through the shared `code/scripts/run-perl-tests-windows.ps1` helper (native Strawberry Perl, absolute `-I` paths for sibling libraries) instead of the previous skip or bare-`perl` recipe.

### Security

- No code change: block_count was already capped, invalid offsets are skipped and decode stops at original_length.
- Added t/02-hostile-headers.t (Test::More): hostile headers checked for bounded time and sane results, plus round trips over every byte string of length <= 2 and a seeded random sample. (The pre-existing t/*.t need Test2::V0, which is not installed in the local review environment.)

## [0.1.0] - 2026-04-11

### Added

- Initial implementation of LZSS compression (CMP02).
- `encode/4` — sliding-window encoder; emits Literal or Match tokens (no `next_char`).
- `decode/2` — token decoder with byte-by-byte overlapping match copy.
- `compress/4` — one-shot compress to CMP02 wire format.
- `decompress/1` — one-shot decompress from CMP02 wire format.
- `_serialise_tokens/2` — groups tokens into flag-byte blocks (8 tokens each).
- `_deserialise_tokens/1` — parses CMP02 binary; caps `block_count` to prevent DoS.
- `make_literal/1` and `make_match/2` token constructors.
- 30+ Test2::V0 tests covering spec vectors, round-trip, wire format, and compression effectiveness.
