# Changelog

## [0.1.0] - 2026-04-12

### Added

- Initial implementation of Huffman encoding and decoding (CMP04 spec).
- `compress` function: frequency histogram → DT27 tree → canonical codes →
  LSB-first bit packing → CMP04 wire format.
- `decompress` function: header parsing → canonical code reconstruction →
  LSB-first bit unpacking → prefix-free symbol decoding.
- `pack_bits_lsb_first` and `unpack_bits_lsb_first` internal helpers.
- Edge case handling: empty input (8-byte header only), single distinct symbol.
- Error handling: malformed header, truncated code-lengths table, exhausted
  bit stream all return `Err(String)` rather than panicking.
- 26 unit tests covering round-trips, wire format bytes, bit I/O helpers,
  compression effectiveness, error paths, and determinism.
- Depends on `huffman-tree` (DT27) for all tree construction and code derivation.
