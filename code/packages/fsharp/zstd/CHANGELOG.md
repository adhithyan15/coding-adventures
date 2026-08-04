# Changelog

All notable changes to this package will be documented in this file.

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
