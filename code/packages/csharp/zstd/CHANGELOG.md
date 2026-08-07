# Changelog

All notable changes to this package will be documented in this file.

## [0.1.2] - 2026-08-05

### Fixed

- **The sequences decoder never implemented Repeated-Offset (R1/R2/R3)
  sequence decoding** (RFC 8878 §3.1.1.3.2.1.1). Real `zstd` encoders use
  repeat offsets constantly — they are one of the format's principal
  entropy wins, especially for periodic or highly repetitive data — but
  this package's own encoder, by design, never emits an offset code below
  2 (the minimum LZSS match offset is 1, so raw offset = offset + 3 is
  always >= 4). That made the "no repeat-offset shortcuts" simplification
  entirely an encoder-side choice, and meant `Decompress`'s own round trip
  with `Compress` — and every existing CLI-interop test's specific input —
  never exercised the repeat-offset decode path. `DecompressBlock`
  computed `matchOffset = rawOffset - 3` unconditionally, which underflows
  for any `Offset_Value <= 3` (a repeat-offset reference, not a literal
  offset), producing a bogus offset that the existing bounds check
  correctly rejected as malformed — even though the frame was valid and
  encoded using a mechanism the decoder simply didn't understand.
  - This gap was first found and fixed in the new `c/zstd` port (PR #9941)
    via ad hoc fuzzing against the real `zstd` CLI, and documented as
    `lessons.md` Lesson 98 there. This package inherited the identical gap
    (confirmed independently in this session: the pre-fix decoder failed
    real-CLI-compressed input of a single repeated byte, and of an
    alternating-literal/long-match pattern, both with
    `InvalidDataException: match offset exceeds decoded output`).
  - Fixed by threading three frame-scoped Repeated_Offset registers
    (`repeatOffset1`/`2`/`3`, defaulting to 1/4/8 per RFC 8878) through
    `Decompress`'s block loop into `DecompressBlock`, and implementing the
    full offset-code-to-actual-offset mapping: offset codes >= 2 are
    ordinary explicit offsets (`rawOffset - 3`, which also rotates the
    repeat-offset history); offset codes 0-1 (`rawOffset` in `{1, 2, 3}`)
    are repeat-offset references, resolved via a 4-way selector
    (`literalIsZero + rawOffset - 1`) that also implements the RFC's
    "when Literals_Length is 0, repeated offsets are shifted by 1" special
    case and the corresponding history rotation (no rotation, swap, or
    full 3-way rotate depending on which register was referenced).
    Algorithm cross-checked against both the RFC 8878 prose and the
    literal reference C source (`ZSTD_decodeSequence` in
    `zstd_decompress_block.c`, fetched directly from
    `github.com/facebook/zstd`, per the Lesson-96 playbook of not trusting
    a paraphrase alone) and against the already-fuzz-tested `c/zstd`
    implementation from PR #9941.
  - Verified with the real `zstd` CLI: a 60-case fuzz corpus (random,
    periodic, constant-byte, cyclic, prose-like, and mixed literal/repeat
    patterns) that failed 24/60 before the fix (11 with this exact
    underflow signature) now fails only on the 13 cases that hit this
    package's pre-existing, unrelated, documented scope limits (Huffman
    literals / FSE-compressed literals / non-predefined FSE modes — out of
    scope for this educational codec, unrelated to repeat offsets). Two new
    permanent tests added: `RepeatOffsetCliInterop` (alternating literal
    run / long same-byte match, forcing the real CLI to reuse a distance
    across sequences) and `SingleByteRunCliInterop` (the exact minimal
    repro from `c/zstd`'s Lesson 98: 4713 bytes of one repeated byte).
    Both reproduce the pre-fix failure and pass byte-exact after the fix.
    All 32 pre-existing tests continue to pass unmodified — this package's
    own encoder never emits repeat-offset codes, so its self-consistency
    round trip is untouched by this fix.

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
