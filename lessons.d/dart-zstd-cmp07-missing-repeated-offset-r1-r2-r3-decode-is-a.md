# `dart/zstd` (CMP07): missing Repeated-Offset (R1/R2/R3) decode is a repo-wide `zstd`-port gap, not a Dart-specific bug

**Date:** 2026-08-05

**What happened:** While implementing the first `c/zstd` port (PR #9941),
that agent found — independently of the Lesson 95-97 FSE codec bugs — that
`ZSTD_decodeSequence`'s **Repeated-Offset (R1/R2/R3) mechanism** (RFC 8878
§3.1.1.3.2.1.1) was never implemented in this repo's `zstd` decoders. The
sequence Offset_Value on the wire is not always a literal distance: values
1, 2, and 3 are reserved as references into a 3-slot history of
recently-used offsets (R1/R2/R3, defaulting to `{1, 4, 8}` at the start of
a frame, threaded across every Compressed block in the frame — not reset
per block); only Offset_Value >= 4 is `actual_offset + 3`. Real `zstd`
encoders use repeat offsets constantly (one of the format's main entropy
wins), but every port in this repo's own `compress()` always emits an
explicit +3-biased offset and so never *emits* Offset_Value 1/2/3 itself —
meaning no in-process round trip, including every prior CLI-fuzz corpus in
Lessons 96/97 (which only ever fuzzed the ours→`zstd -d` direction), could
ever exercise this decode path. `code/packages/dart/zstd` inherited the
identical gap: `_decompressBlock` computed every offset as flat
`of_raw - 3` and threw `FormatException: decoded offset underflow` the
instant a real `zstd`-CLI-produced frame's sequence had Offset_Value 1, 2,
or 3 — confirmed with a minimal repro (4713 bytes of one repeated byte,
compressed by the real CLI: a single Compressed block whose one sequence
has Offset_Value=1, i.e. "reuse the default R1") and with the CMP07 spec's
own TC-8 fixture (`pattern + (b"X" * 128 + pattern) * 10`), both of which
failed identically before the fix and passed after it.

**Rule:**
- A decoder's job is to understand every valid wire form a *real* encoder
  emits, not just the subset this repo's own (deliberately simplified)
  encoder happens to produce. "Our encoder never emits X" is a valid reason
  to skip implementing X in the encoder; it is never a valid reason to skip
  understanding X in the decoder, if the format's real-world encoders use X
  routinely. Repeat offsets are exactly this shape of gap: cheap for the
  encoder to skip, but the single highest-value feature to get right for
  the decoder, since real periodic/repetitive data (the case `zstd` is
  optimized for) triggers it on nearly every sequence.
- Cross-check the algorithm against more than one source before trusting
  it: the exact selector mapping (which of R1/R2/R3 each of Offset_Value
  1/2/3 selects, the `Literals_Length == 0` special case that shifts the
  mapping by one, and the post-sequence history-rotation shape) is easy to
  get subtly wrong from memory or a single paraphrase. `c/zstd`'s PR #9941
  fix (`decompress_block`'s `_resolveOffset`-equivalent) was cross-checked
  against both the RFC 8878 prose and the reference decoder
  (`ZSTD_decodeSequence` in `facebook/zstd`'s `zstd_decompress_block.c`,
  fetched live) and independently fuzz-tested (1500 trials, ASan/UBSan
  clean) before this Dart fix reused the identical, now-doubly-verified
  formula rather than re-deriving it from scratch a third time.
- This is a **repo-wide** `zstd`-port gap, not a Dart-specific bug — every
  language's `zstd` decoder that computes offset as flat `code - 3` without
  a 3-slot offset-history state machine has the identical hole and will
  fail to decode real-world `.zst` files that use repeated offsets (i.e.
  most of them). Each port should be audited and fixed the same way: add a
  frame-scoped `[R1, R2, R3]` list threaded through every Compressed block,
  resolve Offset_Value 1/2/3 against it per the selector table above, leave
  the encoder unchanged, and add a real-CLI-interop regression test — an
  in-process round trip can never catch this class of bug on its own. See
  Lesson 98 above (`code/packages/c/zstd`'s original fix) for the
  cross-checked selector formula this entry's Dart fix reused verbatim.
