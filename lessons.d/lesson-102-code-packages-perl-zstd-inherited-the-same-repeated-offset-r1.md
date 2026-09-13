# Lesson 102 — `code/packages/perl/zstd` inherited the same Repeated-Offset (R1/R2/R3) decode gap discovered in `c/zstd` (Lesson 98), confirmed via real `zstd` CLI interop

**Date:** 2026-08-05

**What happened:** While implementing the new `c/zstd` port (CMP07, PR
#9941), fuzzing against the real `zstd` CLI found that its decoder never
implemented Repeated-Offset (R1/R2/R3) sequence decoding (RFC 8878
§3.1.1.3.2.1.1) — RFC 8878 reserves sequence `Offset_Value` 1..3 for
"repeat offsets" (a reference into a three-entry offset history, default
`1/4/8`, frame-scoped), not a literal `Offset_Value - 3` computation. That
PR's Lesson 98 flagged this as likely repo-wide, since every port in this
repo shares the same "encoder never emits repeat-offset codes" educational
simplification (`raw_offset = offset + 3` always, so `of_code >= 2`
always), meaning no port's own compress()/decompress() round trip — nor
TC-9's one fixed prose corpus — ever exercises the decoder's repeat-offset
path.

Auditing `code/packages/perl/zstd/lib/CodingAdventures/Zstd.pm` confirmed
the same gap: `_decompress_block` computed `offset = of_raw - 3`
unconditionally and `die`d with `"offset underflow (of_raw=$of_raw)"` for
any `of_raw < 3` — i.e. every repeat-offset sequence. Reproduced with the
exact same minimal repro as the C port (4713 bytes of a single repeated
byte — real `zstd` picks a Compressed block with one sequence,
`Offset_Value=1`, reusing `Repeated_Offset1` which starts at its default
value of 1): `decompress_block: offset underflow (of_raw=1)`.

**Fix:** `_decompress_block` and `decompress` now thread a frame-scoped
`[rep1, rep2, rep3]` offset-history array (default `[1, 4, 8]`) through
every Compressed block in a frame, mutated in place. For offset code `>= 2`
(explicit offset), `offset = of_raw - 3`, then the history rotates in the
new offset (`rep3 <- rep2 <- rep1 <- offset`). For offset code `<= 1`
(`of_raw` in `{1, 2, 3}`, a repeat-offset reference), the actual register
used depends on both `of_raw` and whether `Literals_Length == 0` for this
sequence (`selector = ll_is_zero + of_raw - 1`, RFC 8878's "when
Literals_Length is 0, repeated offsets are shifted by 1" rule):
`selector 0` reuses `rep1` unchanged (no rotation), `selector 1` swaps in
`rep2`, `selector 2` rotates in `rep3`, and `selector 3` rotates in
`rep1 - 1`. `ll_is_zero` is knowable from the PEEKED LL code alone (LL code
0 is the only code with baseline 0 and 0 extra bits), before any extra bits
are read — matching the reference decoder's evaluation order. Algorithm
cross-checked against both the RFC 8878 prose and the reference C source
(`ZSTD_decodeSequence` in `zstd_decompress_block.c`, per the Lesson-96
playbook of not trusting either alone), mirroring the `c/zstd` fix exactly.
The encoder is unchanged (decode-only fix).

**Verification:** the original constant-byte repro now decodes correctly;
a 60-trial fuzz sweep (periodic/constant/ramp/low-entropy-random byte
patterns, real `zstd` CLI → `decompress()`, sizes 500–4500 bytes) passed
with zero failures; all 28 pre-existing tests in `t/zstd.t` still pass
unaffected, since this port's own round trip never touches the new code
path. New `RT-12` regression test added covering both the constant-byte
repro and a cyclic-pattern case that forces back-to-back repeat-offset
sequences within one block.

**Rule:** when a sibling language port documents a decode-only feature gap
that stems from a *shared, repo-wide* encoder-side simplification (as
opposed to a port-specific translation mistake), audit every other port
implementing the same spec for the identical gap — the shared design
choice that hid the bug from that port's own tests hides it identically
everywhere else the same choice was made.

---
