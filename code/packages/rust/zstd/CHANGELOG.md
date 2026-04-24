# Changelog — zstd

## 0.1.0 — 2026-04-24

### Added

- Initial implementation of the Zstandard compression algorithm (RFC 8878, CMP07).
- `compress(data: &[u8]) -> Vec<u8>`: encodes any byte slice into a valid ZStd frame.
- `decompress(data: &[u8]) -> Result<Vec<u8>, String>`: decodes any single-segment ZStd frame.
- Full ZStd frame layout: magic number, FHD byte, 8-byte FCS, blocks.
- Three block types:
  - **Raw** blocks for incompressible data.
  - **RLE** blocks for single-value runs (e.g., 1024 'A' bytes → 17 bytes total).
  - **Compressed** blocks using LZ77 back-references + FSE sequence coding.
- Predefined FSE tables for Literal Lengths, Match Lengths, and Offsets
  (from RFC 8878 Appendix B), so frames require no per-block table description.
- `RevBitWriter` / `RevBitReader`: backward bit-stream codec (last-written bits
  read first), matching the ZStd sequence bitstream convention.
- Raw_Literals section encoding/decoding with 1-, 2-, and 3-byte headers.
- Multi-block support for inputs larger than 128 KB.
- Manual wire-format test verifying the decoder against a hand-built raw-block frame.
- 25 unit tests + 3 doctests; all pass.
- Literate-programming comments throughout explaining ZStd internals from first
  principles.

### Implementation notes

- LZ77 token generation is delegated to the `lzss` crate (CMP02) via
  `lzss::encode(block, 32768, 255, 3)` — 32 KB window, max match 255, min match 3.
- FSE encode table uses index-order (not fill-order) position assignment to
  maintain the encode/decode symmetry invariant.
- Sequence FSE symbols are written in ML→OF→LL order so the backward bit-stream
  delivers them in LL→OF→ML decode order.
- Raw_Literals uses size_format 00 (1-byte), 01 (2-byte), or 11 (3-byte) per
  the spec; size_format 10 is also accepted on decode as equivalent to 00.
