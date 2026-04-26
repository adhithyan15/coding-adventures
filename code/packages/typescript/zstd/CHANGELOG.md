# Changelog — @coding-adventures/zstd

All notable changes to this package will be documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.1] — 2026-04-26

### Fixed

- **`encodeSeqCount` / `decodeSeqCount` now use RFC 8878 §3.1.1.3.1 layout.**
  The 2-byte form previously wrote bytes in the wrong order
  (`[count & 0xFF, (count >> 8) | 0x80]`), placing the LOW byte first. The
  decoder reads byte0 to determine the form: for any count ≥ 128 whose low
  byte was < 128 (e.g. count=515 → byte0=0x03), the decoder mis-took the
  1-byte path and returned a tiny garbage count, mis-aligning every byte
  downstream — including the symbol-modes byte, which then often parsed
  with `LL_Mode != 0` and threw "unsupported FSE modes". Roughly half of all
  counts in the 2-byte range silently corrupted; the other half worked, so
  most existing tests passed.
- New regression test in TC-8 round-trips 200 KB of repetitive text, which
  reliably produces > 128 sequences in a single block.

## [0.1.0] — 2026-04-24

### Added

- Initial implementation of ZStd (RFC 8878) compression and decompression in TypeScript.
- `compress(data: Uint8Array): Uint8Array` — encodes a ZStd frame with:
  - 4-byte magic number `0xFD2FB528`
  - Frame Header Descriptor with 8-byte Frame Content Size
  - Multi-block splitting at 128 KB boundaries
  - RLE block detection (all bytes identical)
  - Compressed blocks via LZ77 (LZSS) + FSE sequence encoding
  - Raw block fallback when compression is not beneficial
- `decompress(data: Uint8Array): Uint8Array` — decodes a ZStd frame with:
  - Magic number validation
  - Full Frame Header Descriptor parsing (FCS, Single_Segment, Dict_ID flags)
  - Raw, RLE, and Compressed block support
  - Predefined FSE mode decoding (LL, ML, OF tables per RFC 8878 Appendix B)
  - 256 MB output-size guard against decompression bombs
- `RevBitWriter` — backward bit accumulator using BigInt register (64-bit safe)
- `RevBitReader` — backward bit reader with sentinel detection
- FSE decode table builder (`buildDecodeTable`) following the ZStd spreading algorithm
- FSE encode table builder (`buildEncodeTable`) with symmetric state transitions
- Predefined distributions: `LL_NORM`/`LL_ACC_LOG`, `ML_NORM`/`ML_ACC_LOG`, `OF_NORM`/`OF_ACC_LOG`
- LL/ML code tables from RFC 8878 §3.1.1.3 (36 and 53 entries respectively)
- Raw literals section encoding/decoding (1-byte, 2-byte, 3-byte headers)
- Sequence count encoding/decoding (1-byte, 2-byte, 3-byte formats)
- Comprehensive test suite (TC-1 through TC-9 plus additional round-trip and unit tests)
- Literate programming style comments throughout — explanations, diagrams, examples
