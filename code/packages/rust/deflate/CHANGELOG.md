# Changelog

## 0.2.1 — 2026-07-02

### Fixed

- `inflate` (and therefore `zlib_decompress`) now decode the **full RFC 1951 symbol space**, not just the 4 KB-window subset our own encoder happens to emit. Two standard symbols were missing:
  - **LL length symbol 285** (length 258 exactly, no extra bits) — added to `LENGTH_TABLE`.
  - **Distance codes 24–29** (back-references from 4097 up to 32768, the full 32 KB window) — added to `DIST_TABLE`.
  Real-world producers — Microsoft Office writing OOXML (`.xlsx`/`.docx`/`.pptx`), `zlib`, `gzip`, Python `zipfile`, Java `jar` — routinely use these symbols, so any stream from them previously failed with `inflate: invalid length symbol 285` or `inflate: invalid distance symbol 24`. The decoder was only ever complete for our own 4 KB-window output.
- Added `inflate_full_window_real_stream` regression test built from a Python-`zlib` raw DEFLATE fixture that deliberately exercises symbol 285 (a 600-byte run) and far distance codes (a repeat ~6000 bytes back).

### Security

- `inflate` now caps decompressed output at 256 MB (`MAX_INFLATE_OUTPUT`) across all three block types, guarding against decompression bombs (tiny hostile inputs that expand to gigabytes and exhaust memory). Enforced in the literal, back-reference, and stored-block paths. This matches the guard the `zip` crate's former inline decompressor had, and now protects every `inflate` consumer.

## 0.2.0 — 2026-05-24

### Added

- `inflate(data: &[u8]) -> Result<Vec<u8>, String>` — RFC 1951 raw DEFLATE inflate supporting stored (BTYPE=00), fixed-Huffman (BTYPE=01), and dynamic-Huffman (BTYPE=10) blocks. Implemented with a `BitReader` accumulator struct and a canonical Huffman decoder keyed by `(code_value, code_len)` pairs.
- `zlib_decompress(data: &[u8]) -> Result<Vec<u8>, String>` — RFC 1950 zlib envelope decode; strips CMF/FLG header, delegates to `inflate`, verifies Adler-32 checksum, and returns the original bytes.
- `BitReader` struct: LSB-first 64-bit accumulator with `read_bits`, `align_to_byte`, `read_byte`, `read_u16_le`.
- `build_huffman_decoder` helper: canonical Huffman table from a code-length array, returning `HashMap<(code, len), symbol>`.
- `decode_symbol` function: MSB-first accumulation over `BitReader` for Huffman decoding.
- `decode_block` function: shared literal/length/back-reference decode loop for both fixed and dynamic blocks, reusing `LENGTH_TABLE` and `DIST_TABLE`.
- `decode_code_lengths` function: CL meta-tree decoding with symbols 0–18 (repeat and zero-run extensions).
- `copy_back_ref` function: overlapping back-reference copy (supports run-length encoding when dist < length).
- Eight new tests: `inflate_stored_block`, `inflate_stored_two_blocks`, `zlib_roundtrip`, `inflate_fixed_huffman`, `inflate_dynamic_huffman`, `zlib_decompress_bad_header`, `zlib_decompress_fdict`, `zlib_decompress_too_short`, `zlib_decompress_bad_adler`.

## 0.1.0 — 2026-04-12

### Changed

- Complete rewrite to implement CMP05 wire format using LZSS (CMP02) + dual canonical Huffman trees (DT27).

### Added

- `compress(data: &[u8]) -> Result<Vec<u8>, String>` — two-pass DEFLATE encoder.
- `decompress(data: &[u8]) -> Result<Vec<u8>, String>` — CMP05 wire-format decoder.
- Length code table (symbols 257–284) with extra bits.
- Distance code table (codes 0–23, for offsets 1–4096) with extra bits.
- LSB-first bit stream packing via `BitBuilder`.
- Dependencies on `lzss` (CMP02) and `huffman-tree` (DT27).
