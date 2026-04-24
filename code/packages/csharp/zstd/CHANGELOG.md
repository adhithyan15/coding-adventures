# Changelog — CodingAdventures.Zstd (C#)

## [0.1.0] — 2026-04-24

### Added

- `Zstd.Compress(byte[])` — pure C# ZStd frame compressor (RFC 8878).
- `Zstd.Decompress(byte[])` — pure C# ZStd frame decompressor.
- Support for Raw, RLE, and Compressed block types.
- FSE (Finite State Entropy) encode/decode using RFC 8878 Appendix B predefined distributions for LL, ML, and OF tables.
- Backward bit-stream (RevBitWriter / RevBitReader) — exact mirror of the Rust CMP07 implementation.
- LZ77 back-references via the existing LZSS C# package (32 KB window, min match 3).
- Raw literals section encoding/decoding (type 0).
- RFC-compliant Number_of_Sequences field (1/2/3-byte variable-length encoding per §3.1.1.1.2).
- 20 xunit tests covering: empty input, literal-only, all-256-bytes, RLE blocks, English prose compression ratio, pseudo-random data, multi-block (>128 KB), repeat-offset, deterministic output, and unit tests for FSE helpers and bit-stream round-trips.
