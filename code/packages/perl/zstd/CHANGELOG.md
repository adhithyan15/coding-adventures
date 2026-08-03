# Changelog — CodingAdventures::Zstd

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.2] — 2026-08-03

### Fixed

- **Sequences-section FSE codec conformance (RFC 8878 §3.1.1.3.2.1.2).**
  A repo-wide audit (following the same bug class discovered and fixed in
  `java/zstd` / `kotlin/zstd`, and independently confirmed in `rust/zstd`;
  see `lessons.md` Lesson 96) found the SAME three compounding bugs present
  in this Perl port. All three are self-cancelling under our own
  encoder/decoder round-trip (both sides agreed on the same wrong
  convention), so every one of the existing 26 unit/round-trip tests passed
  despite producing non-conformant `.zst` frames — only real interop against
  the actual `zstd` CLI (the new TC-9 test, see below) could catch this.

  1. **FSE table-spread algorithm used a fabricated two-pass split.**
     `_build_decode_table` and `_build_encode_sym` spread symbols into table
     slots using "first all symbols with count>1, then all symbols with
     count==1" (both in ascending symbol order) — a plausible-looking but
     invented convention. The real algorithm
     (`FSE_buildDTable_internal`'s low-probability branch, verified against
     the reference C source at `github.com/facebook/zstd`) is a SINGLE pass
     over symbols `0..maxSymbolValue`, placing each symbol's full count
     immediately when encountered.

  2. **Per-sequence field order was wrong in two ways.** A conformant
     decoder must PEEK all three symbols (LL, ML, OF) from the CURRENT FSE
     states first — a bare table lookup that consumes no bits — THEN read
     extra bits in order OF, ML, LL, THEN (see bug 3) update the three
     states in order LL, ML, OF. The prior implementation combined
     peek-and-update into a single `_fse_decode_sym` call per field and got
     both the extras/updates relative order and the OF/ML sub-order wrong.
     `_fse_decode_sym` has been removed; `_decompress_block` now inlines the
     three-step peek / read-extras / update sequence directly.

  3. **The state-transition "update" is skipped for the LAST sequence in a
     block.** There is no "next" sequence to prepare a state for, so a
     conformant decoder never performs that read for the final sequence.
     Symmetrically, the encoder's first-processed sequence in its reverse
     loop (semantically the LAST real sequence) cannot derive its starting
     state via a normal bit-flushing transition — there is no corresponding
     decode-side bit-read to consume it. It is now computed directly via
     the new `_fse_init_state` helper (mirrors real zstd's
     `FSE_initCState2`; writes no bits at all). The prior implementation
     always flushed a transition for every sequence uniformly, writing bits
     a real decoder would never read, shifting the bit-alignment of
     everything that followed.

  Verified against the real `zstd` CLI (both directions — compress with
  ours / decompress with `zstd -d`, and compress with `zstd` / decompress
  with ours, including a real `Content_Checksum_Flag=1` frame since `zstd`
  enables checksums by default) on prose, a high-sequence-count (>128,
  crossing the 2-byte `Number_of_Sequences` boundary) input, and the
  minimal Lesson-96 repro (`"ababababab" x 3`).

- **Frame Header Descriptor `Content_Checksum_Flag` doc/comment fix
  (RFC 8878 §3.1.1.1).** The `compress()` comment (and the shared
  `code/specs/CMP07-zstd.md` spec) documented `Content_Checksum_Flag` as
  FHD bit 4; it is actually bit 2 (bit 4 is `Unused_bit`). Verified
  empirically: `zstd -c file` emits FHD `0x64`; `zstd -c --no-check file`
  emits FHD `0x60` (4 bytes shorter) — the differing bit is bit 2. This was
  a documentation-only inaccuracy in this package: `decompress()` never
  parsed any checksum-flag bit at all (checksummed trailers were already
  silently skipped by not being read), and the encoder always writes
  checksum-off `0xE0`, where both bit 2 and bit 4 are 0 — so no functional
  behavior changed. See `lessons.md` Lesson 95.

### Added

- **New `TC-9: real zstd CLI interop` test** (`t/zstd.t`) — the spec's
  actual TC-9 (`code/specs/CMP07-zstd.md` §Test Cases), which this package
  had never implemented; a differently-scoped "bad magic" test had been
  using the `TC-9` label instead (renamed to `ERR-1: bad magic dies` to
  free up the correct name). Compresses with `compress()` and decompresses
  with the real `zstd -d` CLI, and the reverse, via list-form `system()`
  calls against temp files (never a shell, so binary data is never mangled
  by shell interpretation). Skips (does not fail) when `zstd` is not on
  `PATH`.
- **New `RT-11: CLI interop, high sequence count` test** — regression
  coverage for the `Number_of_Sequences` 1-byte/2-byte encoding boundary
  (128+ sequences in one block) against the real `zstd` CLI.

## [0.1.1] — 2026-04-27

### Fixed

- **`decompress` now rejects inputs larger than 64 MB before calling
  `unpack('C*', ...)`.**  `unpack` converts every byte into a full Perl scalar
  (~56 bytes on 64-bit builds), so an oversized compressed input would amplify
  memory by ~56× before any frame-header check could fire.  The guard fires on
  the raw byte count alone: `die "input too large" if length($data) > 64 MB`.
- New `SEC-1` test verifies that a 65 MB input is rejected with a "too large"
  error before any frame parsing occurs.

## [0.1.0] — 2026-04-25

### Added

- Initial implementation of ZStd (RFC 8878) lossless compression and
  decompression in pure Perl.
- `compress($data)` — encodes a binary string to a valid ZStd frame.
  Supports Raw, RLE, and LZ77+FSE compressed blocks.
- `decompress($data)` — decodes any ZStd frame produced by this module
  (Predefined FSE mode, Raw_Literals).
- FSE (Finite State Entropy) encode and decode tables built from the
  RFC 8878 Appendix B predefined distributions for LL, ML, and OF.
- `RevBitWriter` / `RevBitReader` — backward bit-stream implementation
  matching the ZStd sequence section wire format.
- `_build_decode_table` and `_build_encode_sym` — full FSE table
  construction algorithms (spread + phase-3 nb/base assignment).
- Literals section: Raw_Literals encoding and decoding with 1/2/3-byte
  variable-length headers.
- Sequences section: FSE-encoded LL/OF/ML triples with predefined mode.
- Multi-block support: inputs larger than 128 KB are split into separate
  blocks automatically.
- Test suite (`t/zstd.t`) with 19 subtests covering:
  - TC-1 through TC-9 as specified (empty, single byte, all 256 values,
    RLE, prose ratio, LCG random, 200 KB multi-block, 300 KB repetitive,
    bad magic error)
  - RT-1 through RT-10 additional round-trip tests
  - UNIT-1 through UNIT-6 internal helper tests

### Implementation Notes

- Uses `CodingAdventures::LZSS` for LZ77 match-finding (32 KB window,
  max match 255, min match 3).
- The `RevBitReader` shift register is 64-bit; arithmetic uses
  `& 0xFFFFFFFFFFFFFFFF` masks to stay within Perl's native UV range on
  64-bit systems.
- Offset encoding adds +3 to avoid the ZStd reserved repeat-offset values
  (1, 2, 3), matching the Rust reference implementation.
