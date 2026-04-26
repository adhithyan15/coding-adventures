# Changelog — go/zstd

All notable changes to this package follow [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.0.2] — 2026-04-26

### Fixed

- **`encodeSeqCount` / `decodeSeqCount` now use RFC 8878 §3.1.1.3.1 layout.**
  The 2-byte form previously emitted bytes via
  `binary.LittleEndian.AppendUint16(nil, count|0x8000)`, placing the LOW
  byte first. The decoder branches on byte0 to determine the form: for any
  count ≥ 128 whose low byte was < 128 (e.g. count=515 → byte0=0x03), the
  decoder mis-took the 1-byte path and returned a tiny garbage count,
  mis-aligning every byte downstream. Roughly half of all counts in the
  2-byte range silently corrupted; the other half worked, so most existing
  tests passed.
- New `TestSeqCountEndiannessRegression` round-trips 200 KB of repetitive
  text (> 128 sequences in a single block).
- New `TestEncodeSeqCountRoundTrip` exhaustively round-trips every count in
  the 1-byte range (0..127), the entire 2-byte range (128..0x7EFF), and a
  spot of 3-byte values.

## [0.0.1] — 2026-04-24

### Added

- Initial implementation of Zstandard (ZStd) compression/decompression (CMP07).
- `Compress(data []byte) []byte` — encodes a byte slice to a valid ZStd frame
  (RFC 8878). Automatically selects the best block type per 128 KB block:
  - RLE block when all bytes are identical (1 byte payload).
  - Compressed block when FSE + LZ77 shrinks the data.
  - Raw block as fallback when compression is not beneficial.
- `Decompress(data []byte) ([]byte, error)` — decodes any RFC 8878-compliant ZStd
  frame. Supports Raw, RLE, and Compressed (Predefined FSE) block types.
  Caps output at 256 MB to prevent decompression bombs.
- FSE (Finite State Entropy) encode and decode tables built from the predefined
  distributions in RFC 8878 Appendix B (LL acc_log=6, ML acc_log=6, OF acc_log=5).
- Reverse bit-writer (`revBitWriter`) and reverse bit-reader (`revBitReader`)
  implementing ZStd's backward-written bitstream with sentinel-bit framing.
- Internal `buildDecodeTable` and `buildEncodeTable` with the two-pass spread
  algorithm matching the reference implementation.
- Literals section encoding and decoding for Raw_Literals type with 1/2/3-byte
  headers covering sizes up to 1 MB.
- Sequence count encoding and decoding covering all three byte-width ranges.
- `llToCode` and `mlToCode` helpers for mapping values to RFC 8878 code numbers.
- Depends on `go/lzss` (CMP02) for LZ77 match-finding with a 32 KB window.
- 51 unit tests: TC-1 through TC-10 (matching the Rust reference tests) plus
  round-trip tests, wire-format validation, FSE codec unit tests, and error-path
  tests. Coverage: 93.7% of statements.
