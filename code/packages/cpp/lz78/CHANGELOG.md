# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Security

- No code change: token count was already clamped, decode stops at original_length and guards dictionary references.
- Added hostile-header tests (header-claimed counts/lengths of 2^32-1, counts beyond the data) and round trips over every byte string of length <= 2 plus a seeded random sample to tests/*_test.cpp. Verified clean under -fsanitize=address,undefined (g++) as well as the iso-harness g++/clang++ runs.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C++17, header-only port of the Rust `lz78` crate (CMP01), in namespace
  `ca::lz78`: the LZ78 lossless compression algorithm with the same trie-cursor
  encoder, parallel-dictionary decoder, and wire format.
- API: `encode` / `decode` (`decode` takes `std::optional<std::size_t>`),
  `compress` / `decompress` (one-shot CMP01 wire format), and the reusable
  `TrieCursor` (`step` / `insert` / `reset` / `dict_id` / `at_root`), built on
  `std::vector` and `std::unordered_map`.
- Robustness: `decode` / `decompress` bounds- and cycle-check the dictionary so
  malformed input cannot read out of bounds or loop forever (the Rust would
  panic / hang); output is identical for well-formed streams.
- Tests use the crate's own token vectors, text and binary round trips, the
  max-dict cap, the wire-size invariant, determinism, and a malformed-input
  safety check, under GCC and Clang via `iso-harness`.
