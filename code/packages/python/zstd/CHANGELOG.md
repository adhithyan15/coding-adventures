# Changelog

All notable changes to `coding-adventures-zstd` will be documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-24

### Added

- Initial implementation of Zstandard (ZStd) compression/decompression (RFC 8878, CMP07).

- **`compress(data: bytes) -> bytes`**: Compresses input data into a valid ZStd
  frame with:
  - 4-byte magic number (0xFD2FB528 LE)
  - Frame Header Descriptor with 8-byte Frame Content Size
  - Per-block strategy: RLE → Compressed (LZ77+FSE) → Raw (fallback)
  - MAX_BLOCK_SIZE of 128 KB per block

- **`decompress(data: bytes) -> bytes`**: Decompresses a ZStd frame, supporting:
  - Raw blocks (type 0)
  - RLE blocks (type 1)
  - Compressed blocks (type 2) with predefined FSE tables
  - Single-segment and multi-segment frame header layouts
  - Decompression bomb protection (256 MB cap)

- **FSE (Finite State Entropy) codec** using predefined distributions from
  RFC 8878 Appendix B for LL, ML, and OF symbol streams:
  - `_build_decode_table(norm, acc_log)`: Builds the FSE decode table
    with the two-pass spread algorithm and state assignment
  - `_build_encode_sym(norm, acc_log)`: Builds FSE encode tables (delta_nb,
    delta_fs entries and the state table)

- **Reverse bitstream I/O**:
  - `_RevBitWriter`: Accumulates bits LSB-first with sentinel-bit flushing
  - `_RevBitReader`: Reads bits from end-to-start with left-aligned 64-bit register

- **Predefined code tables** matching Rust reference implementation:
  - `LL_CODES` (36 entries): literal length (baseline, extra_bits)
  - `ML_CODES` (53 entries): match length (baseline, extra_bits)
  - `LL_NORM`, `ML_NORM`, `OF_NORM`: predefined FSE distributions

- **Literals section** in Raw_Literals format (type=0) with 1/2/3-byte headers

- **Sequence count** variable-length encoding (1–3 bytes)

- **LZSS integration**: calls `coding_adventures_lzss.encode()` with 32 KB window,
  max match 255, min match 3

- **Test suite** with 12 primary test cases (TC-1 through TC-12) plus 20+
  additional helper unit tests covering:
  - Round-trip correctness for empty, single-byte, all-bytes, RLE, prose, random,
    multi-block RLE, multi-block compressed inputs
  - Error handling for bad magic, truncated input, incompatible FSE modes
  - Wire format decompression from hand-crafted frames
  - FSE encode/decode symmetry unit tests
  - Bit I/O, literals section, and sequence count unit tests
