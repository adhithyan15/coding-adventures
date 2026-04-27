# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

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
