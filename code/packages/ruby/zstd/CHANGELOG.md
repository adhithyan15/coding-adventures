# Changelog — coding_adventures_zstd

All notable changes to this package are recorded here.
Follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) conventions.

## [0.1.1] — 2026-04-26

### Tests

- Added `TestSeqCount#test_low_byte_lt_128_regression` covering counts whose
  low byte is < 128 (128, 256, 300, 515, 768, 1024, 32258). These would have
  silently round-tripped wrong if the encoder ever regressed to the
  byte-swapped form (`[count & 0xFF, (count >> 8) | 0x80]`) that broke the
  TS and Go ports — see PR #1448. Also asserts that `byte0 ≥ 128` for the
  2-byte form, locking in the wire-format invariant.
- Audited `encode_seq_count` / `decode_seq_count`: already RFC 8878
  §3.1.1.3.1-compliant (`[(count >> 8) | 0x80, count & 0xFF]`); no fix
  needed in Ruby.

## [0.1.0] — 2026-04-24

### Added

- Initial implementation of ZStd (RFC 8878) compress/decompress — CMP07.
- `CodingAdventures::Zstd.compress(data)` — produces a valid ZStd frame with:
  - 4-byte magic (0xFD2FB528 LE)
  - 1-byte FHD (Single_Segment=1, FCS_Field_Size=8 bytes)
  - 8-byte Frame_Content_Size
  - One or more blocks (RLE / Compressed / Raw) up to 128 KB each
- `CodingAdventures::Zstd.decompress(data)` — decodes any ZStd frame with:
  - Raw, RLE, and Compressed block types
  - Predefined FSE modes (LL, ML, OF)
  - 256 MiB output cap (zip-bomb guard)
- `RevBitWriter` — backward bit accumulator with sentinel-bit flush
- `RevBitReader` — backward bit reader with left-aligned 64-bit register
- `build_decode_table` — FSE decode table from RFC 8878 Appendix B distributions
- `build_encode_tables` — FSE encode tables (delta_nb / delta_fs / state table)
- Raw_Literals section encoder/decoder with 1/2/3-byte headers
- Sequence count encoder/decoder (RFC 8878 §3.1.1.3.1 format)
- FSE sequence section encoder/decoder using predefined LL/ML/OF tables
- 54 unit and integration tests (TC-1 through TC-9 + helpers)

### Implementation Notes

- Sequence count encoding follows RFC 8878 §3.1.1.3.1: 2-byte form stores
  `byte[0] = 0x80 | (count >> 8)`, `byte[1] = count & 0xFF`. This differs
  from a naïve LE-u16-with-high-bit approach (which fails for counts whose
  low byte is < 128 when the overall count is >= 128).
- LZSS window is 32 KB (wider than the 4 KB LZSS package default) for better
  match ratios at the cost of O(window × block) compression time.
- All string operations use ASCII-8BIT encoding throughout to avoid Ruby
  encoding compatibility errors with high-byte values.

### Dependencies

- `coding_adventures_lzss ~> 0.1` — LZ77 token generation
