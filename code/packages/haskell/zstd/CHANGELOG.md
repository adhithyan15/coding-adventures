# Changelog

## 0.1.1

Audited against a real RFC 8878 conformance bug found in the sibling
`java`/`kotlin`/`rust` `zstd` ports (see `lessons.md` Lesson 95) and
confirmed present here too via real `zstd` CLI interop testing.

### Fixed

- **FSE table-spread used a fabricated two-pass split.** `spreadSymbols`
  placed all `count > 1` symbols first, then all `count == 1` symbols
  (both in ascending symbol order) -- a plausible-looking but invented
  convention. The real algorithm (`FSE_buildDTable_internal`'s
  low-probability branch, verified against the zstd C reference source)
  is a single pass over symbols `0..maxSymbolValue`, placing each symbol's
  full count immediately when encountered. The two-pass version produced a
  different, but internally self-consistent, table layout -- our own
  decoder mirrored our own encoder, so every round-trip test passed, but
  the real `zstd` CLI rejected our compressed output as corrupt.
- **Per-sequence field order was wrong in two ways.** A decoder must PEEK
  all three symbols (LL/ML/OF) from the current FSE states first (free --
  the state itself is the table index, no bits consumed), THEN read extra
  bits in order OF, ML, LL, THEN update states in order LL, ML, OF. The
  previous decoder fused peek-and-update into one step and performed it
  eagerly in LL, OF, ML order *before* reading any extra bits. The
  previous encoder had the mirror-image ordering bug. Also fixed: the
  one-time initial FSE-state read at the start of a compressed block is
  LL, OF, ML -- a genuinely different order from the per-sequence update
  order (LL, ML, OF) -- which the previous decoder got wrong by reading
  LL, ML, OF for both.
- **The state-transition "update" was performed unconditionally for every
  sequence, including the last one in a block.** There is no "next"
  sequence to prepare a state for after the last one, so a real decoder
  skips that read entirely, and a real encoder cannot produce that
  sequence's starting state via a normal bit-flushing transition (there is
  no corresponding decode-side bit-read to consume it) -- it must be
  computed directly via a new `encodeInitState` function (mirrors real
  zstd's `FSE_initCState2`), which writes no bits at all. The previous
  encoder always flushed a transition uniformly, writing bits a real
  decoder would never read and shifting the bit-alignment of everything
  that followed.
- **FHD `Content_Checksum_Flag` was read from bit 4 instead of bit 2**
  (RFC 8878 §3.1.1.1), and the "reserved bits" check treated bits 2+3 as
  jointly reserved -- rejecting every real checksummed frame as malformed
  while never actually detecting a checksum trailer on any frame. Verified
  empirically against `zstd -c`/`zstd -c --no-check` output.

All three FSE-codec bugs were self-cancelling in every existing round-trip
test (encoder and decoder agreed on the same wrong convention), and were
only caught by decompressing this package's own compressed output with
the real `zstd` CLI -- see the new TC-9 interop test below.

### Added

- `ZstdCliInteropSpec`: a real cross-implementation round-trip test (TC-9)
  against the system `zstd` binary via `System.Process`, in both
  directions (compress here / decompress with the CLI, and compress with
  the CLI / decompress here), plus a dedicated high-sequence-count case
  that exercises the 2-byte sequence-count wire form. Gracefully marked
  pending (not failed) when `zstd` isn't on `PATH`.
- `directory` and `process` test-suite dependencies, needed for the above.

### Changed

- Regenerated the hard-coded "established cross-language compressed
  vector" test fixture and the checksum-frame hand-crafted fixture in
  `ZstdSpec` to match the corrected wire format; both were built against
  the old, non-conformant encoding.

## 0.1.0

- Add the pure Haskell CMP07 educational Zstandard encoder and decoder.
- Support standard frame headers, raw blocks, RLE blocks, raw literals, and
  predefined FSE sequence tables.
- Compose the native Haskell `lzss` package for deterministic match finding.
