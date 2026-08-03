# Changelog — swift/zstd

All notable changes to the `swift/zstd` package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.2] — 2026-08-03

### Fixed

- **FSE sequences-section codec conformance (RFC 8878).** A repo-wide audit
  (see `lessons.md` Lesson 96, originating from `java/zstd` PR #9780 and
  confirmed independently against `rust/zstd`) found this package's FSE
  codec produced output that only ever round-tripped against its OWN
  encoder/decoder pair — every internal test passed, but the real `zstd`
  CLI rejected our compressed output with `Decoding error (36): Data
  corruption detected`. Reproduced and confirmed here before fixing, via a
  new real-CLI interop test (`testTC9CliInterop`), which failed identically
  prior to this fix. Three compounding, self-cancelling bugs were found and
  fixed, all in `buildDecodeTable`, `buildEncodeTable`,
  `encodeSequencesSection`, and `decompressBlock`:
  1. **Table-spread algorithm.** `buildDecodeTable`/`buildEncodeTable` used a
     fabricated two-pass split ("all symbols with count > 1 first, then all
     count == 1 symbols", both in ascending symbol order) to spread symbols
     into FSE table slots. The real algorithm
     (`FSE_buildDTable_internal`'s low-probability branch, verified against
     the reference C source in `github.com/facebook/zstd`) is a SINGLE pass
     over symbols `0..<norm.count`, placing each symbol's full count
     immediately when encountered. The two-pass version produced a
     completely different (but internally self-consistent) table layout.
  2. **Per-sequence field order.** RFC 8878 §3.1.1.3.2.1.2 (cross-checked
     against `ZSTD_decodeSequence`): a decoder must PEEK all three symbols
     from the current FSE states first (a bare table lookup — free, no bits
     consumed), THEN read extra bits in order OF, ML, LL, THEN update states
     in order LL, ML, OF. The previous code combined peek-and-update into
     one step per symbol and read extras in the wrong relative order and
     sub-order (LL, ML, OF instead of OF, ML, LL) — it also read the three
     initial FSE states in the wrong order (LL, ML, OF instead of the
     RFC's asymmetric LL, OF, ML).
  3. **Last-sequence state-update skip.** The state-transition "update" must
     be skipped entirely for the LAST sequence in a block (there is no
     "next" sequence to prepare a state for). The encoder's mirror-image
     requirement: the first-processed sequence in its reverse encode loop
     (semantically the LAST real sequence) needs its starting state
     computed directly via a new `fseInitState` function (mirroring real
     zstd's `FSE_initCState2` — no bits written at all), not a normal
     bit-flushing `fseEncodeSym` transition. The previous encoder always
     flushed a transition uniformly for every sequence, and the previous
     decoder always performed the state-update read unconditionally,
     writing/reading bits a real decoder/encoder would never
     read/write — shifting the bit-alignment of everything downstream.
  - `fseDecodeSym` (which combined peek+update) was removed; the
    per-sequence loop in `decompressBlock` now does the peek and the
    conditional update as separate, explicitly ordered steps.
  - Corrected the FHD comment documenting `Content_Checksum_Flag` as bit 4;
    RFC 8878 §3.1.1.1 places it at bit 2 (bit 4 is `Unused_bit`). This
    package always emits `0xE0` regardless (Educational Simplification: no
    checksum is computed), so the wire output was unaffected — only the
    documentation was wrong (see Lesson 95).
- Corrected the `buildDecodeTable` doc comment's inaccurate claim of
  "matching the Rust reference implementation exactly" — `rust/zstd` carried
  the identical table-spread bug (see Lesson 96) prior to this fix, so that
  claim was never something to aspire to; the doc comment now cites the real
  zstd C reference source instead.

### Tests

- Added `testTC9CliInterop` and `testTC9CliInteropCorpus`: real interop
  tests against the system `zstd` CLI (via `Foundation.Process`), in both
  directions — compress with ours / decompress with `zstd -d`, and compress
  with `zstd -c` / decompress with ours. `testTC9CliInterop` uses the
  minimal repro from Lesson 96 (a single sequence, `ll=2, ml=28, off=2`);
  `testTC9CliInteropCorpus` covers periodic patterns, prose, semi-random
  run-length data, and pure pseudo-random data at multiple sizes. Both tests
  are skipped (`XCTSkip`) rather than failed when the `zstd` CLI isn't found
  on PATH. The CLI → ours direction tolerates `ZstdError.unsupportedFSEModes`
  as an expected outcome (a pre-existing, documented scope boundary — this
  decoder only implements Predefined-mode FSE tables and Raw literals, not
  FSE_Compressed/RLE/Repeat modes or Huffman literals — distinct from the
  Lesson 96 conformance bug, which manifested as corruption/mismatch, never
  this typed error).
- Confirmed (empirically, before applying the fix) that `testTC9CliInterop`
  failed with the real CLI's `Decoding error (36): Data corruption
  detected` against the unfixed encoder — the same signature documented in
  Lesson 96 for `java/zstd` and `rust/zstd`.

## [0.1.1] — 2026-04-26

### Tests

- Added `testSeqCountEndiannessRegression`. The test round-trips 200 KB of
  repetitive ASCII, reliably yielding ≥ 128 sequences in a single block —
  exercising the 2-byte path of `encodeSeqCount` / `decodeSeqCount`. Same
  shape as the regression added to TS+Go in PR #1448.
- Audited `encodeSeqCount` / `decodeSeqCount`: already RFC 8878
  §3.1.1.3.1-compliant (`0x80 | (count >> 8), count & 0xFF`); no fix needed.

## [0.1.0] — 2026-04-24

### Added

- Initial implementation of ZStd (RFC 8878) compression and decompression in Swift.
- `compress(_:) -> [UInt8]` — single-shot compressor producing a standards-compliant
  ZStd frame.
- `decompress(_:) throws -> [UInt8]` — single-shot decompressor with 256 MiB output cap.
- `ZstdError` — typed error enum covering all failure modes (bad magic, truncated input,
  unsupported FSE modes, invalid offsets, etc.).
- **FSE predefined tables** from RFC 8878 Appendix B for Literal Length (accLog=6),
  Match Length (accLog=6), and Offset (accLog=5).
- **`buildDecodeTable`** — three-phase FSE decode table builder matching the Rust
  reference implementation exactly (rare-symbols-first spreading + nb/base assignment).
- **`buildEncodeTable`** — symmetric FSE encode table builder returning `([FseEe], [UInt16])`.
- **`RevBitWriter`** / **`RevBitReader`** — backward bit-stream codec used by the ZStd
  sequence section (sentinel-bit framing, left-aligned register on decode).
- **Block strategies**: Raw, RLE (all-bytes-identical detection), Compressed (LZ77 + FSE).
- **Literals section**: Raw_Literals format with 1/2/3-byte header encoding/decoding.
- **Sequences section**: sequence count encoding, symbol-modes byte, FSE bitstream
  encode/decode for LL × OF × ML triples.
- **LL / ML code tables** (36 and 53 entries respectively) matching RFC 8878 §3.1.1.3.
- Dependency on `swift/lzss` for LZ77 match-finding (32 KB window, max match 255, min 3).
- Correct 2-byte sequence count encoding using the RFC 8878 big-endian-ish scheme
  `[0x80 | (count >> 8), count & 0xFF]` which guarantees byte_0 ∈ [0x80, 0xFE] for
  all counts in [128, 32512), avoiding the 0xFF sentinel collision that would corrupt
  the symbol-modes byte offset on decoding.
- `Package.swift` (swift-tools-version 5.9) with LZSS product dependency.
- `BUILD` script running `swift test --enable-code-coverage --verbose`.
- 20 XCTest test cases (TC-1 through TC-15 plus regression tests) covering:
  - Empty, single-byte, all-256-bytes round-trips.
  - RLE block compression ratio check.
  - English prose compression ratio (≥ 20%).
  - LCG pseudo-random data round-trip.
  - Multi-block (200 KB / 300 KB) round-trips.
  - Bad magic, too-short frame error paths.
  - Deterministic output guarantee.
  - Wire-format decode (manual frame, no encoder dependency).
  - Alternating pattern compression ratio (≥ 30%).
