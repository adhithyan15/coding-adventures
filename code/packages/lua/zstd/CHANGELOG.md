# Changelog

## [0.1.1] - 2026-04-27

### Fixed

- **`decompress` now rejects trailing bytes after the last block.**  The block
  loop previously broke on `last_block == 1` and returned successfully even if
  bytes remained in the input.  Garbage bytes, truncation artifacts, or a
  concatenated second frame were silently ignored.  A `pos <= #data` check
  after the loop now raises `"unexpected trailing data"`.
- New `TC-11` tests: a valid frame with 3 trailing garbage bytes is rejected;
  the same frame without trailing bytes decompresses cleanly.

## [0.1.0] - 2026-04-25

### Added
- Initial implementation of ZStd (RFC 8878) compression/decompression
- Full FSE (Finite State Entropy) encode/decode with predefined tables
- RevBitWriter/RevBitReader for ZStd's backward bitstream format
- Raw, RLE, and Compressed block types
- 256 MB decompression bomb protection
- 9 test cases covering round-trips, compression ratios, and error handling
