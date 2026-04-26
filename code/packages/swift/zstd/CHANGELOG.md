# Changelog — swift/zstd

All notable changes to the `swift/zstd` package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.1] — 2026-04-26

### Tests

- Added `testSeqCountEndiannessRegression`. The test round-trips 200 KB of
  repetitive ASCII, reliably yielding ≥ 128 sequences in a single block —
  exercising the 2-byte path of `encodeSeqCount` / `decodeSeqCount`. Same
  shape as the regression added to TS+Go in PR #1448.
- Audited `encodeSeqCount` / `decodeSeqCount`: already RFC 8878
  §3.1.1.3.1-compliant (`0x80 | (count >> 8), count & 0xFF`); no fix needed.

## [0.1.0] — 2026-04-24

### Added

- Initial implementation of ZStd (RFC 8878) compression and decompression in Swift.
- `compress(_:) -> [UInt8]` — single-shot compressor producing a standards-compliant
  ZStd frame.
- `decompress(_:) throws -> [UInt8]` — single-shot decompressor with 256 MiB output cap.
- `ZstdError` — typed error enum covering all failure modes (bad magic, truncated input,
  unsupported FSE modes, invalid offsets, etc.).
- **FSE predefined tables** from RFC 8878 Appendix B for Literal Length (accLog=6),
  Match Length (accLog=6), and Offset (accLog=5).
- **`buildDecodeTable`** — three-phase FSE decode table builder matching the Rust
  reference implementation exactly (rare-symbols-first spreading + nb/base assignment).
- **`buildEncodeTable`** — symmetric FSE encode table builder returning `([FseEe], [UInt16])`.
- **`RevBitWriter`** / **`RevBitReader`** — backward bit-stream codec used by the ZStd
  sequence section (sentinel-bit framing, left-aligned register on decode).
- **Block strategies**: Raw, RLE (all-bytes-identical detection), Compressed (LZ77 + FSE).
- **Literals section**: Raw_Literals format with 1/2/3-byte header encoding/decoding.
- **Sequences section**: sequence count encoding, symbol-modes byte, FSE bitstream
  encode/decode for LL × OF × ML triples.
- **LL / ML code tables** (36 and 53 entries respectively) matching RFC 8878 §3.1.1.3.
- Dependency on `swift/lzss` for LZ77 match-finding (32 KB window, max match 255, min 3).
- Correct 2-byte sequence count encoding using the RFC 8878 big-endian-ish scheme
  `[0x80 | (count >> 8), count & 0xFF]` which guarantees byte_0 ∈ [0x80, 0xFE] for
  all counts in [128, 32512), avoiding the 0xFF sentinel collision that would corrupt
  the symbol-modes byte offset on decoding.
- `Package.swift` (swift-tools-version 5.9) with LZSS product dependency.
- `BUILD` script running `swift test --enable-code-coverage --verbose`.
- 20 XCTest test cases (TC-1 through TC-15 plus regression tests) covering:
  - Empty, single-byte, all-256-bytes round-trips.
  - RLE block compression ratio check.
  - English prose compression ratio (≥ 20%).
  - LCG pseudo-random data round-trip.
  - Multi-block (200 KB / 300 KB) round-trips.
  - Bad magic, too-short frame error paths.
  - Deterministic output guarantee.
  - Wire-format decode (manual frame, no encoder dependency).
  - Alternating pattern compression ratio (≥ 30%).
