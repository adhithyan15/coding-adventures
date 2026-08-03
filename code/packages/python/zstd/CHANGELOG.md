# Changelog

All notable changes to `coding-adventures-zstd` will be documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.1] — 2026-08-03

### Fixed

RFC 8878 conformance bugs in the sequences-section FSE codec, found and fixed
as part of a repo-wide audit that started with the `java/zstd` rescue (#9780)
and `rust/zstd` fix (#9774): our compressed output was being rejected by the
real `zstd` CLI ("Data corruption detected") even though every pre-existing
round-trip test in this package passed, because the encoder and decoder were
both wrong in the SAME internally-self-consistent way. See lessons.md
Lesson 95/96.

- **FSE table-spread algorithm** (`_build_decode_table`, `_build_encode_sym`):
  replaced a fabricated two-pass split (all count>1 symbols first, then
  count==1 symbols) with the real single-pass algorithm — one pass over
  symbols `0..maxSymbolValue`, placing each symbol's full count immediately
  when encountered (`FSE_buildDTable_internal`'s actual behaviour, verified
  against the reference C source `fse_decompress.c`).

- **Per-sequence field order** (`_encode_sequences_section`,
  `_decompress_block`): the decoder now correctly PEEKS all three FSE
  symbols (LL/ML/OF) from the current states first (a free table lookup —
  no bits consumed), THEN reads extra bits in order OF, ML, LL, THEN
  updates states in order LL, ML, OF — and that state update is now
  correctly SKIPPED for a block's last sequence, matching the reference
  decoder (`ZSTD_decodeSequence`). The encoder mirrors this: its
  first-processed (semantically last) sequence now gets a direct-formula
  state init (new `_fse_init_state`, mirroring `FSE_initCState2`) instead
  of a normal bit-flushing transition.

- **Number_of_Sequences 2-byte wire encoding** (`_encode_seq_count`,
  `_decode_seq_count`): removed a spurious `-0x80`/`+0x80` additive offset
  that isn't part of the real format. The correct encoding (RFC 8878
  §3.1.1.3.1) is a direct big-endian split: `byte0 = 0x80 | (count >> 8)`,
  `byte1 = count & 0xFF`, with no offset. This bug hid behind the 1-byte
  encoding for any input producing fewer than 128 LZ77 sequences — it only
  surfaced (as CLI-rejected output) on inputs large/repetitive enough to
  cross that boundary, which is exactly why it wasn't caught by the
  pre-existing test suite.

- **Negative/underflowing match offset now rejected cleanly** instead of
  crashing: decoding a Repeat_Offset (R1/R2/R3) shortcut code — a feature
  outside this simplified codec's supported subset, but one the real `zstd`
  CLI uses by default — used to compute a negative "offset" that bypassed
  the existing `offset == 0` bounds check and caused an uncaught
  `IndexError` inside the match-copy loop. `_decompress_block` now checks
  `of_raw < 3` up front and raises a clear `ValueError` explaining the
  unsupported feature, matching this package's existing "reject what we
  don't support, don't misinterpret it" posture for e.g. non-Raw literals
  and non-Predefined FSE modes.

- **FHD Content_Checksum_Flag bit corrected in documentation and decode
  logic**: it's bit 2 (RFC 8878 §3.1.1.1), not bit 4 as an earlier revision
  of the `compress()` comment claimed. This encoder still always emits 0
  for both bits (no checksum), so the mislabelling was functionally
  harmless on our own output — but `decompress()` now reads the *correct*
  bit and, when set (as it is by default on frames from the real `zstd`
  CLI), correctly skips the trailing 4-byte content checksum rather than
  ignoring it as unaccounted trailing data. `decompress()` also now rejects
  any unexpected trailing bytes after the last block (lessons.md Lesson 94),
  which the checksum-skip logic must run before to avoid a false positive.

### Added

- **Real `zstd` CLI interoperability tests** (`TestCliInterop`, fulfilling
  spec TC-9 from `code/specs/CMP07-zstd.md`, which was previously missing
  from this package's suite): compress with this package and decompress
  with the real `zstd -d` binary, and the reverse, across empty/small/
  binary/multi-block/high-sequence-count payloads. Gracefully skipped (not
  failed) when `zstd` isn't on `PATH`. These are what actually caught the
  bugs above — every purely-internal round-trip test in this file continued
  to pass throughout, because the pre-fix encoder and decoder were wrong in
  the same self-consistent way.
- Exact wire-byte assertions for the corrected `_encode_seq_count` 2-byte
  form (`TestSeqCount::test_wire_bytes_exact`).
- `_fse_init_state`: mirrors real zstd's `FSE_initCState2` — computes an
  FSE encoder's starting state directly from a symbol with no bits flushed,
  used for the first-processed (last real) sequence in
  `_encode_sequences_section`.

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
