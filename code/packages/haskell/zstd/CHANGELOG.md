# Changelog

## 0.1.0 — Initial release

- Complete Haskell implementation of ZStd (CMP07) compression/decompression.
- FSE encode/decode tables built from predefined RFC 8878 Appendix B distributions
  (LL_NORM, ML_NORM, OF_NORM). All modes use Predefined_Mode (0x00 symbol
  compression modes byte).
- Raw_Literals section encoding (type=0, no Huffman). Supports 1-byte, 2-byte,
  and 3-byte size headers for literal counts up to 1 MB.
- Sequence encoding/decoding via a backward bit-stream (RevBitWriter / RevBitReader)
  with sentinel-bit initialisation matching the reference implementation.
- Frame format: 4-byte magic + FHD byte (0xE0: 8-byte FCS, Single_Segment=1) +
  8-byte Frame_Content_Size + blocks.
- Block type selection: RLE (all-identical bytes), Compressed (LZ77+FSE, when
  smaller), Raw (fallback).
- Multi-block support: inputs > 128 KB are split across sequential blocks.
- 14 test cases covering empty input, single byte, all 256 byte values, RLE,
  prose, pseudo-random bytes, 300 KB multi-block, repeat-offset patterns,
  determinism, repeated patterns, hello world, zeros, 0xFF, and manual
  wire-format decoding.
