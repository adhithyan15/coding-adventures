# Changelog

## 0.1.2

- **Fixed: FSE sequences-section codec was not RFC 8878 conformant**, despite
  passing every internal round-trip test. This package is a documented
  near-line-for-line port of `rust/zstd`, which (along with `java/zstd`) was
  confirmed to have the same three compounding bugs — see lessons.md Lesson
  95/96 for the full investigation. Fixed here:
  1. `_buildDecodeTable` / `_buildEncodeSym` used a fabricated two-pass
     table-spread split (all count>1 symbols first, then count==1 symbols)
     instead of the real single-pass algorithm (`FSE_buildDTable_internal`):
     one pass over symbols `0..maxSymbolValue`, placing each symbol's full
     count immediately. The two-pass version produced a different — but
     internally self-consistent — table layout, so our own encoder/decoder
     pair always agreed with itself while producing non-conformant output.
  2. Per-sequence field order in `_encodeSequencesSection` /
     `_decompressBlock` was wrong in two ways: the decoder must PEEK all
     three symbols (LL, ML, OF) from the current FSE states first (a bare
     table lookup — no bits consumed), THEN read extra bits in order
     OF, ML, LL, THEN update states in order LL, ML, OF. The previous code
     combined peek-and-update into one step (`_fseDecodeSym`) and read/wrote
     extras and updates in the wrong relative order. New `_fsePeek` /
     `_fseUpdateState` helpers split peek from update to make this
     structurally impossible to get wrong again.
  3. The FSE state-transition "update" must be skipped for the LAST sequence
     in a block (no "next" sequence needs a prepared state) — encoder's
     first-processed sequence in its reverse loop (semantically the LAST
     real sequence) needs direct-formula state initialisation instead of a
     normal bit-flushing transition. Added `_fseInitState` (mirrors the
     reference `FSE_initCState2`) for this; `_decompressBlock`'s update step
     is now conditioned on `i != nSeqs - 1`.
  Verified against the real `zstd` CLI (compress with ours / decompress with
  `zstd -d`, and the reverse) across a 60-case fuzz corpus spanning periodic
  patterns, semi-random run-length data, pure random data, prose at varying
  repeat counts, and high-sequence-count binary patterns — all pass in the
  ours→`zstd -d` direction (the direction that proves our encoder's wire
  format is conformant).
- **Fixed: FHD `Content_Checksum_Flag` bit was mislabelled as bit 4** in
  comments; the real RFC 8878 §3.1.1.1 position is bit 2 (bit 4 is
  `Unused_bit`). The frame this package's `compress()` emits (`0xE0`) has
  both bits clear either way, so this was not a functional bug for our own
  frames — but `decompress()` now correctly reads the checksum flag from
  bit 2 and skips the trailing 4-byte xxHash64 checksum when present, which
  matters when decompressing real-world / CLI-produced `.zst` files (which
  enable the checksum by default). See lessons.md Lesson 95.
- Added a real **TC-9: CLI interoperability** test that shells out to the
  system `zstd` binary via `dart:io Process` and round-trips in both
  directions (ours→`zstd -d` and `zstd`→ours). The previous test labelled
  `TC-9` was actually an unrelated bad-magic error-handling test (now
  relabelled `Edge: bad magic throws FormatException`) — this package never
  had a real interoperability test before. Skips gracefully (does not fail)
  when the `zstd` CLI isn't on `PATH`.
- Added an `RT: CLI interop — high sequence count` regression test:
  compresses a 9000-byte input producing 128+ sequences (exercising the
  2-byte `Number_of_Sequences` wire form) and verifies the real `zstd -d`
  CLI can decode it, giving the corrected per-sequence FSE codec heavier
  exercise than a single-sequence input would.
- Relabelled the hand-crafted wire-format decode test as `TC-10` to match
  the CMP07 spec's test-case numbering (it was previously unlabelled
  `Wire format: ...`); its FHD byte-layout comment now correctly cites bit 2
  (not bit 4) as `Content_Checksum_Flag`.

## 0.1.1

- Tests: added `Seq count: 200 KB repetitive text — endianness regression`
  to lock in the wire-format invariant for the 2-byte sequence-count form.
  This is the same shape as the regression added to TS+Go in PR #1448 — it
  reliably produces ≥ 128 sequences in a single block, exercising the
  2-byte path of `_encodeSeqCount` / `_decodeSeqCount`.
- Audited `_encodeSeqCount` / `_decodeSeqCount`: already RFC 8878
  §3.1.1.3.1-compliant (the existing comment near the encoder explicitly
  cites the `or 0x8000 as LE16` form being avoided); no fix needed.

## 0.1.0

- Added the initial Dart implementation of the CMP07 ZStd compression package.
- Added `compress` function that encodes data into a valid RFC 8878 ZStd frame.
- Added `decompress` function that decodes any RFC 8878 ZStd frame with Raw, RLE, or Compressed blocks.
- Implemented predefined FSE (Finite State Entropy) tables for Literal Length, Match Length, and Offset coding.
- Implemented the reversed bitstream encoder (`RevBitWriter`) and decoder (`RevBitReader`) used by ZStd's sequence section.
- Implemented raw literals section encoding and decoding with 1/2/3-byte header variants.
- Implemented ZStd sequence count encoding and decoding with 1/2/3-byte forms.
- Integrated the `coding_adventures_lzss` package for LZ77 back-reference generation with a 32 KB window.
- Added decompression bomb guard: output capped at 256 MB.
- Added support for multi-block frames (inputs > 128 KB split across blocks).
- Added RLE block detection: runs of a single byte value emit a 4-byte block instead of full compression overhead.
