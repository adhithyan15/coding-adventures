# Changelog — go/zstd

All notable changes to this package follow [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.0.3] — 2026-08-03

### Fixed

- **FSE sequences-section codec had three compounding conformance bugs
  (RFC 8878 §3.1.1.3.2), all invisible to purely-internal round-trip
  testing.** Found via a repo-wide zstd conformance audit (alongside
  matching fixes to `java/zstd` and `kotlin/zstd`) after the same bug class
  was confirmed to also reproduce in `rust/zstd`. See lessons.md Lesson 96
  for the full story.
  1. `buildDecodeTable`/`buildEncodeTable` spread symbols into FSE table
     slots using a fabricated two-pass split ("all count>1 symbols first,
     then all count==1 symbols", both in ascending symbol order) — a
     plausible-looking but entirely invented convention. The real algorithm
     (`FSE_buildDTable_internal`'s low-probability branch, verified against
     the reference C source at github.com/facebook/zstd) is a SINGLE pass
     over symbols `0..len(norm)-1`, placing each symbol's full count
     immediately when encountered. Both functions now do a single pass.
  2. `decompressBlock`'s per-sequence decode combined "peek symbol" and
     "update state" into one step (via the now-removed `fseDecodeSym`), in
     the wrong order relative to reading extra bits. RFC 8878
     §3.1.1.3.2.1.2 requires: PEEK all three symbols (LL, ML, OF) from the
     current states first (free — no bits consumed), THEN read extra bits
     in order OF, ML, LL, THEN (see #3) update states in order LL, ML, OF.
     This logic is now in a new `decodeSequencesSection` helper, extracted
     out of `decompressBlock` so the low-level FSE unit tests exercise the
     real production decode path instead of a hand-rolled parallel one.
  3. The state-transition update was performed for every sequence,
     including the last one in a block. The real decoder skips it for the
     last sequence (no "next" sequence needs a fresh state), and the
     encoder must mirror this: the first symbol processed in
     `encodeSequencesSection`'s reverse encode loop (semantically the LAST
     real sequence) now gets its starting state from a new `fseInitState`
     function (mirrors real zstd's `FSE_initCState2`, writes no bits at
     all) instead of a normal `fseEncodeSym` bit-flushing transition.
  - All three bugs were self-cancelling as long as encode and decode used
    the SAME (wrong) convention, so every existing test — including two
    low-level "encode two/one sequence(s) by hand, decode them, check
    `(ll,ml,off)` match" unit tests — passed regardless. Confirmed via a
    minimal repro (`compress(strings.Repeat("ababababab", 30))`,
    decompressed with the real `zstd -d` CLI, which failed with
    `Decoding error (36): Data corruption detected` before this fix).
- **`Content_Checksum_Flag` in the Frame Header Descriptor is bit 2, not
  bit 4** (RFC 8878 §3.1.1.1 — bit 4 is `Unused_bit`). Both `Compress`'s
  comment and `Decompress`'s bit-read were wrong; the encoded byte (`0xE0`)
  was numerically unaffected (bit 2 and bit 4 are both 0 in that constant),
  but the decoder's parsed value was previously read from the wrong bit
  position. This package does not yet enforce a Lesson-94-style
  trailing-bytes check, so the bug was latent rather than fatal — fixed
  proactively per lessons.md Lesson 95's warning that the two must be fixed
  together if the trailing-bytes check is ever added.

### Added

- **New TC-11 real `zstd` CLI interop tests** (`TestTC11CliInterop`,
  `TestTC11CliInteropHighSequenceCount`), the test class that actually
  would have caught the FSE bugs above: they shell out to the real `zstd`
  binary (skipped, not failed, if it isn't on `PATH`) and verify both
  directions — compress with this package, decompress with `zstd -d`; and
  compress with `zstd`, decompress with this package. No language port of
  this repo's zstd family had a real-CLI interop test before this change.
- `decodeSequencesSection` — decodes the FSE sequences bitstream into
  `(ll, ml, off)` tuples independent of literal/match-copy logic, shared by
  both `decompressBlock` and the low-level FSE unit tests.
- `fseInitState` — direct-formula FSE encoder state initialisation (mirrors
  `FSE_initCState2`), used for the first-processed (last real) sequence in
  `encodeSequencesSection`.

### Changed

- `TestFSETwoSequenceRoundtrip` / `TestFSESingleSequenceRoundtrip` now call
  the production `encodeSequencesSection`/`decodeSequencesSection`
  functions instead of hand-rolling a parallel encode/decode loop inline —
  the hand-rolled version is exactly the kind of test that stayed green
  through all three bugs above.

## [0.0.2] — 2026-04-26

### Fixed

- **`encodeSeqCount` / `decodeSeqCount` now use RFC 8878 §3.1.1.3.1 layout.**
  The 2-byte form previously emitted bytes via
  `binary.LittleEndian.AppendUint16(nil, count|0x8000)`, placing the LOW
  byte first. The decoder branches on byte0 to determine the form: for any
  count ≥ 128 whose low byte was < 128 (e.g. count=515 → byte0=0x03), the
  decoder mis-took the 1-byte path and returned a tiny garbage count,
  mis-aligning every byte downstream. Roughly half of all counts in the
  2-byte range silently corrupted; the other half worked, so most existing
  tests passed.
- New `TestSeqCountEndiannessRegression` round-trips 200 KB of repetitive
  text (> 128 sequences in a single block).
- New `TestEncodeSeqCountRoundTrip` exhaustively round-trips every count in
  the 1-byte range (0..127), the entire 2-byte range (128..0x7EFF), and a
  spot of 3-byte values.

## [0.0.1] — 2026-04-24

### Added

- Initial implementation of Zstandard (ZStd) compression/decompression (CMP07).
- `Compress(data []byte) []byte` — encodes a byte slice to a valid ZStd frame
  (RFC 8878). Automatically selects the best block type per 128 KB block:
  - RLE block when all bytes are identical (1 byte payload).
  - Compressed block when FSE + LZ77 shrinks the data.
  - Raw block as fallback when compression is not beneficial.
- `Decompress(data []byte) ([]byte, error)` — decodes any RFC 8878-compliant ZStd
  frame. Supports Raw, RLE, and Compressed (Predefined FSE) block types.
  Caps output at 256 MB to prevent decompression bombs.
- FSE (Finite State Entropy) encode and decode tables built from the predefined
  distributions in RFC 8878 Appendix B (LL acc_log=6, ML acc_log=6, OF acc_log=5).
- Reverse bit-writer (`revBitWriter`) and reverse bit-reader (`revBitReader`)
  implementing ZStd's backward-written bitstream with sentinel-bit framing.
- Internal `buildDecodeTable` and `buildEncodeTable` with the two-pass spread
  algorithm matching the reference implementation.
- Literals section encoding and decoding for Raw_Literals type with 1/2/3-byte
  headers covering sizes up to 1 MB.
- Sequence count encoding and decoding covering all three byte-width ranges.
- `llToCode` and `mlToCode` helpers for mapping values to RFC 8878 code numbers.
- Depends on `go/lzss` (CMP02) for LZ77 match-finding with a 32 KB window.
- 51 unit tests: TC-1 through TC-10 (matching the Rust reference tests) plus
  round-trip tests, wire-format validation, FSE codec unit tests, and error-path
  tests. Coverage: 93.7% of statements.
