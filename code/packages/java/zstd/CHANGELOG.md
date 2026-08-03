# Changelog — java/zstd

All notable changes to this package will be documented in this file.

## [0.1.1] — 2026-08-03

Rescued from an orphaned branch (`worktree-feat+zstd-and-catchups`, never PR'd,
~3 months stale) and verified/fixed against current `main` and the current
Java/Gradle toolchain (Java 21, Gradle 8.14.4).

### Fixed

- **`lzss` API drift**: the stale branch was written against an older
  `com.codingadventures.lzss.Lzss`/`LzssToken` API; the current `lzss`
  package exposes `LZSS` (nested `Token`/`Literal`/`Match` types). Updated
  all call sites.
- **Sequence-count wire encoding had a reversed byte order** (2-byte form):
  the marker/high byte must be transmitted first on the wire
  (`(count >> 8) | 0x80` before `count & 0xFF`), matching the Go/Rust
  references and RFC 8878 §3.1.1.3.1 — an earlier revision wrote the low byte
  first. Internally self-consistent (round-tripped against itself) but not
  RFC-conformant; only caught by real interop testing. See lessons.md
  Lesson 250 / Lesson 95.
- **Frame Header Descriptor `Content_Checksum_Flag` bit position**: fixed
  from (incorrectly documented) bit 4 to the correct bit 2, verified
  empirically against the real `zstd` CLI (`zstd -c` vs `zstd -c --no-check`)
  and RFC 8878 §3.1.1.1. `decompress()` now also correctly skips a trailing
  4-byte content checksum when present, rather than leaving it unhandled.
  See lessons.md Lesson 95.
- **Three compounding bugs in the sequences-section FSE codec**, found via
  real `zstd` CLI interop testing (all were invisible to internal round-trip
  tests, since encoder and decoder were wrong in the same self-consistent
  way) — see lessons.md Lesson 96 for full detail:
  1. `buildDecodeTable`/`buildEncodeTable` used a fabricated two-pass symbol
     spread ("count>1 first, then count==1") instead of the real single pass
     over symbols in ascending order.
  2. Per-sequence field order was wrong: real order peeks all 3 FSE symbols
     first (no bits consumed), then reads extra bits in order
     Offset,MatchLength,LiteralsLength, then updates FSE states in order
     LiteralsLength,MatchLength,Offset.
  3. The FSE state-transition update is skipped for the LAST sequence in a
     block; added `fseInitState()` (mirrors the reference `FSE_initCState2`)
     to initialise the encoder's starting state directly from a symbol with
     no bits flushed, matching the decoder never reading an update after the
     last sequence.
- Confirmed this bug class also reproduces against the Rust reference
  implementation (`code/packages/rust/zstd`) with the same minimal repro —
  not a Java-specific porting mistake. Flagged as a follow-up task for the
  other language ports (out of scope for this PR).

### Added

- **Block-size cap enforcement** during decompression: rejects any block
  whose 21-bit `Block_Size` field exceeds 128 KB (the educational
  implementation's own maximum), rather than trusting an attacker-controlled
  frame.
- **Trailing-data rejection**: `decompress()` now errors if bytes remain
  after the last block (plus checksum, if present) instead of silently
  ignoring them (lessons.md Lesson 94).
- **TC-9 (CLI interoperability)**: two new JUnit tests exercise the real
  `zstd` CLI binary in both directions (compress-here/decompress-there and
  vice versa), gracefully skipped via `Assumptions.assumeTrue` if `zstd`
  isn't on `PATH`. This is the test that caught all of the above bugs — no
  existing `zstd` port in this repo had CLI interop coverage before this.
- **TC-10 (hand-built minimal wire-format frame)**: decodes a manually
  constructed 14-byte frame, independent of this package's own encoder.
- Exact wire-byte assertions for the sequence-count 2-byte form
  (`testSeqCountWireBytesExact`), including values whose low byte is < 128
  — the specific case a reversed-byte-order bug would corrupt silently.
- 6 additional JUnit tests (23 total, up from 16 originally staged on the
  stale branch): the 2 CLI interop tests above, a high-sequence-count CLI
  interop regression test, the wire-byte-exact seq-count test, TC-10, and a
  renamed/clarified `rtDeterministicOutput` (previously mislabeled `TC-9`).

## [0.1.0] — 2026-04-24

### Added

- Initial implementation of `Zstd.compress(byte[])` and `Zstd.decompress(byte[])`.
- FSE (Finite State Entropy) encode/decode tables built from RFC 8878 Appendix B
  predefined distributions (LL, ML, OF).
- `RevBitWriter` and `RevBitReader` for the ZStd backward bit-stream codec.
- Raw_Literals section encoding/decoding with 1/2/3-byte variable-length header.
- Sequence section encoding/decoding with predefined FSE modes.
- Block types: Raw (00), RLE (01), Compressed (10).
- Multi-block support for inputs larger than 128 KB.
- 16 JUnit 5 unit tests covering empty input, single bytes, all-byte values,
  RLE detection, prose compression ratio, pseudo-random data, multi-block frames,
  repeat-offset patterns, determinism, and internal codec round-trips.
- `BUILD`, `BUILD_windows`, `required_capabilities.json`, `README.md`.
- Depends on `com.codingadventures:lzss` for LZ77 tokenisation.
