# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

### Changed

- `BUILD_windows` now runs the real test suite on Windows through the shared `code/scripts/run-perl-tests-windows.ps1` helper (native Strawberry Perl, absolute `-I` paths for sibling libraries) instead of the previous skip or bare-`perl` recipe.

### Fixed

- `BUILD_windows` passes dependency library paths with `-I` instead of `set PERL5LIB=...`, which MSYS perl on CI splits on `:` and so never found.

## [0.1.1] - 2026-11-10

### Security

- SEC-003: `decompress` now treats the code-lengths table as hostile input and
  dies for `symbol_count > 256`, duplicate symbols, code lengths outside
  1..56, tables not sorted by length, over-subscribed (Kraft > 1) tables, and a
  declared `original_length` larger than 8 x payload bytes (checked before any
  work is sized from it). Bits are read with `vec` in O(1) from the payload
  instead of being expanded into a bit string, and decoding dies once the
  accumulated bits reach the longest code length. A zero `original_length`
  now short-circuits before the table is parsed. Valid encoder output still
  round-trips.

## [0.1.0] - 2026-04-12

### Added

- Initial implementation of Huffman compression (CMP04).
- `compress/1` — one-shot compress byte string to CMP04 wire format.
- `decompress/1` — one-shot decompress from CMP04 wire format.
- CMP04 wire format: 4-byte original_length + 4-byte symbol_count + code-lengths
  table (N × 2 bytes, sorted by code_length then symbol_value) + LSB-first bit stream.
- `_pack_bits_lsb_first` — packs a bit string into bytes LSB-first with zero padding.
- `_unpack_bits_lsb_first` — unpacks LSB-first bytes back to a bit string.
- `_canonical_codes_from_lengths` — reconstructs canonical Huffman codes from
  a sorted list of (symbol, code_length) pairs (DEFLATE-style).
- Depends on `CodingAdventures::HuffmanTree` (DT27) for tree construction and
  canonical code generation via `canonical_code_table()`.
- 40+ Test2::V0 tests covering round-trips, wire format verification, canonical
  code prefix-free property, compression effectiveness, determinism, and security
  (malformed/truncated input).
- BUILD file using `PERL5LIB=$(cd ../huffman-tree && pwd)/lib prove -l -v t/`.
