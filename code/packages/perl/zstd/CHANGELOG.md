# Changelog — CodingAdventures::Zstd

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-25

### Added

- Initial implementation of ZStd (RFC 8878) lossless compression and
  decompression in pure Perl.
- `compress($data)` — encodes a binary string to a valid ZStd frame.
  Supports Raw, RLE, and LZ77+FSE compressed blocks.
- `decompress($data)` — decodes any ZStd frame produced by this module
  (Predefined FSE mode, Raw_Literals).
- FSE (Finite State Entropy) encode and decode tables built from the
  RFC 8878 Appendix B predefined distributions for LL, ML, and OF.
- `RevBitWriter` / `RevBitReader` — backward bit-stream implementation
  matching the ZStd sequence section wire format.
- `_build_decode_table` and `_build_encode_sym` — full FSE table
  construction algorithms (spread + phase-3 nb/base assignment).
- Literals section: Raw_Literals encoding and decoding with 1/2/3-byte
  variable-length headers.
- Sequences section: FSE-encoded LL/OF/ML triples with predefined mode.
- Multi-block support: inputs larger than 128 KB are split into separate
  blocks automatically.
- Test suite (`t/zstd.t`) with 19 subtests covering:
  - TC-1 through TC-9 as specified (empty, single byte, all 256 values,
    RLE, prose ratio, LCG random, 200 KB multi-block, 300 KB repetitive,
    bad magic error)
  - RT-1 through RT-10 additional round-trip tests
  - UNIT-1 through UNIT-6 internal helper tests

### Implementation Notes

- Uses `CodingAdventures::LZSS` for LZ77 match-finding (32 KB window,
  max match 255, min match 3).
- The `RevBitReader` shift register is 64-bit; arithmetic uses
  `& 0xFFFFFFFFFFFFFFFF` masks to stay within Perl's native UV range on
  64-bit systems.
- Offset encoding adds +3 to avoid the ZStd reserved repeat-offset values
  (1, 2, 3), matching the Rust reference implementation.
