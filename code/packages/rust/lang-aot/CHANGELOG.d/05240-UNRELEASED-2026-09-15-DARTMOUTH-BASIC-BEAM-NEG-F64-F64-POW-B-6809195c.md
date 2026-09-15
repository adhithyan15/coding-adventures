## Unreleased — 2026-09-15 — Dartmouth BASIC BEAM neg(f64)/f64_pow (BEAM03 continuation)

`iir-to-beam` 0.11.0 adds `neg`(f64) (no lowering change needed — the
existing `gc_bif1 erlang:-/1` path was already polymorphic over int and
float) and `f64_pow` (a new `call_ext math:pow/2`, since `math:pow/2` is not
a loader-recognized guard BIF). Design + real-`erl` verification:
`code/specs/BEAM03-float-lowering.md` §8. This promotes two more Dartmouth
BASIC `lang_matrix` rows to `Beam`:

- `10 PRINT ABS(-42)\n20 END\n` → `"42"` — ABS's inline `if X < 0 then -X
  else X` lowering, plus the unary-minus literal `-42` itself, exercises
  `neg`(f64) twice.
- `10 PRINT 4 ^ 0.5\n20 END\n` → `"2"` — a fractional exponent misses the
  literal-integer-exponent fast path and falls through to the general
  `f64_pow` runtime call.

Both pass on real Erlang (`portable_text_stdout_dartmouth_basic_beam_neg_and_pow`,
new). Dartmouth BASIC now declares 22/51 rows on `Beam` (up from 20/51); the
remaining ~29 numeric/`INPUT` rows still need the unscoped BEAM host-input
design (VM-060b) for the 5 `INPUT` rows, and their own per-row real-`erl`
probe for the rest (`FOR`/`LET`/arrays/`DATA`/`RND`/`GOSUB`) — nothing here
established that `neg`/`f64_pow` alone unblocks them.

