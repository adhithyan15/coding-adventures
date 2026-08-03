# Changelog

All notable changes to this package will be documented in this file.

## [0.1.1] - 2026-08-03

### Fixed

- **Sequences-section FSE codec was not RFC 8878 conformant**, despite passing
  every internal round-trip test. This package was added independently
  (months after `code/packages/rust/zstd`), but audited alongside a
  repo-wide sweep (see `gh pr diff 9780`, `code/packages/java/zstd`) after the
  same bug pattern was confirmed in the Rust reference implementation. Three
  compounding bugs, each individually self-cancelling in a same-codebase
  round-trip test (encoder and decoder always agreed with each other) but
  fatal against the real `zstd` CLI:
  1. `BuildDecodeTable`/`BuildEncodeTables` spread FSE table symbols using a
     fabricated two-pass split (all `count>1` symbols first, then all
     `count==1` symbols). The real algorithm (verified against
     `FSE_buildDTable_internal` in `github.com/facebook/zstd`'s
     `fse_decompress.c`) is a single pass over symbols in ascending order,
     placing each symbol's full count immediately.
  2. Per-sequence field order was wrong: a conformant decoder peeks all
     three symbols (LL/OF/ML) from the current states first (zero bits),
     then reads extra value bits in order OF, ML, LL, then updates FSE
     states in order LL, ML, OF. This package combined peek-and-update into
     one step and read extra bits in the wrong order.
  3. The FSE state-transition update was performed for every sequence
     uniformly, including the last one. A real decoder never updates state
     after the final sequence in a block (there is no "next" sequence to
     prepare a state for); the encoder must mirror this by computing the
     first-processed (semantically last) sequence's starting state directly
     via the `FSE_initCState2` formula, writing zero bits, instead of a
     normal bit-flushing transition. Added `FseInitState` alongside the
     renamed `FseEncodeTransition` (previously the single overloaded
     `EncodeSymbol`) to implement this.
- **Frame Header Descriptor `Content_Checksum_Flag` was read from bit 4
  instead of bit 2**, and the reserved-bit rejection mask (`0x0C`) incorrectly
  included bit 2 — meaning any real-world frame with a trailing content
  checksum (the default for the `zstd` CLI) was rejected outright as having
  "reserved frame-header bits set" before checksum parsing was ever reached.
  Verified empirically: `zstd -c` emits FHD `0x64`; `zstd -c --no-check`
  emits FHD `0x60`. RFC 8878 §3.1.1.1: bit 4 is `Unused_bit` (must be
  ignored, not rejected), bit 2 is `Content_Checksum_Flag`.
- Added a real TC-9 CLI-interoperability test (`Tc9CliInterop`,
  `RepeatingPatternCliInterop`) that shells out to the actual `zstd` binary
  in both directions (compress-here/decompress-there and the reverse).
  Confirmed the pre-fix code failed `Tc9CliInterop` with the real CLI's
  "Data corruption detected" error; the fix passes cleanly. This is the only
  kind of test that can catch this bug class — a same-codebase round-trip
  test cannot, since encoder and decoder always agreed with each other on
  the same wrong convention.

## [0.1.0] - 2026-07-18

### Added

- Pure C# CMP07 Zstandard educational codec with raw, RLE, and compressed blocks
- Raw literal sections and predefined FSE tables for literal-length, match-length, and offset codes
- One-shot `Compress` and `Decompress` helpers backed by the native C# LZSS package
- xUnit coverage for multi-block frames, header variants, malformed input, compression ratios, and deterministic output
