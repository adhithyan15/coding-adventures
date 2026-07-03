# Changelog

## 0.4.0 — 2026-07-02

### Added — dynamic-Huffman encoding (BTYPE=10)

- **`compress` now emits dynamic-Huffman blocks when they are smaller.** For each
  input it costs, in exact bits, both a fixed-table encoding (BTYPE=01) and a
  data-adapted **dynamic** encoding (BTYPE=10, code lengths transmitted inline)
  of the same LZSS token stream, then emits the cheaper one as a single final
  block. On text and other skewed data this is a large win (the fixed tables
  spend 8–9 bits per literal; an adapted tree gives common bytes 2–4-bit codes).
  On tiny or near-incompressible inputs, where the dynamic code-length header
  outweighs its savings, `compress` falls back to fixed — so the output is
  **never larger** than the previous fixed-only encoding. `compress`'s signature
  and its standard-RFC-1951 output guarantee are unchanged; the empty-input and
  correctness behaviour are preserved.

  Measured on ~2.4 KB of repeated English prose: **2364 → 174 bytes** with a
  dynamic block, versus **235 bytes** for a fixed-only encoding of the same
  tokens (≈ 26 % smaller than fixed, 13.6× smaller than the input).

- **Length-limited Huffman via the package-merge algorithm** (Larmore–Hirschberg,
  *A fast algorithm for optimal length-limited Huffman codes*, JACM 1990). RFC
  1951 caps codes at 15 bits (literal/length and distance) and 7 bits
  (code-length alphabet); an optimal Huffman tree over up to 286 symbols can
  exceed 15 bits on skewed frequencies. Package-merge produces the *optimal* code
  subject to `max_len`, and provably always yields a valid prefix code (Kraft sum
  ≤ 1) whenever the alphabet fits in `2^max_len` symbols — which all three of our
  alphabets do (286 ≤ 2¹⁵, 30 ≤ 2¹⁵, 19 ≤ 2⁷). The implementation asserts both
  invariants (`len ≤ max_len` and Kraft ≤ 1) so a malformed tree can never reach
  the wire.

- **Code-length RLE** (CL symbols 0–18: 16 = repeat-previous 3–6, 17 = zero-run
  3–10, 18 = zero-run 11–138) mirroring `inflate`'s `decode_code_lengths`, with
  the CL tree itself length-limited to 7 bits, so the transmitted header is
  compact.

- **Edge cases handled per RFC 1951 §3.2.7:** a block with no matches still emits
  a valid `HDIST` with one dummy distance code of length 1; single-symbol
  alphabets receive a valid 1-bit code.

### Verification

- New tests: package-merge cap/Kraft invariants under pathological skew
  (including Fibonacci weights), skewed-distribution round-trips, dynamic-wins
  and fixed-fallback size assertions, a broad round-trip battery (empty, every
  byte value, all-256, highly repetitive, pseudo-random, multi-KB text), and the
  no-distance-code dummy path. All existing round-trip, `inflate`, and
  `zlib_decompress` tests continue to pass (30 tests total).
- **Cross-checked with Python `zlib` in both directions:** Python
  `zlib.decompress(bytes.fromhex(<our output>), -15)` reads our fixed *and*
  dynamic streams byte-for-byte across a 2000-input randomized fuzz (heavily
  skewed, uniform-random, tiny-alphabet, periodic, and run-heavy inputs); our
  `inflate` reads Python's dynamic output (existing `inflate_dynamic_huffman` and
  `inflate_full_window_real_stream` tests).
- Downstream `png` (15 tests) and `zip` (24 + 5 tests) — which depend on
  `deflate` via `zlib_compress`/`inflate` — are unaffected and pass.

## 0.3.0 — 2026-07-02

### Changed (breaking wire format)

- **`compress` now emits a standard RFC 1951 raw DEFLATE stream** instead of the previous non-standard custom container. The old format prepended a 4-byte original-length field and an explicit `(symbol, code_length)` table to a private bit stream — bytes no other tool could read. `compress` now produces a single **fixed-Huffman block** (BTYPE=01) using the pre-defined RFC 1951 §3.2.6 code tables, so the output is decodable by any conforming inflater (`inflate` here, `zlib`, `gzip`, `unzip`, browsers). Verified in both directions: our `inflate` reads Python-`zlib` output, and Python-`zlib` reads our `compress` output.
- **`decompress` is now an alias for `inflate`.** The custom-format decoder (with its unchecked indexing, back-reference-offset underflow, and no output cap) is deleted; `decompress` forwards to the hardened, bomb-capped `inflate`, which reads stored/fixed/dynamic blocks. No caller outside this crate used `compress`/`decompress`.
- LZSS window widened from 4096 to the full RFC 1951 **32768** bytes for better matching.

### Removed

- The custom-format machinery: `build_canonical_codes`, `reverse_code_map`, `unpack_bits`, `BitBuilder::write_bit_string`, and the `HuffmanTree`-based encoder.
- The **`huffman-tree` path dependency** — no longer used now that `compress` uses the fixed (pre-defined) code tables.

### Notes

- Fixed-Huffman is standard and correct for every input but not the smallest encoding. Dynamic-Huffman **encoding** (BTYPE=10, better ratios) is a future optimisation and needs length-limited Huffman trees; the decoder already reads dynamic blocks.

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
