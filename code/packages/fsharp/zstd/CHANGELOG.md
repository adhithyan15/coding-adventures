# Changelog — CodingAdventures.Zstd.FSharp

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-24

### Added

- Initial implementation of ZStd (RFC 8878 / CMP07) compression in pure F#.
- `Zstd.Compress(data: byte[]) : byte[]` — produces a valid ZStd frame with
  magic number, Frame Header Descriptor, 8-byte Frame_Content_Size, and one
  or more data blocks.
- `Zstd.Decompress(data: byte[]) : byte[]` — decodes Raw, RLE, and
  Compressed blocks; validates magic; skips Window_Descriptor and Dict_ID
  fields as required by RFC 8878.
- FSE (Finite State Entropy) encode and decode tables built from the
  predefined distributions in RFC 8878 Appendix B — no per-frame table
  description overhead.
- `RevBitWriter` — accumulates bits LSB-first and produces a backward bit
  stream with a sentinel byte. Used for writing the FSE sequence bitstream.
- `RevBitReader` — initialises from the end of a byte slice, reads bits
  LEFT-ALIGNED from a 64-bit shift register going backward toward byte 0.
  Fixed sentinel-detection to find the highest set bit (not the lowest).
- `buildDecodeTable` and `buildEncodeTable` — two-pass spread algorithm
  matching the reference implementation's deterministic symbol ordering.
- `llToCode` / `mlToCode` — linear-scan LL/ML code number lookup.
- Raw_Literals encoding with 1/2/3-byte size headers per RFC 8878 §3.1.1.2.1.
- Sequence count encoding per RFC 8878 §3.1.1.1.2 (1, 2, or 3 bytes).
- Block selection heuristic: RLE → Compressed (LZ77 + FSE) → Raw fallback.
- Decompression bomb guard: output capped at 256 MB.
- 15 xunit tests covering empty input, single byte, all-256-bytes, RLE block,
  English prose, pseudo-random, multi-block (300 KB), repeat-offset patterns,
  determinism, RevBitWriter/Reader round-trip, FSE decode table coverage,
  and LL/ML code lookup correctness.

### Notes

- Depends on `CodingAdventures.Lzss.FSharp` for LZ77 token generation
  (32 KB window, max match 255, min match 3).
- Only Predefined FSE modes are supported (mode byte must be 0x00).
- Only Raw_Literals (type=0) are produced by the encoder; the decoder
  rejects Huffman-coded literals with a clear error message.
