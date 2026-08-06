# Changelog

All notable changes to this package will be documented in this file.

## [0.1.2] - 2026-08-05

Audited against a second, independent RFC 8878 conformance gap — this one a
decode-only FEATURE GAP rather than the FSE-codec class fixed in 0.1.1 —
first found and fixed in `code/packages/c/zstd` (PR #9941, `lessons.md`
Lesson 98). This package inherited the identical gap.

### Fixed

- **`DecompressBlock` never implemented Repeated-Offset (R1/R2/R3) sequence
  decoding (RFC 8878 §3.1.1.3.2.1.1).** An `Offset_Value` of 1, 2, or 3 is a
  reference into a three-entry offset history (`Repeated_Offset1/2/3`,
  frame-scoped, defaulting to 1/4/8 for the first block and updated after
  every Compressed block's sequences), not a literal `Offset_Value - 3`
  computation. `DecompressBlock` computed `matchOffset = rawOffset - 3`
  unconditionally, so any `rawOffset` of 1-3 underflowed to a bogus huge
  offset — rejected by the existing offset-bounds check as "malformed" even
  though the frame was perfectly valid, just encoded via a mechanism the
  decoder didn't understand.
- Why this package's own round-trip tests never caught it: `EncodeSequences`
  is, by design, incapable of emitting `Offset_Value <= 3` (the minimum
  LZSS match offset is 1, so `rawOffset = offset + 3 >= 4` always) — the
  "no repeat-offset shortcuts" educational simplification is entirely an
  ENCODER-side choice. But the real `zstd` CLI's encoder uses repeat offsets
  constantly (one of its principal entropy wins, especially for
  periodic/repetitive data), so this package's decoder systematically failed
  to decode a meaningful fraction of real-world `.zst` files. Even the
  existing TC-9 CLI-interop tests (added in 0.1.1) never exercised this path
  — their fixed prose corpus never happened to produce a real-`zstd`-encoded
  sequence with `Offset_Value <= 3`.
- Fixed by threading the three `Repeated_Offset` registers (frame-scoped, as
  `byref<int>` parameters) from `Decompress` through every Compressed
  block's `DecompressBlock` call, and implementing the full
  peek-then-select-then-rotate mechanism per RFC 8878 §3.1.1.3.2.1.1 —
  including the "when `Literals_Length` is 0, repeated offsets are shifted
  by 1" special case (using the peeked, not-yet-extra-bit-read literal-length
  code, which is sufficient to know whether the eventual literal length is
  zero). Cross-checked against both the RFC 8878 prose and the literal
  reference C source (`ZSTD_decodeSequence` in `zstd_decompress_block.c`
  from github.com/facebook/zstd), per the Lesson-96/98 playbook of never
  re-deriving wire-format semantics from memory alone. The encoder is
  deliberately left unchanged — still never emits repeat-offset codes; this
  is a decode-only fix.

### Added

- Two new TC-9 regression tests: `4713` bytes of a single repeated byte
  (real `zstd` compresses this to one Compressed block whose one sequence is
  2 literal bytes + a match with `Offset_Value=1`, i.e. "reuse
  `Repeated_Offset1`" — an unmistakable RLE-via-repeat-offset pattern that
  reproduced the pre-fix bug directly), and a periodic 6-byte-cycle pattern
  as an independent repro not dependent on real zstd's constant-byte
  heuristic specifically.
- Ad hoc (not committed) verification: a 42-case fuzz sweep against the real
  `zstd` CLI (constant, periodic at several cycle lengths, ramp, random, and
  prose patterns, at sizes from 16 bytes to 20 KB) — all round-tripped
  byte-exact after the fix.

## [0.1.1] - 2026-08-03

