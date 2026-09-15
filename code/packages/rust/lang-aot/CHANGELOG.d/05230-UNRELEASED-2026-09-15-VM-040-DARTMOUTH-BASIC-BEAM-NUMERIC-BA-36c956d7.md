## Unreleased — 2026-09-15 — VM-040 Dartmouth BASIC BEAM numeric baseline (BEAM03)

The VM-040 BASIC BEAM pure-string slice below deliberately deferred BASIC's
numeric corpus, since `iir-to-beam` had zero `f64` lowering at all. `iir-to-beam`
0.10.0 adds it (design + real-`erl` verification: `code/specs/BEAM03-float-
lowering.md`), so this slice promotes the two BASIC "numeric baseline" rows —
the ones immediately preceding the 18-row pure-string family — to `Beam`:

- `10 PRINT 42\n20 END\n` → `"42"` — BA7-1b's paradigm case: an
  integer-spelled literal still rides the shared `f64` track through
  `__basic_print_real`, whose sign/zero/magnitude-bucket dispatch and
  `real_to_int_trunc` digit-extraction chain exercise the full new op set
  (`const`/`add`/`sub`/`mul`/`div`/`cmp_lt`/`cmp_eq`/`cmp_ge`/`int_to_real`/
  `real_to_int_trunc`), not just a bare float-load proof.
- `10 PRINT 6 ^ 2 + 6\n20 END\n` → `"42"` — a literal-integer-exponent `^`
  (`dartmouth-basic-iir-compiler`'s `literal_power_lowers_to_repeated_f64_mul`
  fast path: one `mul`, no `f64_pow`) plus a top-level `add`, on top of the
  same print path.

Both pass on real Erlang (`portable_text_stdout_dartmouth_basic_beam_numeric_baseline`,
new). `iir-to-beam` 0.10.0 also fixed a genuine discovered defect (VM-D034)
along the way: the existing i64 `div` lowering used `erlang:div/2`, which
traps on a float operand — f64 `div` now dispatches to `erlang:'/'/2`
instead. General f64 division by zero remains a documented, out-of-scope
platform gap (real Erlang floats cannot represent IEEE-754 Inf/NaN at all);
not reachable by either promoted row (both divisors are nonzero compile-time
constants). Dartmouth BASIC now declares 20/51 rows on `Beam`; the remaining
~26 numeric/`FOR`/`LET`/`RND` rows need `neg`(f64) and `f64_pow` (not yet
implemented) plus the still-unscoped BEAM host-input design (VM-060b) for
the 5 `INPUT` rows.

