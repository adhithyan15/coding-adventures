# Lesson 99 — `fsharp/zstd` inherited the same Repeated-Offset (R1/R2/R3) decode gap as `c/zstd` (Lesson 98), confirmed via real `zstd` CLI interop

**Date:** 2026-08-05

**What happened:** While implementing `c/zstd` (CMP07, PR #9941), an ad hoc
fuzz sweep against the real `zstd` CLI found that its decoder never
implemented Repeated-Offset (R1/R2/R3) sequence decoding (RFC 8878
§3.1.1.3.2.1.1) — an `Offset_Value` of 1, 2, or 3 is a reference into a
three-entry offset history, not a literal `Offset_Value - 3` computation.
That PR flagged every other language's `zstd` port as "suspect of the same
gap until specifically checked against varied real-world `zstd`-CLI-encoded
input" (documented as that PR's own Lesson 98). Auditing
`code/packages/fsharp/zstd/Zstd.fs`'s `DecompressBlock` confirmed it: the
line `let matchOffset = rawOffset - 3` computed the actual match offset
unconditionally, with no repeat-offset interpretation for `rawOffset` values
1-3, and no offset-history registers threaded through the frame at all.

A regression test (added to `ZstdTests.fs` BEFORE the fix, to prove the gap
rather than assume it): compressing 4713 bytes of a single repeated byte
`'Z'` with the real `zstd` CLI (no `--no-check`, exercising the checksum
trailer too) and decompressing with this package's `Zstd.Decompress`
raised `System.IO.InvalidDataException: match offset exceeds decoded
output` — real `zstd` chose a Compressed block whose one sequence is 2
literal bytes ("ZZ") + a match with `Offset_Value=1` (i.e. "reuse
`Repeated_Offset1`", which starts at its RFC-mandated default of 1, an
unmistakable RLE-via-repeat-offset pattern); the pre-fix decoder computed
`rawOffset - 3 = 1 - 3` which underflows a `let` binding typed as `int` to a
large negative-then-implicitly-huge value once used as an array/offset
computation, correctly rejected by the existing offset-bounds check as
malformed — even though the frame was perfectly valid.

**Why it went unnoticed:** Identical shape to the `c/zstd` finding. This
package's own `EncodeSequences` is, by design, incapable of emitting
`Offset_Value <= 3` (the minimum LZSS match offset is 1, so
`rawOffset = offset + 3 >= 4` always), so a self round-trip — and every
pre-existing TC-9 CLI-interop test in this package (added in 0.1.1 for the
Lesson 96 FSE-codec bugs), whose fixed prose corpus never happened to
produce a real-`zstd`-encoded sequence with `Offset_Value <= 3` — never
exercised this decode path.

**Rule (same as Lesson 98, reconfirmed cross-language):**
- Decode-side feature scope and encode-side feature scope are separate
  decisions. An "educational subset" simplification stated as "we don't
  emit repeat-offset sequences" must not be silently read as "we don't need
  to decode them," when the decoder's job is to accept output from the real,
  independent `zstd` CLI ecosystem — which uses repeat offsets constantly,
  as one of its principal entropy wins.
- A single fixed TC-9 corpus is necessary but not sufficient evidence of
  decoder conformance. Before trusting "TC-9 passes," fuzz the same
  interop check across varied inputs (constant-byte runs are the cheapest,
  most reliable way to force real zstd into a repeat-offset-heavy encoding).
- Fixed in `code/packages/fsharp/zstd/Zstd.fs`: `DecompressBlock` now takes
  `rep1`/`rep2`/`rep3` as `byref<int>` (frame-scoped, threaded from
  `Decompress` through every Compressed block, default 1/4/8 for the first
  block per RFC 8878), and implements the full peek-then-select-then-rotate
  mechanism for `Offset_Value` 1-3 — including the "when `Literals_Length`
  is 0, repeated offsets are shifted by 1" special case, using the peeked
  (not-yet-extra-bit-read) literal-length code to know whether the eventual
  literal length is zero. Ported from, and cross-checked against, both the
  literal reference C source (`ZSTD_decodeSequence` in
  `zstd_decompress_block.c`, github.com/facebook/zstd) transcribed in
  `c/zstd`'s fix (`gh pr diff 9941`) and an independent RFC 8878 §3.1.1.3.2.1.1
  fetch — not re-derived from memory. The encoder is unchanged (still never
  emits repeat-offset codes; decode-only fix). semantic package version
  bumped 0.1.1 -> 0.1.2.
- Verified via two new TC-9 regression tests (the 4713-byte constant-run
  repro above, plus an independent periodic-6-byte-cycle repro not
  dependent on the constant-byte-specific heuristic) — all 26 existing +
  new tests pass, line coverage 91.16% (threshold 80%) — plus an ad hoc
  42-case fuzz sweep against the real `zstd` CLI (constant, periodic at
  several cycle lengths, ramp, random, and prose patterns, 16 bytes to
  20 KB), all byte-exact.
- Every other language's `zstd` port in this repo remains suspect of the
  same gap until specifically audited — this PR only covers `fsharp/zstd`.

See also `gh pr diff 9941` (`c/zstd`, Lesson 98) for the original finding
and reference fix this port's fix was cross-checked against.

---