Audited against a real RFC 8878 conformance bug confirmed present in
`code/packages/java/zstd`, `code/packages/kotlin/zstd`, and
`code/packages/rust/zstd` (fixed in java/zstd PR #9780, documented in
lessons.md Lesson 96). This package inherited the *same* bug — it was not,
as initially hoped, an independent-enough implementation (PR #8570) to have
avoided it.

### Fixed

- **Three compounding bugs in the sequences-section FSE codec**, found by
  auditing against the real zstd C reference source
  (`FSE_buildDTable_internal`, `ZSTD_decodeSequence`, `FSE_encodeSymbol`,
  `FSE_initCState2` from github.com/facebook/zstd) and confirmed with real
  `zstd` CLI interop testing — all three were invisible to every existing
  round-trip test in this package, since the encoder and decoder were wrong
  in the same self-consistent way:
  1. `BuildDecodeTable`/`BuildEncodeTables` used a fabricated two-pass symbol
     spread ("count>1 first, then count==1") instead of the real single pass
     over symbols 0..maxSymbolValue in ascending order, placing each
     symbol's full count immediately.
  2. Per-sequence field order was wrong in two ways (RFC 8878
     §3.1.1.3.2.1.2): a decoder must PEEK all three FSE symbols from the
     current state first (free — no bits consumed), THEN read extra bits in
     order Offset, MatchLength, LiteralsLength, THEN (see bug 3) update
     states in order LiteralsLength, MatchLength, Offset. The prior code
     combined peek-and-update into a single step per symbol and got both
     the extras/updates relative order and the Offset/MatchLength update
     sub-order wrong.
  3. The FSE state-transition update is skipped for the LAST sequence in a
     block (there's no "next" sequence to prepare a state for); added
     `InitEncodeState` (mirrors the reference `FSE_initCState2`) to
     initialise the encoder's starting state directly from a symbol with no
     bits flushed, matching the decoder never reading an update after the
     last sequence. The prior encoder always flushed a transition for every
     sequence, including the first one processed in its reverse encode
     loop, which has no incoming state to transition from.
- **Frame Header Descriptor `Content_Checksum_Flag` bit position**: fixed
  from bit 4 (`0x10`) to the correct bit 2 (`0x04`), and the reserved-bit
  validation narrowed from bits `[3:2]` (`0x0C`) to just bit 3 (`0x08`) —
  bit 4 is `Unused_bit` per RFC 8878 §3.1.1.1, not reserved and not the
  checksum flag. Verified empirically against the real `zstd` CLI: `zstd -c
  file` (checksum on by default) emits FHD byte `0x64`; `zstd -c
  --no-check file` emits `0x60` — the differing bit is bit 2. Under the old
  (wrong) reserved-bit mask, any real-world checksummed frame would have
  been rejected as having "reserved frame-header bits set" the moment a
  Lesson-94-style trailing-bytes check were added — this package already
  had that check, so the bug was live from day one for any frame produced
  by real `zstd` with its default checksum-on behaviour. See lessons.md
  Lesson 95.

### Added

- **TC-9 (CLI interoperability)**: three new xUnit tests exercise the real
  `zstd` CLI binary in both directions (compress-here/decompress-there and
  vice versa, plus a dedicated high-sequence-count regression that crosses
  the sequence-count wire encoding's 128-sequence 1-byte -> 2-byte
  boundary), gracefully skipped (no assertions run) if `zstd` isn't on
  `PATH`. This is the class of test that actually caught the bugs above —
  every existing round-trip-only test passed throughout, because they only
  ever checked this package's decoder against its own encoder.
- A companion test asserting FHD bit 4 (`Unused_bit`) is correctly ignored
  rather than mistaken for the checksum flag, from the opposite direction
  of the Lesson 95 regression test.

## [0.1.0] - 2026-07-18

### Added

- Pure F# CMP07 Zstandard educational codec with raw, RLE, and compressed blocks
- Raw literal sections and predefined FSE tables for literal-length, match-length, and offset codes
- One-shot `Compress` and `Decompress` helpers backed by the native F# LZSS package
- xUnit coverage for multi-block frames, header variants, malformed input, compression ratios, and deterministic output
